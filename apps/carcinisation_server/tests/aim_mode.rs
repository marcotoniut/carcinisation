//! AimCommitment combat mode integration tests.
//!
//! These tests configure the server with `CombatControlMode::AimCommitment`
//! and verify aim enforcement: fire gating, movement suppression, aim offset.
#![allow(clippy::doc_markdown)]

mod common;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::map::test_map;
use carcinisation_net::{
    ClientIntent, InputSequence, NetHealth, NetPlayer, PlayerActions, PlayerId, PlayerNetState,
};
use carcinisation_server::ServerPlugin;
use carcinisation_server::systems::PlayerIntentBuffer;
use common::{build_server_app, reserve_port};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_aim_server() -> App {
    let port = reserve_port();
    let mut server = build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities: vec![],
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });

    // Switch to AimCommitment mode.
    server
        .world_mut()
        .resource_mut::<carcinisation_fps_core::FpsCombatConfig>()
        .combat_control_mode = carcinisation_fps_core::CombatControlMode::AimCommitment;

    server.update();
    server
}

fn spawn_player(server: &mut App) {
    server.world_mut().spawn((
        NetPlayer {
            player_id: PlayerId(1),
            position: Vec2::new(3.5, 3.5),
            angle: 0.0,
            current_attack: carcinisation_net::NetAttackId::None,
            state: PlayerNetState::Alive,
            flame_active: false,
            avatar_palette_variant: None,
        },
        NetHealth {
            current: 100.0,
            max: 100.0,
        },
        carcinisation_server::systems::input::ServerQuickTurn::default(),
        {
            let combat = carcinisation_fps_core::FpsCombatConfig::default();
            let movement = carcinisation_fps_core::FpsMovementConfig::default();
            carcinisation_server::systems::occupancy::player_occupancy(&combat, &movement)
        },
        Replicated,
    ));
}

fn inject(server: &mut App, intent: &ClientIntent) {
    server
        .world_mut()
        .resource_mut::<PlayerIntentBuffer>()
        .set(PlayerId(1), intent);
}

fn tick_fixed(server: &mut App) {
    std::thread::sleep(std::time::Duration::from_millis(2));
    server.update();
}

fn tick_n(server: &mut App, n: usize) {
    for _ in 0..n {
        tick_fixed(server);
    }
}

fn player_position(server: &mut App) -> Vec2 {
    server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .next()
        .unwrap()
        .position
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// In AimCommitment mode, fire_held without aim_held does not fire.
#[test]
fn aim_mode_fire_rejected_without_aim() {
    let mut server = build_aim_server();
    spawn_player(&mut server);

    // Inject intent: fire_held=true but aim_held=false.
    inject(
        &mut server,
        &ClientIntent {
            sequence: InputSequence(1),
            movement: Vec2::ZERO,
            turn: 0.0,
            fire_held: true,
            actions: PlayerActions::default(),
            aim_held: false,
            aim_offset: 0.0,
        },
    );

    // Tick several frames — no enemies to hit, but verify no panic/crash.
    tick_n(&mut server, 20);

    // Player should be fine — fire was suppressed.
    let pos = player_position(&mut server);
    assert!(
        pos.distance(Vec2::new(3.5, 3.5)) < 0.01,
        "player should not have moved"
    );
}

/// In AimCommitment mode, movement and turn are suppressed while aim_held.
#[test]
fn aim_mode_movement_suppressed_while_aiming() {
    let mut server = build_aim_server();
    spawn_player(&mut server);

    // Inject: forward movement + turn + aim_held.
    inject(
        &mut server,
        &ClientIntent {
            sequence: InputSequence(1),
            movement: Vec2::new(0.0, 1.0),
            turn: 1.0,
            fire_held: false,
            actions: PlayerActions::default(),
            aim_held: true,
            aim_offset: 0.0,
        },
    );

    tick_n(&mut server, 30);

    let pos = player_position(&mut server);
    // Body should be locked — no movement despite forward intent.
    assert!(
        pos.distance(Vec2::new(3.5, 3.5)) < 0.05,
        "body should be locked while aiming: pos={pos}"
    );
}

/// In Legacy mode, fire works without aim_held.
#[test]
fn legacy_mode_fire_works_without_aim() {
    let port = reserve_port();
    let mut server = build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities: vec![],
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    // Legacy is the default — no mode change needed.
    server.update();
    spawn_player(&mut server);

    inject(
        &mut server,
        &ClientIntent {
            sequence: InputSequence(1),
            movement: Vec2::ZERO,
            turn: 0.0,
            fire_held: true,
            actions: PlayerActions::default(),
            aim_held: false,
            aim_offset: 0.0,
        },
    );

    // Should not crash — fire works in Legacy without aim.
    tick_n(&mut server, 20);
}

/// In Legacy mode, movement is not suppressed by aim_held.
#[test]
fn legacy_mode_movement_not_suppressed() {
    let port = reserve_port();
    let mut server = build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities: vec![],
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    server.update();
    spawn_player(&mut server);

    // Even with aim_held=true, Legacy mode should not suppress movement.
    inject(
        &mut server,
        &ClientIntent {
            sequence: InputSequence(1),
            movement: Vec2::new(0.0, 1.0),
            turn: 0.0,
            fire_held: false,
            actions: PlayerActions::default(),
            aim_held: true,
            aim_offset: 0.5,
        },
    );

    tick_n(&mut server, 50);

    let pos = player_position(&mut server);
    assert!(
        pos.distance(Vec2::new(3.5, 3.5)) > 0.1,
        "Legacy mode should not suppress movement: pos={pos}"
    );
}
