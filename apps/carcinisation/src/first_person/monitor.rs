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
use carcinisation_fps::plugin::{CameraRes, MapRes, MosquitonSprites, SpideySprites, SpritePairs};
use carcinisation_fps_core::{Map, PlayerFlamethrowerConfig, cast_ray};
use carcinisation_map_view::config::MapViewConfig;
use carcinisation_map_view::overlay::{
    self, MapViewEntityMarker, MapViewOverlay, cell_to_pixel, flip_y,
};
use carcinisation_map_view::{MapViewMonitorMode, MapViewToggle};
use carcinisation_net::{
    ConnectMode, MonitorAck, NetBurning, NetEnemy, NetEnemyState, NetEnemyType, NetGroundFire,
    NetPickupKind, NetPlayer, NetProjectile, NetProtocolPlugin, PlayerId, PlayerNetState,
    components::NetPickup, register_net_all,
};

use super::ConnectionState;

/// Resource holding the server address for the monitor connection.
#[derive(Resource)]
struct MonitorConnectAddr(SocketAddr);

/// Monitor camera mode.
#[derive(Resource, Debug, Default)]
pub enum MonitorCameraMode {
    /// Free-roam: arrow keys pan across the map. Default mode.
    #[default]
    Free,
    /// Follow a specific player by their stable `PlayerId`.
    Follow(PlayerId),
}

/// Pan speed in map cells per second for free-cam mode.
const FREE_CAM_SPEED: f32 = 4.0;

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
        app.init_resource::<MonitorCameraMode>();

        app.add_plugins(bevy_replicon_renet2::RepliconRenetPlugins)
            .insert_resource(ConnectionState::Connecting {
                addr: self.connect_addr,
                start_time: std::time::Instant::now(),
            })
            .insert_resource(MonitorConnectAddr(self.connect_addr))
            .add_observer(handle_monitor_ack)
            .add_systems(Startup, init_monitor_transport)
            .add_systems(PostStartup, center_camera_on_map)
            .add_systems(Update, monitor_connection_watchdog)
            .add_systems(
                Update,
                (
                    monitor_camera_input,
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

/// Centre the camera on the map at startup.
fn center_camera_on_map(map_res: Res<MapRes>, mut camera_res: ResMut<CameraRes>) {
    camera_res.0.position = Vec2::new(map_res.0.width as f32 / 2.0, map_res.0.height as f32 / 2.0);
    camera_res.0.angle = 0.0;
}

/// Handle monitor camera input: free-cam panning, follow-player cycling.
///
/// Controls:
/// - Arrow keys / WASD: pan in free-cam mode
/// - Tab: cycle to next alive player (enters follow mode)
/// - Escape: return to free-cam mode
#[allow(clippy::needless_pass_by_value)]
fn monitor_camera_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    net_players: Query<&NetPlayer>,
    mut camera_res: ResMut<CameraRes>,
    mut mode: ResMut<MonitorCameraMode>,
) {
    // Tab: cycle through alive players (sorted by PlayerId for stability).
    if keys.just_pressed(KeyCode::Tab) {
        let mut alive: Vec<&NetPlayer> = net_players
            .iter()
            .filter(|p| p.state == PlayerNetState::Alive)
            .collect();
        alive.sort_by_key(|p| p.player_id.0);

        if !alive.is_empty() {
            let next_id = match *mode {
                MonitorCameraMode::Free => alive[0].player_id,
                MonitorCameraMode::Follow(current_id) => {
                    // Find current in sorted list, advance to next (wrapping).
                    let pos = alive
                        .iter()
                        .position(|p| p.player_id == current_id)
                        .map_or(0, |i| (i + 1) % alive.len());
                    alive[pos].player_id
                }
            };
            *mode = MonitorCameraMode::Follow(next_id);
            info!("Monitor: following player {:?}", next_id);
        }
    }

    // Escape: return to free-cam.
    if keys.just_pressed(KeyCode::Escape) && !matches!(*mode, MonitorCameraMode::Free) {
        *mode = MonitorCameraMode::Free;
        info!("Monitor: free-cam");
    }

    match *mode {
        MonitorCameraMode::Free => {
            let dt = time.delta_secs();
            let speed = FREE_CAM_SPEED * dt;
            let mut delta = Vec2::ZERO;
            if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
                delta.y += speed;
            }
            if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
                delta.y -= speed;
            }
            if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
                delta.x += speed;
            }
            if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
                delta.x -= speed;
            }
            camera_res.0.position += delta;
        }
        MonitorCameraMode::Follow(target_id) => {
            // Find the followed player by stable PlayerId.
            if let Some(player) = net_players
                .iter()
                .find(|p| p.player_id == target_id && p.state == PlayerNetState::Alive)
            {
                camera_res.0.position = player.position;
                camera_res.0.angle = player.angle;
            } else {
                // Followed player died or disconnected — return to free-cam.
                *mode = MonitorCameraMode::Free;
            }
        }
    }
}

/// Detect connection timeout and transport disconnect.
fn monitor_connection_watchdog(
    mut connection_state: ResMut<ConnectionState>,
    client: Option<Res<RenetClient>>,
) {
    match &*connection_state {
        ConnectionState::Connecting { start_time, addr } => {
            let elapsed = std::time::Instant::now().duration_since(*start_time);
            if elapsed.as_secs() > 10 {
                let reason = format!("Connection to {addr} timed out after {elapsed:?}");
                warn!("{reason}");
                *connection_state = ConnectionState::Failed { reason };
            }
        }
        ConnectionState::Connected => {
            if let Some(client) = &client
                && client.is_disconnected()
            {
                let reason = client
                    .disconnect_reason()
                    .map_or_else(|| "unknown".into(), |r| format!("{r:?}"));
                warn!("Monitor: disconnected ({reason})");
                *connection_state = ConnectionState::Disconnected { reason };
            }
        }
        _ => {}
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

/// Palette index for projectile markers on the monitor overlay.
const PROJECTILE_MARKER_COLOR: u8 = 4;
/// Palette index for health pickup markers.
const PICKUP_HEALTH_COLOR: u8 = 3;
/// Palette index for ammo/weapon pickup markers.
const PICKUP_ITEM_COLOR: u8 = 2;

/// Cached static sprites and derived constants for net overlay markers.
#[derive(Default)]
pub struct CachedNetSprites {
    player_marker: Option<CxImage>,
    projectile_circle: Option<CxImage>,
    pickup_health: Option<CxImage>,
    pickup_item: Option<CxImage>,
}

#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct NetMarkerSources<'w, 's> {
    net_players: Query<'w, 's, &'static NetPlayer>,
    net_enemies: Query<'w, 's, &'static NetEnemy>,
    net_projectiles: Query<'w, 's, &'static NetProjectile>,
    net_ground_fires: Query<'w, 's, &'static NetGroundFire>,
    net_burning: Query<'w, 's, (&'static NetEnemy, &'static NetBurning)>,
    net_pickups: Query<'w, 's, &'static NetPickup>,
    map_res: Res<'w, MapRes>,
    sprite_pairs: Res<'w, SpritePairs>,
    mosquiton_sprites: Res<'w, MosquitonSprites>,
    spidey_sprites: Res<'w, SpideySprites>,
    attack_sprites: Res<'w, PlayerAttackSprites>,
    flame_config: Res<'w, PlayerFlamethrowerConfig>,
    time: Res<'w, Time>,
    camera: Res<'w, CameraRes>,
    config: Res<'w, MapViewConfig>,
}

/// Build the per-frame overlay from replicated Net* components (monitor only).
///
/// Replaces `overlay::build_entity_snapshot` in monitor mode — clears markers
/// and fills from `NetPlayer`/`NetEnemy`/`NetProjectile`.
#[allow(clippy::too_many_arguments)]
fn build_net_entity_snapshot(
    sources: NetMarkerSources,
    mut overlay: ResMut<MapViewOverlay>,
    mut cached: Local<CachedNetSprites>,
) {
    overlay.markers.clear();
    append_net_markers_inner(
        &sources.net_players,
        &sources.net_enemies,
        &sources.net_projectiles,
        &sources.net_ground_fires,
        &sources.net_burning,
        &sources.net_pickups,
        &sources.map_res.0,
        &sources.sprite_pairs,
        &sources.mosquiton_sprites,
        &sources.spidey_sprites,
        &sources.attack_sprites,
        &sources.flame_config,
        &sources.time,
        sources.camera.0.position,
        &sources.config,
        &mut overlay,
        &mut cached,
    );
}

/// Append replicated Net* entity markers to the existing overlay (multiplayer client).
///
/// Does NOT clear markers — runs after `build_entity_snapshot` to add net
/// entities that don't exist as local FPS components in `RemoteClient` mode.
#[allow(clippy::too_many_arguments)]
pub(super) fn append_net_markers(
    sources: NetMarkerSources,
    mut overlay: ResMut<MapViewOverlay>,
    mut cached: Local<CachedNetSprites>,
) {
    append_net_markers_inner(
        &sources.net_players,
        &sources.net_enemies,
        &sources.net_projectiles,
        &sources.net_ground_fires,
        &sources.net_burning,
        &sources.net_pickups,
        &sources.map_res.0,
        &sources.sprite_pairs,
        &sources.mosquiton_sprites,
        &sources.spidey_sprites,
        &sources.attack_sprites,
        &sources.flame_config,
        &sources.time,
        sources.camera.0.position,
        &sources.config,
        &mut overlay,
        &mut cached,
    );
}

/// Spacing between simulated flame samples (in map cells).
const FLAME_SAMPLE_SPACING: f32 = 0.5;

fn assert_valid_flame_range(range: f32) {
    assert!(
        range.is_finite() && range >= 0.0,
        "invalid PlayerFlamethrowerConfig.range for monitor flame overlay: {range}"
    );
}

fn flame_chain_sample_count(range: f32) -> usize {
    assert_valid_flame_range(range);
    (range / FLAME_SAMPLE_SPACING).ceil() as usize
}

fn flame_chain_max_distance(origin: Vec2, direction: Vec2, range: f32, map: &Map) -> f32 {
    assert_valid_flame_range(range);
    let wall_hit = cast_ray(map, origin, direction);
    if wall_hit.wall_id > 0 {
        range.min(wall_hit.distance)
    } else {
        range
    }
}

#[allow(clippy::too_many_arguments)]
fn append_net_markers_inner(
    net_players: &Query<&NetPlayer>,
    net_enemies: &Query<&NetEnemy>,
    net_projectiles: &Query<&NetProjectile>,
    net_ground_fires: &Query<&NetGroundFire>,
    net_burning: &Query<(&NetEnemy, &NetBurning)>,
    net_pickups: &Query<&NetPickup>,
    map: &Map,
    sprite_pairs: &SpritePairs,
    mosquiton_sprites: &MosquitonSprites,
    spidey_sprites: &SpideySprites,
    attack_sprites: &PlayerAttackSprites,
    flame_config: &PlayerFlamethrowerConfig,
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
    let pickup_size = ms.max(3);
    if cached.pickup_health.is_none() {
        cached.pickup_health = Some(overlay::circle_sprite(pickup_size, PICKUP_HEALTH_COLOR, 1));
    }
    if cached.pickup_item.is_none() {
        cached.pickup_item = Some(overlay::circle_sprite(pickup_size, PICKUP_ITEM_COLOR, 1));
    }
    let player_base = cached.player_marker.as_ref().unwrap();
    let projectile_circle = cached.projectile_circle.as_ref().unwrap();
    let pickup_health = cached.pickup_health.as_ref().unwrap();
    let pickup_item = cached.pickup_item.as_ref().unwrap();

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
        let direction = Vec2::new(cos_a, sin_a);
        let max_distance =
            flame_chain_max_distance(player.position, direction, flame_config.range, map);
        let flame_samples = flame_chain_sample_count(max_distance);
        for i in 1..=flame_samples {
            let d = (i as f32 * FLAME_SAMPLE_SPACING).min(max_distance);
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

    // --- Pickup layer ---

    for pickup in net_pickups.iter() {
        if !pickup.available {
            continue;
        }
        let sprite = match pickup.kind {
            NetPickupKind::Health => pickup_health,
            NetPickupKind::Ammo | NetPickupKind::Weapon => pickup_item,
        };
        overlay.markers.push(MapViewEntityMarker {
            centre_x: cell_to_pixel(pickup.position.x, ts),
            centre_y: flip_y(pickup.position.y, ts, gh),
            sprite: sprite.clone(),
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

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn flame_chain_sample_count_tracks_config_range() {
        assert_eq!(flame_chain_sample_count(0.0), 0);
        assert_eq!(flame_chain_sample_count(0.5), 1);
        assert_eq!(flame_chain_sample_count(5.0), 10);
        assert_eq!(flame_chain_sample_count(5.1), 11);
    }

    #[test]
    #[should_panic(expected = "invalid PlayerFlamethrowerConfig.range")]
    fn flame_chain_sample_count_rejects_invalid_range() {
        let _ = flame_chain_sample_count(f32::NAN);
    }

    #[test]
    #[should_panic(expected = "invalid PlayerFlamethrowerConfig.range")]
    fn flame_chain_max_distance_rejects_invalid_range_before_clipping() {
        let map = carcinisation_fps_core::test_map();
        let _ = flame_chain_max_distance(Vec2::new(1.5, 2.5), Vec2::X, f32::NAN, &map);
    }

    #[test]
    fn flame_chain_max_distance_clips_to_nearest_wall() {
        let map = carcinisation_fps_core::test_map();
        let origin = Vec2::new(1.5, 2.5);
        let direction = Vec2::new(1.0, 0.0);

        assert_eq!(flame_chain_max_distance(origin, direction, 5.0, &map), 1.5);
    }

    #[test]
    fn flame_chain_max_distance_uses_range_when_wall_is_farther() {
        let map = carcinisation_fps_core::test_map();
        let origin = Vec2::new(1.5, 1.5);
        let direction = Vec2::new(1.0, 0.0);

        assert_eq!(flame_chain_max_distance(origin, direction, 5.0, &map), 5.0);
    }
}
