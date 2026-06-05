//! Deterministic replay tests — verify identical inputs produce identical
//! simulation hashes across independent runs.
//!
//! Uses `TimeUpdateStrategy::FixedTimesteps(1)` so each `app.update()` runs
//! exactly one fixed tick without wall-clock dependency.

mod common;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind, test_map};
use carcinisation_net::{
    ClientIntent, InputSequence, NetAttackId, NetEnemy, NetHealth, NetPlayer, PlayerActions,
    PlayerId, PlayerNetState,
    sim_hash::{collect_enemy_state, collect_player_state, compute_sim_hash},
};
use carcinisation_server::ServerPlugin;
use common::build_server_app;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a replay server with no real networking.
///
/// Port 0 lets the OS assign a free port atomically — no TOCTOU race from
/// `reserve_port()` probe-then-bind. The transport is never used (no clients
/// connect; entities are spawned directly).
fn build_replay_server() -> App {
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 100,
            speed: 0.0,
        },
        x: 3.5,
        y: 1.5,
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
    // Deterministic: each app.update() = exactly 1 fixed tick.
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app
}

fn spawn_player(server: &mut App, pid: u32, x: f32, y: f32) {
    use carcinisation_server::systems::ServerQuickTurn;
    server.world_mut().spawn((
        NetPlayer {
            player_id: PlayerId(pid),
            position: bevy::math::Vec2::new(x, y),
            angle: 0.0,
            current_attack: NetAttackId::None,
            state: PlayerNetState::Alive,
            flame_active: false,
            avatar_palette_variant: None,
        },
        NetHealth {
            current: 100.0,
            max: 100.0,
        },
        ServerQuickTurn::default(),
        Replicated,
    ));
}

fn inject_intent(
    server: &mut App,
    pid: u32,
    seq: u32,
    movement: bevy::math::Vec2,
    fire_held: bool,
) {
    use carcinisation_server::systems::PlayerIntentBuffer;
    server.world_mut().resource_mut::<PlayerIntentBuffer>().set(
        PlayerId(pid),
        &ClientIntent {
            sequence: InputSequence(seq),
            movement,
            turn: 0.0,
            fire_held,
            aim_held: fire_held,
            actions: PlayerActions::default(),
        },
    );
}

fn snapshot_hash(server: &mut App) -> u64 {
    let players: Vec<_> = server
        .world_mut()
        .query::<(&NetPlayer, &NetHealth)>()
        .iter(server.world())
        .collect();
    let player_state = collect_player_state(&players);

    let enemies: Vec<_> = server
        .world_mut()
        .query::<(&NetEnemy, &NetHealth)>()
        .iter(server.world())
        .collect();
    let enemy_state = collect_enemy_state(&enemies);

    let projs: Vec<_> = server
        .world_mut()
        .query::<&carcinisation_net::NetProjectile>()
        .iter(server.world())
        .cloned()
        .collect();

    compute_sim_hash(&player_state, &enemy_state, &projs)
}

/// Run a deterministic simulation: N ticks of movement input, recording
/// per-tick hashes.
fn run_replay(ticks: usize) -> Vec<u64> {
    let mut server = build_replay_server();
    server.update(); // init

    spawn_player(&mut server, 1, 1.5, 1.5);

    let mut hashes = Vec::with_capacity(ticks);
    for tick in 0..ticks {
        // Inject forward movement every tick.
        inject_intent(
            &mut server,
            1,
            u32::try_from(tick).unwrap(),
            bevy::math::Vec2::new(0.0, 1.0),
            false,
        );
        server.update();
        hashes.push(snapshot_hash(&mut server));
    }
    hashes
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Two independent runs with identical inputs must produce identical per-tick
/// simulation hashes.
#[test]
fn deterministic_replay_produces_identical_hashes() {
    let ticks = 60; // 2 seconds at 30 Hz

    let hashes_a = run_replay(ticks);
    let hashes_b = run_replay(ticks);

    assert_eq!(
        hashes_a.len(),
        hashes_b.len(),
        "both runs should produce same number of ticks"
    );

    for (i, (a, b)) in hashes_a.iter().zip(hashes_b.iter()).enumerate() {
        assert_eq!(
            a, b,
            "hash divergence at tick {i}: run_a={a:#018x} run_b={b:#018x}"
        );
    }
}

/// Simulation state should change over time (not stuck at initial hash).
#[test]
fn replay_state_evolves() {
    let hashes = run_replay(30);

    let first = hashes[0];
    let last = hashes[hashes.len() - 1];
    assert_ne!(
        first, last,
        "state should change between tick 0 and tick 29 (player is moving)"
    );
}
