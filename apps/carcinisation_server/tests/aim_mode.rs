//! AimCommitment combat mode integration tests.
//!
//! These tests configure the server with `CombatControlMode::AimCommitment`
//! and verify aim enforcement: fire gating, movement suppression, external push.
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
use carcinisation_server::systems::occupancy::ServerPlayerImpulse;
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

fn player_angle(server: &mut App) -> f32 {
    server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .next()
        .unwrap()
        .angle
}

fn player_attack(server: &mut App) -> carcinisation_net::NetAttackId {
    server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .next()
        .unwrap()
        .current_attack
}

fn angle_delta(a: f32, b: f32) -> f32 {
    let delta = (a - b).rem_euclid(std::f32::consts::TAU);
    delta.min(std::f32::consts::TAU - delta)
}

const fn intent_with_action(action: u8, aim_held: bool) -> ClientIntent {
    ClientIntent {
        sequence: InputSequence(1),
        movement: Vec2::ZERO,
        turn: 0.0,
        fire_held: false,
        actions: PlayerActions::from_raw(action),
        aim_held,
    }
}

fn aim_only_intent() -> ClientIntent {
    ClientIntent {
        sequence: InputSequence(2),
        movement: Vec2::ZERO,
        turn: 0.0,
        fire_held: false,
        actions: PlayerActions::default(),
        aim_held: true,
    }
}

fn player_entity(server: &mut App) -> Entity {
    server
        .world_mut()
        .query::<(Entity, &NetPlayer)>()
        .iter(server.world())
        .find(|(_, player)| player.player_id == PlayerId(1))
        .map(|(entity, _)| entity)
        .unwrap()
}

fn insert_test_impulse(server: &mut App) {
    let entity = player_entity(server);
    server
        .world_mut()
        .entity_mut(entity)
        .insert(ServerPlayerImpulse(
            carcinisation_fps_core::occupancy::OccupancyImpulse {
                direction: Vec2::X,
                strength: 6.0,
                remaining: 0.2,
                duration: 0.2,
            },
        ));
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

/// In AimCommitment mode, translation is suppressed but turn is allowed while aim_held.
#[test]
fn aim_mode_movement_suppressed_turn_allowed() {
    let mut server = build_aim_server();
    spawn_player(&mut server);

    let angle_before = player_angle(&mut server);

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
        },
    );

    tick_n(&mut server, 30);

    let pos = player_position(&mut server);
    // Feet locked — no translation despite forward intent.
    assert!(
        pos.distance(Vec2::new(3.5, 3.5)) < 0.05,
        "feet should be locked while aiming: pos={pos}"
    );

    // But turn should have applied — body rotates while aiming.
    let angle_after = player_angle(&mut server);
    let angle_diff = (angle_after - angle_before).abs();
    assert!(
        angle_diff > 0.1,
        "body should turn while aiming: before={angle_before}, after={angle_after}"
    );
}

/// In AimCommitment mode, AimMode suppresses player intent but not external push.
#[test]
fn aim_mode_external_impulse_still_moves_player() {
    let mut server = build_aim_server();
    spawn_player(&mut server);
    inject(
        &mut server,
        &ClientIntent {
            sequence: InputSequence(1),
            movement: Vec2::new(0.0, 1.0),
            turn: 1.0,
            fire_held: false,
            actions: PlayerActions::default(),
            aim_held: true,
        },
    );
    let before = player_position(&mut server);
    insert_test_impulse(&mut server);

    tick_n(&mut server, 30);

    let after = player_position(&mut server);
    assert!(
        after.x > before.x + 0.05,
        "external impulse should move aiming player: before={before}, after={after}"
    );
    assert!(
        (after.y - before.y).abs() < 0.05,
        "voluntary forward movement should remain suppressed while impulse applies"
    );
}

#[test]
fn aim_mode_preserves_pre_aim_weapon_switch_when_packets_coalesce() {
    let mut server = build_aim_server();
    spawn_player(&mut server);

    inject(
        &mut server,
        &intent_with_action(PlayerActions::WEAPON_SWITCH, false),
    );
    inject(&mut server, &aim_only_intent());

    tick_n(&mut server, 30);

    assert_eq!(
        player_attack(&mut server),
        carcinisation_net::NetAttackId::Projectile
    );
}

#[test]
fn aim_mode_preserves_pre_aim_snap_when_packets_coalesce() {
    for (label, action) in [
        ("quick", PlayerActions::QUICK_TURN),
        ("left", PlayerActions::SNAP_TURN_LEFT),
        ("right", PlayerActions::SNAP_TURN_RIGHT),
    ] {
        let mut server = build_aim_server();
        spawn_player(&mut server);
        let angle_before = player_angle(&mut server);

        inject(&mut server, &intent_with_action(action, false));
        inject(&mut server, &aim_only_intent());

        tick_n(&mut server, 30);

        let angle_after = player_angle(&mut server);
        assert!(
            angle_delta(angle_after, angle_before) > 0.1,
            "pre-aim {label} snap action should survive later aim packet: before={angle_before}, after={angle_after}"
        );
    }
}

#[test]
fn aim_mode_preserves_pre_aim_quick_turn_when_packets_coalesce() {
    let mut server = build_aim_server();
    spawn_player(&mut server);
    let angle_before = player_angle(&mut server);

    inject(
        &mut server,
        &intent_with_action(PlayerActions::QUICK_TURN, false),
    );
    inject(&mut server, &aim_only_intent());

    tick_n(&mut server, 30);

    let angle_after = player_angle(&mut server);
    assert!(
        angle_delta(angle_after, angle_before) > 0.1,
        "pre-aim snap action should survive later aim packet: before={angle_before}, after={angle_after}"
    );
}

#[test]
fn legacy_mode_preserves_pre_aim_quick_turn_when_packets_coalesce() {
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
    server
        .world_mut()
        .resource_mut::<carcinisation_fps_core::FpsCombatConfig>()
        .combat_control_mode = carcinisation_fps_core::CombatControlMode::Legacy;
    server.update();
    spawn_player(&mut server);

    let angle_before = player_angle(&mut server);

    inject(
        &mut server,
        &intent_with_action(PlayerActions::QUICK_TURN, false),
    );
    inject(&mut server, &aim_only_intent());

    tick_n(&mut server, 30);

    let angle_after = player_angle(&mut server);
    assert!(
        angle_delta(angle_after, angle_before) > 0.1,
        "Legacy mode should preserve pre-aim quick-turn across coalesced packets: before={angle_before}, after={angle_after}"
    );
}

#[test]
fn aim_mode_rejects_weapon_switch_while_aiming() {
    let mut server = build_aim_server();
    spawn_player(&mut server);

    inject(
        &mut server,
        &ClientIntent {
            sequence: InputSequence(1),
            movement: Vec2::ZERO,
            turn: 0.0,
            fire_held: false,
            actions: PlayerActions::from_raw(PlayerActions::WEAPON_SWITCH),
            aim_held: true,
        },
    );

    tick_n(&mut server, 30);

    assert_eq!(
        player_attack(&mut server),
        carcinisation_net::NetAttackId::None
    );
}

#[test]
fn aim_mode_accepts_weapon_switch_outside_aim_mode() {
    let mut server = build_aim_server();
    spawn_player(&mut server);

    inject(
        &mut server,
        &ClientIntent {
            sequence: InputSequence(1),
            movement: Vec2::ZERO,
            turn: 0.0,
            fire_held: false,
            actions: PlayerActions::from_raw(PlayerActions::WEAPON_SWITCH),
            aim_held: false,
        },
    );

    tick_n(&mut server, 30);

    assert_eq!(
        player_attack(&mut server),
        carcinisation_net::NetAttackId::Projectile
    );
}

#[test]
fn legacy_mode_preserves_weapon_switch_when_packets_coalesce() {
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
    server
        .world_mut()
        .resource_mut::<carcinisation_fps_core::FpsCombatConfig>()
        .combat_control_mode = carcinisation_fps_core::CombatControlMode::Legacy;
    server.update();
    spawn_player(&mut server);

    inject(
        &mut server,
        &intent_with_action(PlayerActions::WEAPON_SWITCH, false),
    );
    inject(&mut server, &aim_only_intent());

    tick_n(&mut server, 30);

    assert_eq!(
        player_attack(&mut server),
        carcinisation_net::NetAttackId::Projectile
    );
}

/// Malformed/stale clients cannot snap turn while holding AimMode.
#[test]
fn aim_mode_rejects_snap_actions_while_aiming_but_preserves_fire() {
    for (label, action) in [
        ("quick", PlayerActions::QUICK_TURN),
        ("left", PlayerActions::SNAP_TURN_LEFT),
        ("right", PlayerActions::SNAP_TURN_RIGHT),
    ] {
        let mut server = build_aim_server();
        spawn_player(&mut server);

        let angle_before = player_angle(&mut server);

        inject(
            &mut server,
            &ClientIntent {
                sequence: InputSequence(1),
                movement: Vec2::new(0.0, 1.0),
                turn: 0.0,
                fire_held: true,
                actions: PlayerActions::from_raw(action),
                aim_held: true,
            },
        );

        tick_fixed(&mut server);

        assert!(
            server
                .world()
                .resource::<PlayerIntentBuffer>()
                .peek_fire_held(&PlayerId(1)),
            "fire should remain held while aiming after rejecting {label} snap action"
        );

        tick_n(&mut server, 29);

        let angle_after = player_angle(&mut server);
        assert!(
            angle_delta(angle_after, angle_before) < 0.01,
            "{label} snap action should be ignored while aiming: before={angle_before}, after={angle_after}"
        );

        let pos = player_position(&mut server);
        assert!(
            pos.distance(Vec2::new(3.5, 3.5)) < 0.05,
            "translation should remain suppressed while aiming with {label} action: pos={pos}"
        );
    }
}

/// AimCommitment only rejects snap actions while AimMode is held.
#[test]
fn aim_mode_accepts_snap_actions_outside_aim_mode() {
    for (label, action) in [
        ("quick", PlayerActions::QUICK_TURN),
        ("left", PlayerActions::SNAP_TURN_LEFT),
        ("right", PlayerActions::SNAP_TURN_RIGHT),
    ] {
        let mut server = build_aim_server();
        spawn_player(&mut server);

        let angle_before = player_angle(&mut server);

        inject(
            &mut server,
            &ClientIntent {
                sequence: InputSequence(1),
                movement: Vec2::ZERO,
                turn: 0.0,
                fire_held: false,
                actions: PlayerActions::from_raw(action),
                aim_held: false,
            },
        );

        tick_n(&mut server, 30);

        let angle_after = player_angle(&mut server);
        assert!(
            angle_delta(angle_after, angle_before) > 0.1,
            "AimCommitment outside AimMode should accept {label} snap action: before={angle_before}, after={angle_after}"
        );
    }
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

    // Explicitly set Legacy mode (RON file may have AimCommitment for playtesting).
    server
        .world_mut()
        .resource_mut::<carcinisation_fps_core::FpsCombatConfig>()
        .combat_control_mode = carcinisation_fps_core::CombatControlMode::Legacy;

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

    // Explicitly set Legacy mode (RON file may have AimCommitment for playtesting).
    server
        .world_mut()
        .resource_mut::<carcinisation_fps_core::FpsCombatConfig>()
        .combat_control_mode = carcinisation_fps_core::CombatControlMode::Legacy;

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
        },
    );

    tick_n(&mut server, 50);

    let pos = player_position(&mut server);
    assert!(
        pos.distance(Vec2::new(3.5, 3.5)) > 0.1,
        "Legacy mode should not suppress movement: pos={pos}"
    );
}

/// In Legacy mode, external impulse still moves the player.
#[test]
fn legacy_mode_external_impulse_still_moves_player() {
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
    server
        .world_mut()
        .resource_mut::<carcinisation_fps_core::FpsCombatConfig>()
        .combat_control_mode = carcinisation_fps_core::CombatControlMode::Legacy;
    server.update();
    spawn_player(&mut server);
    let before = player_position(&mut server);
    insert_test_impulse(&mut server);

    tick_n(&mut server, 30);

    let after = player_position(&mut server);
    assert!(
        after.x > before.x + 0.05,
        "external impulse should move legacy player: before={before}, after={after}"
    );
}

/// Legacy mode still accepts snap actions.
#[test]
fn legacy_mode_accepts_snap_actions() {
    for (label, action) in [
        ("quick", PlayerActions::QUICK_TURN),
        ("left", PlayerActions::SNAP_TURN_LEFT),
        ("right", PlayerActions::SNAP_TURN_RIGHT),
    ] {
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
        server
            .world_mut()
            .resource_mut::<carcinisation_fps_core::FpsCombatConfig>()
            .combat_control_mode = carcinisation_fps_core::CombatControlMode::Legacy;
        server.update();
        spawn_player(&mut server);

        let angle_before = player_angle(&mut server);

        inject(
            &mut server,
            &ClientIntent {
                sequence: InputSequence(1),
                movement: Vec2::ZERO,
                turn: 0.0,
                fire_held: false,
                actions: PlayerActions::from_raw(action),
                aim_held: true,
            },
        );

        tick_n(&mut server, 30);

        let angle_after = player_angle(&mut server);
        assert!(
            angle_delta(angle_after, angle_before) > 0.1,
            "Legacy mode should still accept {label} snap action: before={angle_before}, after={angle_after}"
        );
    }
}
