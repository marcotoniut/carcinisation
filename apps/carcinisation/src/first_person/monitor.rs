//! Map monitor client plugin — passive spectator that receives replication
//! without spawning a player entity on the server.

use std::net::SocketAddr;
#[cfg(not(target_family = "wasm"))]
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use bevy_replicon::prelude::*;
#[cfg(not(target_family = "wasm"))]
use bevy_replicon_renet2::RenetChannelsExt;
#[cfg(not(target_family = "wasm"))]
use bevy_replicon_renet2::renet2::{ConnectionConfig, RenetClient};

use carapace::image::CxImage;
use carcinisation_fps::player_attack::PlayerAttackSprites;
use carcinisation_fps::plugin::{CameraRes, MosquitonSprites, SpideySprites, SpritePairs};
use carcinisation_map_view::config::MapViewConfig;
use carcinisation_map_view::overlay::{
    self, MapViewEntityMarker, MapViewOverlay, cell_to_pixel, flip_y,
};
use carcinisation_map_view::{MapViewMonitorMode, MapViewToggle};
use carcinisation_net::{
    ConnectMode, MonitorAck, NetBurning, NetEnemy, NetEnemyState, NetEnemyType, NetGroundFire,
    NetPlayer, NetProjectile, NetProtocolPlugin, PlayerNetState, register_net_all,
};

use super::ConnectionState;

/// Resource holding the server address for the monitor connection.
#[derive(Resource)]
struct MonitorConnectAddr(SocketAddr);

/// Plugin for a passive map monitor client.
///
/// Sets up networking with `ConnectMode::Monitor` in the renet2 handshake
/// `user_data`. No input/prediction systems are registered — the monitor
/// only receives replicated state and syncs the camera to follow players.
pub struct MapMonitorClientPlugin {
    pub connect_addr: SocketAddr,
}

impl Plugin for MapMonitorClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NetProtocolPlugin)
            .add_plugins(RepliconSharedPlugin {
                auth_method: AuthMethod::None,
            })
            .add_plugins(ClientPlugin)
            .add_plugins(ClientMessagePlugin);

        register_net_all(app);

        app.insert_resource(MapViewMonitorMode);

        app.add_plugins(bevy_replicon_renet2::RepliconRenetPlugins)
            .insert_resource(ConnectionState::Connecting {
                addr: self.connect_addr,
                start_time: std::time::Instant::now(),
            })
            .insert_resource(MonitorConnectAddr(self.connect_addr))
            .add_observer(handle_monitor_ack)
            .add_systems(Startup, init_monitor_transport)
            .add_systems(Update, monitor_connection_watchdog)
            .add_systems(
                Update,
                (
                    sync_monitor_camera,
                    build_net_entity_snapshot
                        .before(carcinisation_map_view::overlay::update_marker_overlay),
                )
                    .run_if(is_monitor_connected)
                    .run_if(|toggle: Res<MapViewToggle>| toggle.enabled),
            );
    }
}

fn is_monitor_connected(state: Res<ConnectionState>) -> bool {
    matches!(*state, ConnectionState::Connected)
}

/// Handle the server's acknowledgement that we're connected as a monitor.
fn handle_monitor_ack(_trigger: On<MonitorAck>, mut connection_state: ResMut<ConnectionState>) {
    if !matches!(*connection_state, ConnectionState::Connected) {
        *connection_state = ConnectionState::Connected;
        info!("Monitor connection established");
    }
}

/// Follow the first alive player's position and angle.
///
/// If no players are alive, keeps the last known position (or map centre
/// from initial `CameraRes`).
fn sync_monitor_camera(net_players: Query<&NetPlayer>, mut camera_res: ResMut<CameraRes>) {
    if let Some(player) = net_players
        .iter()
        .find(|p| p.state == PlayerNetState::Alive)
    {
        camera_res.0.position = player.position;
        camera_res.0.angle = player.angle;
    }
}

/// Detect connection timeout / disconnect.
fn monitor_connection_watchdog(mut connection_state: ResMut<ConnectionState>) {
    if let ConnectionState::Connecting { start_time, addr } = &*connection_state {
        let elapsed = std::time::Instant::now().duration_since(*start_time);
        if elapsed.as_secs() > 10 {
            let reason = format!("Connection to {addr} timed out after {elapsed:?}");
            warn!("{reason}");
            *connection_state = ConnectionState::Failed { reason };
        }
    }
}

/// Set up the renet2 client transport with `ConnectMode::Monitor` in `user_data`.
#[cfg(not(target_family = "wasm"))]
fn init_monitor_transport(
    mut commands: Commands,
    connect_addr: Res<MonitorConnectAddr>,
    channels: Res<RepliconChannels>,
    mut connection_state: ResMut<ConnectionState>,
) {
    use bevy_renet2::netcode::{ClientAuthentication, NativeSocket};

    // Reset start_time — plugin ctor captured Instant::now() before app init.
    if let ConnectionState::Connecting { start_time, .. } = &mut *connection_state {
        *start_time = std::time::Instant::now();
    }

    let client_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos() as u64;

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards");

    let server_configs = channels.server_configs();
    let client_configs = channels.client_configs();
    let connection_config = ConnectionConfig::from_channels(server_configs, client_configs);

    let local_addr =
        std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 0);
    let socket = NativeSocket::new(std::net::UdpSocket::bind(local_addr).expect("bind"))
        .expect("create socket");

    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: carcinisation_net::PROTOCOL_ID,
        socket_id: 0,
        server_addr: connect_addr.0,
        user_data: Some(ConnectMode::Monitor.to_user_data()),
    };

    let transport =
        bevy_renet2::netcode::NetcodeClientTransport::new(current_time, authentication, socket)
            .expect("create monitor transport");

    let client = RenetClient::new(connection_config, transport.is_reliable());

    commands.insert_resource(client);
    commands.insert_resource(transport);

    info!("Monitor connecting to {}...", connect_addr.0);
}

// ---------------------------------------------------------------------------
// Net overlay — renders replicated entities on the map view
// ---------------------------------------------------------------------------

/// Palette indices for monitor markers.
const PLAYER_MARKER_COLOR: u8 = 1;
const PROJECTILE_MARKER_COLOR: u8 = 4;

/// Cached static sprites for net overlay markers.
#[derive(Default)]
pub struct CachedNetSprites {
    player_marker: Option<CxImage>,
    projectile_circle: Option<CxImage>,
}

/// Build the per-frame overlay from replicated Net* components (monitor only).
///
/// Replaces `overlay::build_entity_snapshot` in monitor mode — clears markers
/// and fills from `NetPlayer`/`NetEnemy`/`NetProjectile`.
#[allow(clippy::too_many_arguments)]
fn build_net_entity_snapshot(
    net_players: Query<&NetPlayer>,
    net_enemies: Query<&NetEnemy>,
    net_projectiles: Query<&NetProjectile>,
    net_ground_fires: Query<&NetGroundFire>,
    net_burning: Query<(&NetEnemy, &NetBurning)>,
    sprite_pairs: Res<SpritePairs>,
    mosquiton_sprites: Res<MosquitonSprites>,
    spidey_sprites: Res<SpideySprites>,
    attack_sprites: Res<PlayerAttackSprites>,
    time: Res<Time>,
    camera: Res<CameraRes>,
    config: Res<MapViewConfig>,
    mut overlay: ResMut<MapViewOverlay>,
    mut cached: Local<CachedNetSprites>,
) {
    overlay.markers.clear();
    append_net_markers_inner(
        &net_players,
        &net_enemies,
        &net_projectiles,
        &net_ground_fires,
        &net_burning,
        &sprite_pairs,
        &mosquiton_sprites,
        &spidey_sprites,
        &attack_sprites,
        &time,
        camera.0.position,
        &config,
        &mut overlay,
        &mut cached,
    );
}

/// Append replicated Net* entity markers to the existing overlay (multiplayer client).
///
/// Does NOT clear markers — runs after `build_entity_snapshot` to add net
/// entities that don't exist as local FPS components in `RemoteClient` mode.
#[allow(clippy::too_many_arguments)]
pub fn append_net_markers(
    net_players: Query<&NetPlayer>,
    net_enemies: Query<&NetEnemy>,
    net_projectiles: Query<&NetProjectile>,
    net_ground_fires: Query<&NetGroundFire>,
    net_burning: Query<(&NetEnemy, &NetBurning)>,
    sprite_pairs: Res<SpritePairs>,
    mosquiton_sprites: Res<MosquitonSprites>,
    spidey_sprites: Res<SpideySprites>,
    attack_sprites: Res<PlayerAttackSprites>,
    time: Res<Time>,
    camera: Res<CameraRes>,
    config: Res<MapViewConfig>,
    mut overlay: ResMut<MapViewOverlay>,
    mut cached: Local<CachedNetSprites>,
) {
    append_net_markers_inner(
        &net_players,
        &net_enemies,
        &net_projectiles,
        &net_ground_fires,
        &net_burning,
        &sprite_pairs,
        &mosquiton_sprites,
        &spidey_sprites,
        &attack_sprites,
        &time,
        camera.0.position,
        &config,
        &mut overlay,
        &mut cached,
    );
}

/// Number of flame samples to simulate along the stream direction.
const FLAME_CHAIN_SAMPLES: usize = 4;
/// Spacing between simulated flame samples (in map cells).
const FLAME_SAMPLE_SPACING: f32 = 0.5;

#[allow(clippy::too_many_arguments)]
fn append_net_markers_inner(
    net_players: &Query<&NetPlayer>,
    net_enemies: &Query<&NetEnemy>,
    net_projectiles: &Query<&NetProjectile>,
    net_ground_fires: &Query<&NetGroundFire>,
    net_burning: &Query<(&NetEnemy, &NetBurning)>,
    sprite_pairs: &SpritePairs,
    mosquiton_sprites: &MosquitonSprites,
    spidey_sprites: &SpideySprites,
    attack_sprites: &PlayerAttackSprites,
    time: &Time,
    player_pos: Vec2,
    config: &MapViewConfig,
    overlay: &mut MapViewOverlay,
    cached: &mut CachedNetSprites,
) {
    let ts = config.tile_size;
    let ms = config.marker_size;
    let ems = overlay::enemy_marker_size(ms);
    let gh = overlay.grid_height;
    let elapsed = time.elapsed_secs();
    let flame_size = (ms / 2).max(3);
    let proj_size = (ms / 2).max(3);

    // Ensure static sprites are cached.
    let pms = (ms + 2).max(5);
    if cached.player_marker.is_none() {
        cached.player_marker = Some(overlay::player_marker_sprite(pms));
    }
    if cached.projectile_circle.is_none() {
        cached.projectile_circle = Some(overlay::circle_sprite(
            proj_size,
            PROJECTILE_MARKER_COLOR,
            1,
        ));
    }
    let player_base = cached.player_marker.as_ref().unwrap();
    let projectile_circle = cached.projectile_circle.as_ref().unwrap();

    // --- Flame layer (behind everything) ---

    let flame_frame = attack_sprites.flame_frame_loop(elapsed);
    let scaled_flame = overlay::scale(flame_frame, flame_size);

    // Player flamethrower streams — simulate a chain of samples.
    for player in net_players.iter() {
        if player.state != PlayerNetState::Alive || !player.flame_active {
            continue;
        }
        let cos_a = player.angle.cos();
        let sin_a = player.angle.sin();
        for i in 1..=FLAME_CHAIN_SAMPLES {
            let d = i as f32 * FLAME_SAMPLE_SPACING;
            let fx = player.position.x + cos_a * d;
            let fy = player.position.y + sin_a * d;
            overlay.markers.push(MapViewEntityMarker {
                centre_x: cell_to_pixel(fx, ts),
                centre_y: flip_y(fy, ts, gh),
                sprite: scaled_flame.clone(),
            });
        }
    }

    // Ground fires.
    for fire in net_ground_fires.iter() {
        overlay.markers.push(MapViewEntityMarker {
            centre_x: cell_to_pixel(fire.position.x, ts),
            centre_y: flip_y(fire.position.y, ts, gh),
            sprite: scaled_flame.clone(),
        });
    }

    // Burning enemies — overlay a flame on their position.
    for (enemy, burning) in net_burning.iter() {
        if burning.intensity <= 0.0 {
            continue;
        }
        overlay.markers.push(MapViewEntityMarker {
            centre_x: cell_to_pixel(enemy.position.x, ts),
            centre_y: flip_y(enemy.position.y, ts, gh),
            sprite: scaled_flame.clone(),
        });
    }

    // --- Enemy layer ---

    for enemy in net_enemies.iter() {
        if matches!(enemy.state, NetEnemyState::Dead { .. }) {
            continue;
        }
        let source: &CxImage = match enemy.enemy_type {
            NetEnemyType::Basic => {
                if let Some((alive, _)) = sprite_pairs.0.first() {
                    alive
                } else {
                    continue;
                }
            }
            NetEnemyType::Mosquiton => mosquiton_sprites.0.alive_sprite_at(elapsed),
            NetEnemyType::Spidey => spidey_sprites.0.alive_sprite_at(elapsed),
        };
        let scaled = overlay::scale(source, ems);
        let rotated = overlay::rotate(&scaled, overlay::angle_toward(enemy.position, player_pos));
        overlay.markers.push(MapViewEntityMarker {
            centre_x: cell_to_pixel(enemy.position.x, ts),
            centre_y: flip_y(enemy.position.y, ts, gh),
            sprite: rotated,
        });
    }

    // --- Projectile layer ---

    for proj in net_projectiles.iter() {
        overlay.markers.push(MapViewEntityMarker {
            centre_x: cell_to_pixel(proj.position.x, ts),
            centre_y: flip_y(proj.position.y, ts, gh),
            sprite: projectile_circle.clone(),
        });
    }

    // --- Player layer (on top of everything) ---

    for player in net_players.iter() {
        if player.state != PlayerNetState::Alive {
            continue;
        }
        let rotated = overlay::rotate(player_base, player.angle);
        overlay.markers.push(MapViewEntityMarker {
            centre_x: cell_to_pixel(player.position.x, ts),
            centre_y: flip_y(player.position.y, ts, gh),
            sprite: rotated,
        });
    }
}
