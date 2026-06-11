#![allow(dead_code, clippy::cast_possible_truncation)]

pub mod combat;
pub mod reset;

use std::net::SocketAddr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use bevy_renet2::netcode::{NativeSocket, NetcodeClientTransport};
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::RenetChannelsExt;
use carcinisation_fps_core::map::test_map;
use carcinisation_server::ServerPlugin;

/// Create a `ServerPlugin` for tests using the hardcoded `test_map` (no entities).
pub fn test_server_plugin(port: u16) -> ServerPlugin {
    ServerPlugin {
        port,
        map: test_map(),
        entities: Vec::new(),
        player_starts: Vec::new(),
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    }
}

/// Build a server app for input/movement tests.
pub fn build_fixed_tick_server(port: u16) -> App {
    build_server_app(test_server_plugin(port))
}

/// Build a client app for input/movement tests.
pub fn build_fixed_tick_client(addr: SocketAddr) -> App {
    build_client_app(
        carcinisation_net::NetProtocolPlugin,
        carcinisation_net::register_net_all,
        addr,
    )
}

/// Tick server + client. A 2 ms sleep lets real time accumulate so that:
/// 1. Bevy's `FixedUpdate` fires once per ~16–17 ticks (30 Hz ≈ 33 ms period).
/// 2. UDP packets have wall-clock time to arrive before the server's next
///    `FixedUpdate` processes them.
///
/// NOTE: Do NOT replace this with `TimeUpdateStrategy::ManualDuration` on the
/// server. That makes `FixedUpdate` fire every `server.update()`, but the client
/// sends input in its `Update` phase (after `server.update()`), so packets arrive
/// 1–2 ticks too late and inputs are silently dropped.
pub fn tick_with_sleep(server: &mut App, client: &mut App) {
    std::thread::sleep(Duration::from_millis(2));
    server.update();
    client.update();
}

/// Tick server + two clients. See `tick_with_sleep` for why the sleep is needed.
pub fn tick3_with_sleep(server: &mut App, c1: &mut App, c2: &mut App) {
    std::thread::sleep(Duration::from_millis(2));
    server.update();
    c1.update();
    c2.update();
}
use bevy_replicon_renet2::renet2::{ConnectionConfig, RenetClient, RenetServer};

/// Per-process port base offset to avoid collisions under parallel test execution
/// (cargo-nextest). Each process gets a unique 20k-port range based on its PID.
static PORT_BASE: OnceLock<u16> = OnceLock::new();

/// Counter offset from `PORT_BASE` for sequential port allocation.
static NEXT_TEST_PORT: AtomicU16 = AtomicU16::new(0);

/// Reserve the next available test port, verifying the port is bindable.
/// Retries up to 64 times on bind failure (port in `TIME_WAIT` or already in use).
/// Port range per process: [25000 + (PID % 20000), 65535).
#[allow(clippy::single_match)]
pub fn reserve_port() -> u16 {
    let base =
        *PORT_BASE.get_or_init(|| 25000u16.wrapping_add((std::process::id() as u16) % 20000));
    for _ in 0..64 {
        let port = base.wrapping_add(NEXT_TEST_PORT.fetch_add(1, Ordering::SeqCst));
        let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), port);
        if let Ok(_socket) = std::net::UdpSocket::bind(addr) {
            return port; // Socket drops, port is free for the test server.
        }
    }
    panic!("Could not find a free UDP port after 64 attempts");
}

/// Build a minimal headless server App.
pub fn build_server_app(server_plugin: impl Plugin + 'static) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(server_plugin);
    app.finish();
    app
}

/// Build a minimal headless client App.
pub fn build_client_app(
    net_protocol: impl Plugin + 'static,
    register_net: fn(&mut App),
    server_addr: SocketAddr,
) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));

    app.add_plugins(net_protocol)
        .add_plugins(RepliconSharedPlugin {
            auth_method: AuthMethod::None,
        })
        .add_plugins(bevy_replicon::prelude::ClientPlugin)
        .add_plugins(bevy_replicon::prelude::ClientMessagePlugin);

    register_net(&mut app);

    app.add_plugins(bevy_replicon_renet2::RepliconRenetPlugins)
        .add_systems(
            Startup,
            move |mut commands: Commands, channels: Res<RepliconChannels>| {
                let client_id = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("time went backwards")
                    .as_millis() as u64;

                let current_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("time went backwards");

                let server_configs = channels.server_configs();
                let client_configs = channels.client_configs();
                let connection_config =
                    ConnectionConfig::from_channels(server_configs, client_configs);

                let local_addr =
                    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 0);
                let socket =
                    NativeSocket::new(std::net::UdpSocket::bind(local_addr).expect("bind"))
                        .expect("create socket");

                let authentication = bevy_renet2::netcode::ClientAuthentication::Unsecure {
                    client_id,
                    protocol_id: carcinisation_net::PROTOCOL_ID,
                    socket_id: 0,
                    server_addr,
                    user_data: None,
                };

                let transport = NetcodeClientTransport::new(current_time, authentication, socket)
                    .expect("create client transport");

                let client = RenetClient::new(connection_config, transport.is_reliable());

                commands.insert_resource(client);
                commands.insert_resource(transport);
            },
        )
        .add_systems(
            PreUpdate,
            kickstart_client_transport
                .run_if(resource_added::<RenetClient>)
                .before(bevy_renet2::prelude::RenetReceive),
        );

    app.finish();
    app
}

#[allow(clippy::needless_pass_by_value)]
fn kickstart_client_transport(
    mut client: ResMut<RenetClient>,
    mut transport: ResMut<NetcodeClientTransport>,
    time: Res<Time<Real>>,
) {
    if let Err(e) = transport.update(time.delta(), &mut client) {
        trace!("CLIENT kickstart transport error: {:?}", e);
    }
}

/// Run one update cycle on both server and client.
pub fn update_both(server: &mut App, client: &mut App) {
    server.update();
    client.update();
}

/// Wait up to `max_frames` for a condition to become true.
pub fn wait_for<F>(max_frames: u32, server: &mut App, client: &mut App, mut condition: F) -> bool
where
    F: FnMut(&mut App, &mut App) -> bool,
{
    for _ in 0..max_frames {
        update_both(server, client);
        if condition(server, client) {
            return true;
        }
    }
    false
}

/// Check that `RenetServer` and `NetcodeServerTransport` exist.
pub fn assert_server_resources(app: &App) {
    assert!(
        app.world().get_resource::<RenetServer>().is_some(),
        "RenetServer resource should exist"
    );
    assert!(
        app.world()
            .get_resource::<bevy_renet2::netcode::NetcodeServerTransport>()
            .is_some(),
        "NetcodeServerTransport should exist"
    );
}

/// Check that client is connected or connecting.
pub fn assert_client_connected(app: &App, frame_label: &str) {
    let client_res = app
        .world()
        .get_resource::<RenetClient>()
        .expect("client should exist");
    assert!(
        client_res.is_connected() || client_res.is_connecting(),
        "{}: client unexpectedly disconnected (connected={}, connecting={}, disconnected={})",
        frame_label,
        client_res.is_connected(),
        client_res.is_connecting(),
        client_res.is_disconnected()
    );
}

// ---------------------------------------------------------------------------
// Shared ECS helpers — extracted from per-file duplicates
// ---------------------------------------------------------------------------

use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind};
use carcinisation_net::components::NetEnemy;
use carcinisation_net::{
    ClientIntent, InputSequence, NetAttackId, NetEnemyState, NetHealth, NetPlayer, PlayerActions,
    PlayerId, PlayerNetState,
};
use carcinisation_server::systems::PlayerIntentBuffer;

/// Tick server only with 2 ms sleep (same rationale as `tick_with_sleep`).
pub fn tick_server(server: &mut App) {
    std::thread::sleep(Duration::from_millis(2));
    server.update();
}

/// Tick server-only in a loop until `condition` returns `true`, with early exit.
/// Returns `true` if the condition was met within `max_ticks`.
pub fn wait_for_server_condition(
    server: &mut App,
    max_ticks: u32,
    mut condition: impl FnMut(&mut App) -> bool,
) -> bool {
    for _ in 0..max_ticks {
        tick_server(server);
        if condition(server) {
            return true;
        }
    }
    false
}

/// Build a server with `test_map` + one stationary Mosquiton at the given position.
pub fn build_server_with_enemy(port: u16, enemy_x: f32, enemy_y: f32) -> App {
    build_server_with_enemies(port, test_map(), vec![(enemy_x, enemy_y, 100, 0.0)])
}

/// Build a server with a custom map + multiple Mosquitons.
/// Each tuple is `(x, y, health, speed)`.
pub fn build_server_with_enemies(
    port: u16,
    map: carcinisation_fps_core::map::Map,
    enemies: Vec<(f32, f32, u32, f32)>,
) -> App {
    let entities = enemies
        .into_iter()
        .map(|(x, y, health, speed)| EntitySpawnData {
            kind: EntitySpawnKind::Mosquiton { health, speed },
            x,
            y,
        })
        .collect();
    build_server_app(ServerPlugin {
        port,
        map,
        entities,
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Deterministic (no-sleep) server builders
// ---------------------------------------------------------------------------

use bevy::time::TimeUpdateStrategy;

/// Build a deterministic server — each `app.update()` runs exactly one
/// `FixedUpdate` cycle (30 Hz). No wall-clock dependency, no sleep.
///
/// Uses `port: 0` (OS-assigned) since no client will connect. Intent
/// injection goes through `PlayerIntentBuffer` directly.
pub fn build_deterministic_server_with_enemy(enemy_x: f32, enemy_y: f32) -> App {
    build_deterministic_server_with_enemies(test_map(), vec![(enemy_x, enemy_y, 100, 0.0)])
}

/// Deterministic server with a custom map + multiple Mosquitons.
pub fn build_deterministic_server_with_enemies(
    map: carcinisation_fps_core::map::Map,
    enemies: Vec<(f32, f32, u32, f32)>,
) -> App {
    let entities = enemies
        .into_iter()
        .map(|(x, y, health, speed)| EntitySpawnData {
            kind: EntitySpawnKind::Mosquiton { health, speed },
            x,
            y,
        })
        .collect();
    let mut app = build_server_app(ServerPlugin {
        port: 0,
        map,
        entities,
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app
}

/// Deterministic server with a single stationary **Basic** enemy
/// (`EntitySpawnKind::Enemy` → `NetEnemyType::Basic`). Basic enemies have no
/// AI sim, so they do not move — useful for exercising the authoritative
/// `enemy_type → collision_set(Basic)` hitscan path without movement jitter.
pub fn build_deterministic_server_with_basic_enemy(enemy_x: f32, enemy_y: f32) -> App {
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Enemy {
            color: 0,
            health: 100,
            speed: 0.0,
        },
        x: enemy_x,
        y: enemy_y,
    }];
    let mut app = build_server_app(ServerPlugin {
        port: 0,
        map: test_map(),
        entities,
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app
}

/// Wait up to `max_ticks` (deterministic) for a condition to become true.
/// Each tick is exactly one `FixedUpdate` cycle — no sleep, no jitter.
pub fn wait_for_deterministic(
    server: &mut App,
    max_ticks: u32,
    mut condition: impl FnMut(&mut App) -> bool,
) -> bool {
    for _ in 0..max_ticks {
        server.update();
        if condition(server) {
            return true;
        }
    }
    false
}

/// Spawn an alive player with full health at the given position (angle 0, facing east).
pub fn spawn_alive_player(server: &mut App, pid: u32, x: f32, y: f32) {
    spawn_player_with_state(server, pid, x, y, PlayerNetState::Alive);
}

/// Spawn a player with a specific state.
pub fn spawn_player_with_state(server: &mut App, pid: u32, x: f32, y: f32, state: PlayerNetState) {
    let hp = if matches!(&state, PlayerNetState::Alive) {
        100.0
    } else {
        0.0
    };
    server.world_mut().spawn((
        NetPlayer {
            player_id: PlayerId(pid),
            position: Vec2::new(x, y),
            angle: 0.0,
            current_attack: NetAttackId::None,
            state,
            flame_active: false,
            avatar_palette_variant: None,
        },
        NetHealth {
            current: hp,
            max: 100.0,
        },
        Replicated,
    ));
}

/// Get the first enemy's current health, if any.
pub fn get_enemy_health(server: &mut App) -> Option<f32> {
    server
        .world_mut()
        .query::<(&NetEnemy, &NetHealth)>()
        .iter(server.world())
        .next()
        .map(|(_, h)| h.current)
}

/// Spawn a **static, sim-less Spidey** directly as a replicated `NetEnemy`.
///
/// `tick_net_enemy_ai` only ticks Mosquitons and no `ServerSpideySim` is
/// attached, so this enemy never moves or re-orients — its `position`/`angle`
/// stay exactly as given. That makes per-part hitscan geometry fully
/// deterministic, which is required to aim shots at the head vs the body. The
/// `enemy_type: Spidey` still drives the authoritative `collision_set(Spidey)`
/// path inside real server combat. Returns the spawned entity.
pub fn spawn_static_spidey(
    server: &mut App,
    object_id: u32,
    pos: Vec2,
    angle: f32,
    health: f32,
) -> Entity {
    use carcinisation_net::{NetEnemyType, NetworkObjectId};
    use carcinisation_server::systems::combat::{EnemyGameplayYaw, ServerBurnState};
    server
        .world_mut()
        .spawn((
            NetEnemy {
                object_id: NetworkObjectId(object_id),
                position: pos,
                angle,
                state: NetEnemyState::Idle,
                enemy_type: NetEnemyType::Spidey,
                visual_height: 0.0,
                visual_phase: 0.0,
            },
            NetHealth {
                current: health,
                max: health,
            },
            ServerBurnState::default(),
            // Collision reads gameplay yaw (Phase 13A decouple), so seed it to
            // the same facing as NetEnemy.angle. No sim ⇒ stays static.
            EnemyGameplayYaw(angle),
            Replicated,
        ))
        .id()
}

/// Get the first enemy's state, if any.
pub fn get_enemy_state(server: &mut App) -> Option<NetEnemyState> {
    server
        .world_mut()
        .query::<&NetEnemy>()
        .iter(server.world())
        .next()
        .map(|e| e.state)
}

/// Get a specific player's current health.
pub fn get_player_health(server: &mut App, pid: u32) -> Option<f32> {
    server
        .world_mut()
        .query::<(&NetPlayer, &NetHealth)>()
        .iter(server.world())
        .find(|(p, _)| p.player_id.0 == pid)
        .map(|(_, h)| h.current)
}

/// Set a specific player's health directly.
pub fn set_player_health(server: &mut App, pid: u32, hp: f32) {
    let mut q = server.world_mut().query::<(&NetPlayer, &mut NetHealth)>();
    for (p, mut h) in q.iter_mut(server.world_mut()) {
        if p.player_id.0 == pid {
            h.current = hp;
        }
    }
}

/// Set the first enemy's health directly.
pub fn set_enemy_health(server: &mut App, hp: f32) {
    let mut q = server.world_mut().query::<(&NetEnemy, &mut NetHealth)>();
    for (_, mut h) in q.iter_mut(server.world_mut()) {
        h.current = hp;
    }
}

/// Force the first enemy into a specific state.
pub fn force_enemy_state(server: &mut App, state: NetEnemyState) {
    let mut q = server.world_mut().query::<&mut NetEnemy>();
    for mut e in q.iter_mut(server.world_mut()) {
        e.state = state;
    }
}

/// Force a player's current attack (pistol / flamethrower).
pub fn force_player_attack(server: &mut App, pid: u32, attack: NetAttackId) {
    let mut q = server.world_mut().query::<&mut NetPlayer>();
    for mut p in q.iter_mut(server.world_mut()) {
        if p.player_id.0 == pid {
            p.current_attack = attack;
        }
    }
}

/// Inject a fire-held intent for the given player into the server's intent buffer.
/// Sets `aim_held: true` so fire works in both Legacy and `AimCommitment` modes.
pub fn inject_fire(server: &mut App, pid: u32) {
    server.world_mut().resource_mut::<PlayerIntentBuffer>().set(
        PlayerId(pid),
        &ClientIntent {
            sequence: InputSequence(0),
            movement: Vec2::ZERO,
            turn: 0.0,
            fire_held: true,
            aim_held: true,
            actions: PlayerActions::default(),
        },
    );
}

/// Inject an intent with arbitrary movement and fire state.
pub fn inject_intent(server: &mut App, pid: u32, movement: Vec2, fire_held: bool) {
    server.world_mut().resource_mut::<PlayerIntentBuffer>().set(
        PlayerId(pid),
        &ClientIntent {
            sequence: InputSequence(0),
            movement,
            turn: 0.0,
            fire_held,
            aim_held: false,
            actions: PlayerActions::default(),
        },
    );
}
