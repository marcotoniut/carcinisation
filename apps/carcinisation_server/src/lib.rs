//! Dedicated game server using `bevy_replicon` + `bevy_renet2`.
#![allow(clippy::needless_pass_by_value)]

pub mod systems;

use bevy::prelude::*;
use bevy_renet2::prelude::ServerEvent;
use bevy_replicon::prelude::ServerTriggerExt;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::RenetChannelsExt;
use bevy_replicon_renet2::netcode::{
    NativeSocket, NetcodeServerTransport, ServerAuthentication, ServerSetupConfig,
};
use bevy_replicon_renet2::renet2::RenetServer;

use carcinisation_fps_core::{
    MosquitonAiConfig,
    map::{EntitySpawnData, EntitySpawnKind, Map, PlayerStartData},
};
use carcinisation_net::{CombatSet, MovementSet, TickSet};
use carcinisation_net::{
    FlameActive, NetAttackId, NetEnemyState, NetEnemyType, NetHealth, NetPlayer, NetProtocolPlugin,
    NetworkObjectId, PlayerId, PlayerIdAssigned, PlayerNetState, register_net_all,
};
use systems::combat::process_combat;
use systems::input::{apply_buffered_movement, receive_client_intent};
use systems::{
    BurnContactCooldowns, EnemyAiSet, EnemyAttackSet, FireCooldownMap, FlameActiveTracker,
    FlameCharCooldowns, NetEnemy, NextProjectileId, PlayerInputTracker, PlayerIntentBuffer,
    ProjectileSet, ServerEnemyAiConfig, ServerMosquitonSim, ServerMosquitonSimConfig,
    ServerQuickTurn, ServerTurnConfig, tick_burn_contact_damage, tick_despawn_timers,
    tick_enemy_attacks, tick_enemy_death_timers, tick_net_enemy_ai, tick_pending_projectiles,
    tick_player_lifecycle, tick_projectiles_server,
};

/// Component attached to `ConnectedClient` to track assigned `PlayerId`.
#[derive(Component, Debug, Clone, Copy)]
struct ClientPlayerId(PlayerId);

/// Server-side map resource for collision.
#[derive(Resource)]
pub struct ServerMap(pub Map);

/// Per-server player ID counter (starts at 1).
#[derive(Resource)]
struct NextPlayerId(u32);

impl Default for NextPlayerId {
    fn default() -> Self {
        Self(1)
    }
}

impl NextPlayerId {
    fn next(&mut self) -> PlayerId {
        let id = PlayerId(self.0);
        self.0 += 1;
        id
    }
}

/// Per-server spawn point rotator.
#[derive(Resource, Default)]
struct SpawnIndex(usize);

/// Map entity spawns, stored as a resource for the startup system.
#[derive(Resource, Default)]
pub struct MapEntities(pub Vec<EntitySpawnData>);

/// Player spawn points, stored as a resource for connection handling.
#[derive(Resource, Default)]
pub struct MapPlayerStarts(pub Vec<PlayerStartData>);

pub struct ServerPlugin {
    pub port: u16,
    pub map: Map,
    pub entities: Vec<EntitySpawnData>,
    pub player_starts: Vec<PlayerStartData>,
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "brp")]
        {
            if !app.is_plugin_added::<bevy::log::LogPlugin>() {
                app.add_plugins(bevy::log::LogPlugin::default());
            }
            app.add_plugins(bevy::remote::RemotePlugin::default());
            app.add_plugins(bevy::remote::http::RemoteHttpPlugin::default());
        }

        app.add_plugins(NetProtocolPlugin)
            .add_plugins(bevy_replicon::prelude::RepliconSharedPlugin {
                auth_method: bevy_replicon::prelude::AuthMethod::None,
            })
            .add_plugins(bevy_replicon::prelude::ServerPlugin::default())
            .add_plugins(bevy_replicon::prelude::ServerMessagePlugin);

        register_net_all(app);
        app.register_type::<MosquitonAiConfig>()
            .register_type::<ServerEnemyAiConfig>();

        app.add_plugins(bevy_replicon_renet2::RepliconRenetPlugins)
            .add_systems(
                PreUpdate,
                init_server_setup.run_if(not(resource_exists::<RenetServer>)),
            )
            .add_systems(
                PreUpdate,
                monitor_server_events.after(bevy_renet2::prelude::RenetReceive),
            )
            .init_resource::<PlayerInputTracker>()
            .init_resource::<PlayerIntentBuffer>()
            .init_resource::<ServerTurnConfig>()
            .init_resource::<FireCooldownMap>()
            .init_resource::<FlameActiveTracker>()
            .init_resource::<FlameCharCooldowns>()
            .init_resource::<BurnContactCooldowns>()
            .init_resource::<NextPlayerId>()
            .init_resource::<SpawnIndex>()
            .insert_resource(ServerMap(self.map.clone()))
            .insert_resource(MapEntities(self.entities.clone()))
            .insert_resource(MapPlayerStarts(normalized_player_starts(
                &self.map,
                &self.player_starts,
            )))
            .add_observer(receive_client_intent)
            .add_observer(handle_client_connect)
            .add_observer(handle_client_disconnect)
            .add_systems(Startup, spawn_map_enemies)
            .configure_sets(
                FixedUpdate,
                (
                    MovementSet,
                    EnemyAiSet,
                    EnemyAttackSet,
                    ProjectileSet,
                    CombatSet,
                    TickSet,
                )
                    .chain(),
            )
            .add_systems(FixedUpdate, apply_buffered_movement.in_set(MovementSet))
            .add_systems(FixedUpdate, tick_net_enemy_ai.in_set(EnemyAiSet))
            .add_systems(FixedUpdate, tick_enemy_attacks.in_set(EnemyAttackSet))
            .add_systems(
                FixedUpdate,
                tick_pending_projectiles
                    .in_set(EnemyAttackSet)
                    .after(tick_enemy_attacks),
            )
            .add_systems(FixedUpdate, tick_projectiles_server.in_set(ProjectileSet))
            .add_systems(FixedUpdate, process_combat.in_set(CombatSet))
            .add_systems(
                FixedUpdate,
                tick_burn_contact_damage
                    .in_set(CombatSet)
                    .after(process_combat),
            )
            .add_systems(
                FixedUpdate,
                tick_enemy_death_timers
                    .in_set(CombatSet)
                    .after(process_combat),
            )
            .add_systems(
                FixedUpdate,
                tick_player_lifecycle
                    .in_set(CombatSet)
                    .after(process_combat)
                    .after(tick_burn_contact_damage),
            )
            .add_systems(FixedUpdate, tick_despawn_timers.in_set(TickSet))
            .init_resource::<NextProjectileId>()
            .insert_resource(ServerPort(self.port));

        let wall_count = self.map.cells.iter().filter(|&&c| c > 0).count();
        info!(
            "ServerPlugin built: port={}, map={}x{} ({} walls)",
            self.port, self.map.width, self.map.height, wall_count
        );
    }
}

#[derive(Resource)]
pub struct ServerPort(pub u16);

fn init_server_setup(
    mut commands: Commands,
    server_port: Res<ServerPort>,
    channels: Res<RepliconChannels>,
) {
    use bevy_replicon_renet2::renet2::ConnectionConfig;
    use std::time::{SystemTime, UNIX_EPOCH};

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards");

    let server_configs = channels.server_configs();
    let client_configs = channels.client_configs();

    let connection_config = ConnectionConfig::from_channels(server_configs, client_configs);

    let server = RenetServer::new(connection_config);
    let public_addr = std::net::SocketAddr::new(
        std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
        server_port.0,
    );
    let socket = NativeSocket::new(std::net::UdpSocket::bind(public_addr).expect("bind"))
        .expect("create socket");
    let server_config = ServerSetupConfig {
        current_time,
        max_clients: 10,
        protocol_id: carcinisation_net::PROTOCOL_ID,
        authentication: ServerAuthentication::Unsecure,
        socket_addresses: vec![vec![public_addr]],
    };
    let transport = NetcodeServerTransport::new(server_config, socket).expect("create transport");

    commands.insert_resource(server);
    commands.insert_resource(transport);

    info!("Server listening on 0.0.0.0:{} (UDP)", server_port.0);
}

fn monitor_server_events(mut events: MessageReader<ServerEvent>) {
    for event in events.read() {
        trace!("SERVER EVENT: {:?}", event);
    }
}

fn handle_client_connect(
    trigger: On<Add, ConnectedClient>,
    mut commands: Commands,
    mut next_id: ResMut<NextPlayerId>,
    mut spawn_idx: ResMut<SpawnIndex>,
    player_starts: Res<MapPlayerStarts>,
) {
    let player_id = next_id.next();
    let spawn = player_starts.0[spawn_idx.0 % player_starts.0.len()];
    let position = Vec2::new(spawn.x, spawn.y);
    let angle = spawn.angle_deg.to_radians();
    spawn_idx.0 += 1;

    let client_entity = trigger.event().entity;

    info!(
        "Client entity {:?} connected, assigned PlayerId {:?}",
        client_entity, player_id
    );

    commands
        .entity(client_entity)
        .insert(ClientPlayerId(player_id));

    commands.server_trigger(ToClients {
        mode: SendMode::Direct(bevy_replicon::prelude::ClientId::Client(client_entity)),
        message: PlayerIdAssigned(player_id),
    });

    commands.spawn((
        NetPlayer {
            player_id,
            position,
            angle,
            current_attack: NetAttackId::None,
            state: PlayerNetState::Alive,
        },
        NetHealth {
            current: 100.0,
            max: 100.0,
        },
        ServerQuickTurn::default(),
        Replicated,
    ));
}

fn normalized_player_starts(map: &Map, map_starts: &[PlayerStartData]) -> Vec<PlayerStartData> {
    let mut starts = map_starts.to_vec();
    starts.extend(fallback_player_starts(map));
    starts
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]
fn fallback_player_starts(map: &Map) -> Vec<PlayerStartData> {
    let preferred = [
        PlayerStartData {
            x: 1.5,
            y: 1.5,
            angle_deg: 0.0,
        },
        PlayerStartData {
            x: 6.5,
            y: 1.5,
            angle_deg: 0.0,
        },
        PlayerStartData {
            x: 1.5,
            y: 6.5,
            angle_deg: 0.0,
        },
        PlayerStartData {
            x: 6.5,
            y: 6.5,
            angle_deg: 0.0,
        },
    ];

    let mut starts: Vec<PlayerStartData> = preferred
        .into_iter()
        .filter(|spawn| map.get(spawn.x.floor() as i32, spawn.y.floor() as i32) == 0)
        .collect();

    if starts.is_empty() {
        for y in 1..map.height.saturating_sub(1) {
            for x in 1..map.width.saturating_sub(1) {
                if map.get(x as i32, y as i32) == 0 {
                    starts.push(PlayerStartData {
                        x: x as f32 + 0.5,
                        y: y as f32 + 0.5,
                        angle_deg: 0.0,
                    });
                }
            }
        }
    }

    if starts.is_empty() {
        starts.push(PlayerStartData {
            x: 1.5,
            y: 1.5,
            angle_deg: 0.0,
        });
    }

    starts
}

#[allow(clippy::too_many_arguments)]
fn handle_client_disconnect(
    trigger: On<Remove, ConnectedClient>,
    mut commands: Commands,
    client_query: Query<&ClientPlayerId>,
    player_query: Query<(Entity, &NetPlayer)>,
    mut tracker: ResMut<PlayerInputTracker>,
    mut buffer: ResMut<PlayerIntentBuffer>,
    mut cooldowns: ResMut<FireCooldownMap>,
    mut flame_tracker: ResMut<FlameActiveTracker>,
    mut burn_cooldowns: ResMut<BurnContactCooldowns>,
    mut char_cooldowns: ResMut<FlameCharCooldowns>,
) {
    let client_entity = trigger.event().entity;

    let Some(client_pid) = client_query.get(client_entity).ok() else {
        warn!(
            "Disconnected client {:?} had no ClientPlayerId",
            client_entity
        );
        return;
    };
    let player_id = client_pid.0;

    for (entity, np) in player_query.iter() {
        if np.player_id == player_id {
            commands.entity(entity).despawn();
            break;
        }
    }

    // Emit FlameActive(false) if the player was flaming, so clients clear visuals.
    if flame_tracker.0.get(&player_id).copied().unwrap_or(false) {
        commands.server_trigger(ToClients {
            mode: SendMode::Broadcast,
            message: FlameActive {
                player_id,
                active: false,
            },
        });
    }

    tracker.remove_player(&player_id);
    buffer.remove_player(&player_id);
    cooldowns.remove_player(&player_id);
    flame_tracker.remove_player(&player_id);
    burn_cooldowns.remove_player(&player_id);
    char_cooldowns.remove_player(&player_id);

    info!(
        "Client {:?} disconnected, cleaned up PlayerId {:?}",
        client_entity, player_id
    );
}

/// Spawns enemies from the map's entity list on server startup.
#[allow(clippy::cast_precision_loss)]
fn spawn_map_enemies(mut commands: Commands, map_entities: Res<MapEntities>) {
    let mut next_id = 1_u32;
    let mut count = 0;

    for spawn in &map_entities.0 {
        let Some(health) = spawn.kind.health() else {
            continue; // Skip non-enemy entities (Pillars).
        };

        let object_id = NetworkObjectId(next_id);
        next_id += 1;

        let mut enemy_commands = commands.spawn((
            NetEnemy {
                object_id,
                position: bevy::math::Vec2::new(spawn.x, spawn.y),
                angle: 0.0,
                state: NetEnemyState::Idle,
                enemy_type: net_enemy_type_from_spawn(spawn),
            },
            NetHealth {
                current: health as f32,
                max: health as f32,
            },
            Replicated,
        ));
        if let Some(ai_config) = server_enemy_ai_config_from_spawn(spawn) {
            enemy_commands.insert(ai_config);
        }
        if let EntitySpawnKind::Mosquiton { speed, .. } = &spawn.kind {
            enemy_commands.insert((
                ServerMosquitonSim::default(),
                ServerMosquitonSimConfig::with_speed(*speed),
            ));
        }
        count += 1;
    }

    info!("Spawned {count} enemies from map entities");
}

fn net_enemy_type_from_spawn(spawn: &EntitySpawnData) -> NetEnemyType {
    match &spawn.kind {
        EntitySpawnKind::Pillar { .. }
        | EntitySpawnKind::Enemy { .. }
        | EntitySpawnKind::SpriteEnemy { .. } => NetEnemyType::Basic,
        EntitySpawnKind::Mosquiton { .. } => NetEnemyType::Mosquiton,
    }
}

fn server_enemy_ai_config_from_spawn(spawn: &EntitySpawnData) -> Option<ServerEnemyAiConfig> {
    match &spawn.kind {
        EntitySpawnKind::Mosquiton { speed, .. } => Some(ServerEnemyAiConfig::mosquiton(*speed)),
        EntitySpawnKind::Pillar { .. }
        | EntitySpawnKind::Enemy { .. }
        | EntitySpawnKind::SpriteEnemy { .. } => None,
    }
}
