//! Dedicated game server using `bevy_replicon` + `bevy_renet2`.
#![allow(clippy::needless_pass_by_value)]

pub mod systems;

use bevy::prelude::*;
use bevy_replicon::prelude::ServerTriggerExt;
use bevy_replicon::prelude::*;
use bevy_replicon::shared::backend::connected_client::NetworkId;
use bevy_replicon_renet2::RenetChannelsExt;
use bevy_replicon_renet2::netcode::{
    NativeSocket, NetcodeServerTransport, ServerAuthentication, ServerSetupConfig,
};
use bevy_replicon_renet2::renet2::RenetServer;

pub use crate::systems::pickup::PickupSet;
use carcinisation_fps_core::{
    MosquitonAiConfig,
    map::{EntitySpawnData, EntitySpawnKind, Map, PlayerStartData},
    pickup::PickupRules,
};
use carcinisation_net::protocol::NetPickupKind;
use carcinisation_net::{
    AvatarPaletteVariant, ConnectMode, FlameActive, MonitorAck, NetAttackId, NetEnemyState,
    NetEnemyType, NetHealth, NetPlayer, NetProtocolPlugin, NetworkObjectId, PlayerId,
    PlayerIdAssigned, PlayerNetState, components::NetPickup, register_net_all,
};
use carcinisation_net::{CombatSet, MovementSet, TickSet};
use systems::admin::{poll_admin_socket, setup_admin_socket};
use systems::combat::process_combat;
use systems::diagnostics::{DiagnosticsState, tick_diagnostics_end, tick_diagnostics_start};
use systems::input::{apply_buffered_movement, receive_client_intent, send_input_acks};
use systems::reset::{MapResetRequested, handle_map_reset};
use systems::{
    BurnContactCooldowns, EnemyAiSet, EnemyAttackSet, FireCooldownMap, FlameActiveTracker,
    FlameCharCooldowns, GroundFireContactCooldowns, GroundFireCount, NextProjectileId,
    PlayerInputTracker, PlayerIntentBuffer, ProjectileSet, ServerEnemyAiConfig, ServerMosquitonSim,
    ServerMosquitonSimConfig, ServerQuickTurn, ServerSpideySim, ServerSpideySimConfig,
    tick_burn_contact_damage, tick_despawn_timers, tick_enemy_attacks, tick_enemy_death_timers,
    tick_ground_fire_damage, tick_net_enemy_ai, tick_pending_projectiles, tick_player_lifecycle,
    tick_projectiles_server, tick_spidey_attacks,
};

/// Component attached to `ConnectedClient` to track assigned `PlayerId`.
#[derive(Component, Debug, Clone, Copy)]
struct ClientPlayerId(PlayerId);

/// Marker component for monitor (spectator) clients. No player entity is spawned.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct ClientMonitor;

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
    const fn next(&mut self) -> PlayerId {
        let id = PlayerId(self.0);
        self.0 += 1;
        id
    }
}

/// Server-side avatar palette variant pool.
///
/// Maintains the six variants and tracks which are currently assigned.
/// When all six are in use, new players get a deterministic overflow
/// variant based on `PlayerId`. Variants are returned to the pool on
/// disconnect.
#[derive(Resource)]
struct AvatarPalettePool {
    /// Indexed by the variant enum's discriminant.
    free: [bool; AvatarPaletteVariant::COUNT],
}

impl Default for AvatarPalettePool {
    fn default() -> Self {
        Self {
            free: [true; AvatarPaletteVariant::COUNT],
        }
    }
}

impl AvatarPalettePool {
    /// Assign a variant, preferring a free slot.
    fn assign(&mut self, player_id: PlayerId) -> AvatarPaletteVariant {
        for (i, slot) in self.free.iter_mut().enumerate() {
            if *slot {
                *slot = false;
                return variant_from_index(i);
            }
        }
        variant_from_index(player_id.0 as usize % AvatarPaletteVariant::COUNT)
    }

    /// Release a previously assigned variant back to the pool.
    const fn release(&mut self, variant: AvatarPaletteVariant) {
        self.free[variant_index(variant)] = true;
    }
}

const fn variant_index(v: AvatarPaletteVariant) -> usize {
    use AvatarPaletteVariant::{Abc, Acb, Bac, Bca, Cab, Cba};
    match v {
        Abc => 0,
        Acb => 1,
        Bac => 2,
        Bca => 3,
        Cab => 4,
        Cba => 5,
    }
}

fn variant_from_index(i: usize) -> AvatarPaletteVariant {
    use AvatarPaletteVariant::{Abc, Acb, Bac, Bca, Cab, Cba};
    match i % AvatarPaletteVariant::COUNT {
        0 => Abc,
        1 => Acb,
        2 => Bac,
        3 => Bca,
        4 => Cab,
        5 => Cba,
        _ => unreachable!("index % COUNT must be 0..{}", AvatarPaletteVariant::COUNT),
    }
}

/// Per-server spawn point rotator.
#[derive(Resource, Default)]
pub struct SpawnIndex(pub usize);

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
    /// If set, the server binds a local admin socket at this path.
    pub admin_socket: Option<String>,
    /// Human-readable instance name (e.g. "deathmatch").
    pub instance_name: String,
    /// Map file path for status reporting.
    pub map_path: String,
}

#[allow(clippy::too_many_lines)]
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
            .init_resource::<PlayerInputTracker>()
            .init_resource::<PlayerIntentBuffer>()
            .init_resource::<FireCooldownMap>()
            .init_resource::<FlameActiveTracker>()
            .init_resource::<FlameCharCooldowns>()
            .init_resource::<BurnContactCooldowns>()
            .init_resource::<GroundFireContactCooldowns>()
            .init_resource::<GroundFireCount>()
            .insert_resource(systems::combat::load_burn_config())
            .insert_resource(carcinisation_fps_core::PlayerFlamethrowerConfig::load())
            .insert_resource(carcinisation_fps_core::FpsMovementConfig::load())
            .insert_resource(carcinisation_fps_core::FpsCombatConfig::load())
            .init_resource::<NextPlayerId>()
            .init_resource::<AvatarPalettePool>()
            .init_resource::<SpawnIndex>()
            .insert_resource(PickupRules::load())
            .init_resource::<systems::pickup::PickupEventBuffer>()
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
                    PickupSet,
                    TickSet,
                )
                    .chain(),
            )
            .add_systems(FixedUpdate, apply_buffered_movement.in_set(MovementSet))
            .add_systems(
                FixedUpdate,
                send_input_acks
                    .in_set(MovementSet)
                    .after(apply_buffered_movement),
            )
            .add_systems(FixedUpdate, tick_net_enemy_ai.in_set(EnemyAiSet))
            .add_systems(FixedUpdate, tick_enemy_attacks.in_set(EnemyAttackSet))
            .add_systems(FixedUpdate, tick_spidey_attacks.in_set(EnemyAttackSet))
            .add_systems(
                FixedUpdate,
                tick_pending_projectiles
                    .in_set(EnemyAttackSet)
                    .after(tick_enemy_attacks)
                    .after(tick_spidey_attacks),
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
                tick_ground_fire_damage
                    .in_set(CombatSet)
                    .after(process_combat)
                    .after(tick_burn_contact_damage),
            )
            .add_systems(
                FixedUpdate,
                systems::combat::tick_enemy_burning
                    .in_set(CombatSet)
                    .after(process_combat)
                    .after(tick_ground_fire_damage),
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
            .add_systems(FixedUpdate, systems::pickup_system.in_set(PickupSet))
            .add_systems(
                FixedUpdate,
                systems::pickup::flush_pickup_events.after(PickupSet),
            )
            .add_systems(FixedUpdate, tick_despawn_timers.in_set(TickSet))
            .add_systems(
                FixedUpdate,
                tick_diagnostics_start
                    .in_set(MovementSet)
                    .before(apply_buffered_movement),
            )
            .add_systems(
                FixedUpdate,
                tick_diagnostics_end
                    .in_set(TickSet)
                    .after(tick_despawn_timers),
            )
            .init_resource::<DiagnosticsState>()
            .init_resource::<NextProjectileId>()
            .init_resource::<MapResetRequested>()
            .insert_resource(ServerPort(self.port))
            .add_systems(
                FixedUpdate,
                handle_map_reset
                    .in_set(MovementSet)
                    .before(tick_diagnostics_start),
            );

        // Admin socket (optional — skipped in tests or when no path is given).
        if let Some(ref socket_path) = self.admin_socket {
            let admin_state = setup_admin_socket(
                socket_path,
                self.instance_name.clone(),
                self.map_path.clone(),
            );
            app.insert_resource(admin_state).add_systems(
                FixedUpdate,
                poll_admin_socket
                    .in_set(TickSet)
                    .after(tick_diagnostics_end),
            );
        }

        let wall_count = self.map.cells.iter().filter(|&&c| c > 0).count();
        let spawn_count = self.player_starts.len();
        let entity_count = self.entities.len();
        info!(
            "ServerPlugin built: port={} map={}x{} walls={} spawns={} entities={} tick_hz=30",
            self.port, self.map.width, self.map.height, wall_count, spawn_count, entity_count
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
        // Includes headroom for monitor/spectator connections beyond the player slots.
        max_clients: 16,
        protocol_id: carcinisation_net::PROTOCOL_ID,
        authentication: ServerAuthentication::Unsecure,
        socket_addresses: vec![vec![public_addr]],
    };
    let transport = NetcodeServerTransport::new(server_config, socket).expect("create transport");

    commands.insert_resource(server);
    commands.insert_resource(transport);

    info!("Server listening on 0.0.0.0:{} (UDP)", server_port.0);
}

#[allow(clippy::too_many_arguments)]
fn handle_client_connect(
    trigger: On<Add, ConnectedClient>,
    mut commands: Commands,
    mut next_id: ResMut<NextPlayerId>,
    mut spawn_idx: ResMut<SpawnIndex>,
    player_starts: Res<MapPlayerStarts>,
    mut palette_pool: ResMut<AvatarPalettePool>,
    transport: Res<NetcodeServerTransport>,
    network_ids: Query<&NetworkId>,
) {
    let client_entity = trigger.event().entity;

    // Determine connect mode from user_data embedded in the renet2 handshake.
    let connect_mode = if let Ok(nid) = network_ids.get(client_entity) {
        transport
            .user_data(nid.get())
            .map(|ud| ConnectMode::from_user_data(&ud))
            .unwrap_or(ConnectMode::Player)
    } else {
        ConnectMode::Player
    };

    let client_id = bevy_replicon::prelude::ClientId::Client(client_entity);

    match connect_mode {
        ConnectMode::Monitor => {
            info!(
                "Monitor client {:?} connected (spectator only)",
                client_entity
            );
            commands.entity(client_entity).insert(ClientMonitor);
            commands.server_trigger(ToClients {
                mode: SendMode::Direct(client_id),
                message: MonitorAck,
            });
        }
        ConnectMode::Player => {
            let player_id = next_id.next();
            let spawn = player_starts.0[spawn_idx.0 % player_starts.0.len()];
            let position = Vec2::new(spawn.x, spawn.y);
            let angle = spawn.angle_deg.to_radians();
            spawn_idx.0 += 1;

            let avatar_variant = palette_pool.assign(player_id);

            info!(
                "Client entity {:?} connected, assigned PlayerId {:?} variant {:?}",
                client_entity, player_id, avatar_variant
            );

            commands
                .entity(client_entity)
                .insert(ClientPlayerId(player_id));

            commands.server_trigger(ToClients {
                mode: SendMode::Direct(client_id),
                message: PlayerIdAssigned(player_id),
            });

            commands.spawn((
                NetPlayer {
                    player_id,
                    position,
                    angle,
                    current_attack: NetAttackId::None,
                    state: PlayerNetState::Alive,
                    flame_active: false,
                    avatar_palette_variant: Some(avatar_variant),
                },
                NetHealth {
                    current: 100.0,
                    max: 100.0,
                },
                ServerQuickTurn::default(),
                Replicated,
            ));
        }
    }
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
    monitor_query: Query<&ClientMonitor>,
    player_query: Query<(Entity, &NetPlayer)>,
    mut tracker: ResMut<PlayerInputTracker>,
    mut buffer: ResMut<PlayerIntentBuffer>,
    mut cooldowns: ResMut<FireCooldownMap>,
    mut flame_tracker: ResMut<FlameActiveTracker>,
    mut burn_cooldowns: ResMut<BurnContactCooldowns>,
    mut char_cooldowns: ResMut<FlameCharCooldowns>,
    mut gf_cooldowns: ResMut<GroundFireContactCooldowns>,
    mut palette_pool: ResMut<AvatarPalettePool>,
) {
    let client_entity = trigger.event().entity;

    // Monitor clients have no player state to clean up.
    if monitor_query.get(client_entity).is_ok() {
        info!("Monitor client {:?} disconnected", client_entity);
        return;
    }

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
            if let Some(variant) = np.avatar_palette_variant {
                palette_pool.release(variant);
            }
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
    gf_cooldowns.remove_player(&player_id);

    info!(
        "Client {:?} disconnected, cleaned up PlayerId {:?}",
        client_entity, player_id
    );
}

/// Spawns enemies from the map's entity list on server startup.
fn spawn_map_enemies(mut commands: Commands, map_entities: Res<MapEntities>) {
    let count = spawn_map_enemies_inner(&mut commands, &map_entities.0);
    info!("Spawned {count} enemies from map entities");
}

/// Shared enemy spawning logic used by both startup and map reset.
#[allow(clippy::cast_precision_loss)]
pub fn spawn_map_enemies_inner(commands: &mut Commands, entities: &[EntitySpawnData]) -> u32 {
    use carcinisation_fps_core::pickup::PickupKind;

    let mut next_id = 1_u32;
    let mut count = 0u32;

    for spawn in entities {
        let object_id = NetworkObjectId(next_id);
        next_id += 1;

        if let Some(health) = spawn.kind.health() {
            // Spawn enemy
            let mut enemy_commands = commands.spawn((
                systems::NetEnemy {
                    object_id,
                    position: bevy::math::Vec2::new(spawn.x, spawn.y),
                    angle: 0.0,
                    state: NetEnemyState::Idle,
                    enemy_type: net_enemy_type_from_spawn(spawn),
                    visual_height: 0.0,
                    visual_phase: 0.0,
                },
                NetHealth {
                    current: health as f32,
                    max: health as f32,
                },
                carcinisation_net::NetBurning::default(),
                crate::systems::combat::ServerBurnState::default(),
                Replicated,
            ));
            if let Some(ai_config) = server_enemy_ai_config_from_spawn(spawn) {
                enemy_commands.insert(ai_config);
            }
            if let EntitySpawnKind::Mosquiton { speed, .. } = &spawn.kind {
                let seed =
                    carcinisation_fps_core::corpse_seed(bevy::math::Vec2::new(spawn.x, spawn.y));
                enemy_commands.insert((
                    ServerMosquitonSim {
                        seed,
                        ..Default::default()
                    },
                    ServerMosquitonSimConfig::with_speed(*speed),
                ));
            }
            if let EntitySpawnKind::Spidey { speed, .. } = &spawn.kind {
                let seed =
                    carcinisation_fps_core::corpse_seed(bevy::math::Vec2::new(spawn.x, spawn.y));
                let combat = carcinisation_fps_core::FpsCombatConfig::load();
                enemy_commands.insert((
                    ServerSpideySim {
                        seed,
                        ..Default::default()
                    },
                    ServerSpideySimConfig::from_combat_config(&combat, *speed),
                ));
            }
            count += 1;
        } else if let EntitySpawnKind::Pickup { kind, respawnable } = &spawn.kind {
            // Spawn pickup as a replicated entity so clients see available state.
            commands.spawn((
                NetPickup {
                    object_id,
                    position: bevy::math::Vec2::new(spawn.x, spawn.y),
                    kind: match kind {
                        PickupKind::Health => NetPickupKind::Health,
                        PickupKind::Ammo => NetPickupKind::Ammo,
                        PickupKind::Weapon => NetPickupKind::Weapon,
                    },
                    available: true,
                    respawn_remaining: None,
                    respawnable: *respawnable,
                },
                Replicated,
            ));
            count += 1;
        } else {
            // Skip non-enemy, non-pickup entities (e.g., Pillars)
        }
    }

    count
}

#[must_use]
pub const fn net_enemy_type_from_spawn(spawn: &EntitySpawnData) -> NetEnemyType {
    match &spawn.kind {
        EntitySpawnKind::Pillar { .. }
        | EntitySpawnKind::Enemy { .. }
        | EntitySpawnKind::SpriteEnemy { .. }
        | EntitySpawnKind::Pickup { .. } => NetEnemyType::Basic,
        EntitySpawnKind::Mosquiton { .. } => NetEnemyType::Mosquiton,
        EntitySpawnKind::Spidey { .. } => NetEnemyType::Spidey,
    }
}

#[must_use]
pub fn server_enemy_ai_config_from_spawn(spawn: &EntitySpawnData) -> Option<ServerEnemyAiConfig> {
    match &spawn.kind {
        EntitySpawnKind::Mosquiton { speed, .. } => Some(ServerEnemyAiConfig::mosquiton(*speed)),
        // Spidey uses its own sim (ServerSpideySim), not the shared AI dispatcher.
        EntitySpawnKind::Pillar { .. }
        | EntitySpawnKind::Enemy { .. }
        | EntitySpawnKind::SpriteEnemy { .. }
        | EntitySpawnKind::Spidey { .. }
        | EntitySpawnKind::Pickup { .. } => None,
    }
}

#[cfg(test)]
mod pool_tests {
    use super::*;

    fn unique_variants(count: usize) -> Vec<AvatarPaletteVariant> {
        let mut pool = AvatarPalettePool::default();
        (0..count)
            .map(|i| pool.assign(PlayerId(u32::try_from(i).unwrap() + 1)))
            .collect()
    }

    #[test]
    fn first_six_assignments_are_all_unique() {
        let variants = unique_variants(AvatarPaletteVariant::COUNT);
        let mut dedup = variants.clone();
        dedup.sort_by_key(|v| variant_index(*v));
        dedup.dedup_by_key(|v| variant_index(*v));
        assert_eq!(
            dedup.len(),
            AvatarPaletteVariant::COUNT,
            "first {} players must get unique variants",
            AvatarPaletteVariant::COUNT,
        );
    }

    #[test]
    fn overflow_assigns_deterministic_variant_by_player_id() {
        let variants = unique_variants(AvatarPaletteVariant::COUNT + 1);
        // PlayerId(7) overflows: 7 % COUNT
        let expected = variant_from_index(7 % AvatarPaletteVariant::COUNT);
        assert_eq!(variants[AvatarPaletteVariant::COUNT], expected);
    }

    #[test]
    fn release_makes_variant_available_again() {
        let mut pool = AvatarPalettePool::default();
        let v1 = pool.assign(PlayerId(1));
        let _ = pool.assign(PlayerId(2));
        let _ = pool.assign(PlayerId(3));
        pool.release(v1);
        let v4 = pool.assign(PlayerId(4));
        assert_eq!(v4, v1, "released variant should be reassigned next");
    }

    #[test]
    fn pool_default_all_free() {
        let pool = AvatarPalettePool::default();
        assert!(pool.free.iter().all(|&f| f));
    }

    #[test]
    fn overflow_is_deterministic() {
        let a = unique_variants(20);
        let b = unique_variants(20);
        assert_eq!(a, b, "overflow assignment must be deterministic");
    }
}
