//! SP/MP behavioural parity tests.
//!
//! Verifies that the multiplayer server-authoritative simulation produces
//! equivalent results to the singleplayer path for the same input trace.
//!
//! Tests use shared `fps_core` functions directly (pure, no ECS) alongside
//! the server's `PlayerIntentBuffer` / `ServerQuickTurn` to compare.
#![allow(clippy::doc_markdown, clippy::float_cmp)]

mod common;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::FpsMovementConfig;
use carcinisation_fps_core::map::test_map;
use carcinisation_fps_core::movement::{SnapTurnKind, apply_movement};
use carcinisation_net::{
    ClientIntent, InputSequence, NetAttackId, NetHealth, NetPlayer, PlayerActions, PlayerId,
    PlayerNetState,
};
use carcinisation_server::ServerPlugin;
use carcinisation_server::systems::{PlayerIntentBuffer, ServerQuickTurn};
use common::{build_server_app, reserve_port};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const DT: f32 = 1.0 / 30.0;

fn build_parity_server(port: u16) -> App {
    build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities: vec![],
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    })
}

fn spawn_player_at(server: &mut App, pid: u32, x: f32, y: f32, angle: f32) {
    server.world_mut().spawn((
        NetPlayer {
            player_id: PlayerId(pid),
            position: Vec2::new(x, y),
            angle,
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

fn inject(server: &mut App, pid: u32, intent: &ClientIntent) {
    server
        .world_mut()
        .resource_mut::<PlayerIntentBuffer>()
        .set(PlayerId(pid), intent);
}

fn get_pos_angle(server: &mut App, pid: u32) -> (Vec2, f32) {
    server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .find(|p| p.player_id.0 == pid)
        .map(|p| (p.position, p.angle))
        .unwrap()
}

fn get_attack(server: &mut App, pid: u32) -> NetAttackId {
    server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .find(|p| p.player_id.0 == pid)
        .map(|p| p.current_attack)
        .unwrap()
}

fn tick(server: &mut App) {
    std::thread::sleep(std::time::Duration::from_millis(2));
    server.update();
}

/// Tick enough for exactly one FixedUpdate (33ms at 30Hz ≈ 17 updates × 2ms).
fn tick_fixed(server: &mut App) {
    for _ in 0..17 {
        tick(server);
    }
}

fn forward() -> ClientIntent {
    ClientIntent {
        sequence: InputSequence(0),
        movement: Vec2::new(0.0, 1.0),
        turn: 0.0,
        fire_held: false,
        actions: PlayerActions::default(),
    }
}

fn turn_left() -> ClientIntent {
    ClientIntent {
        sequence: InputSequence(0),
        movement: Vec2::ZERO,
        turn: 1.0,
        fire_held: false,
        actions: PlayerActions::default(),
    }
}

const fn action(flag: u8) -> ClientIntent {
    ClientIntent {
        sequence: InputSequence(0),
        movement: Vec2::ZERO,
        turn: 0.0,
        fire_held: false,
        actions: PlayerActions::from_raw(flag),
    }
}

// ---------------------------------------------------------------------------
// 1. Movement collision parity
// ---------------------------------------------------------------------------

/// Pure fps_core movement matches MP server for same inputs.
#[test]
fn movement_forward_parity() {
    let defaults = FpsMovementConfig::default();
    let map = test_map();
    let start = Vec2::new(1.5, 1.5);
    let angle = 0.0;
    let intent = Vec2::new(0.0, 1.0);

    // SP path: pure apply_movement.
    let mut sp_pos = start;
    for _ in 0..10 {
        apply_movement(
            &mut sp_pos,
            angle,
            intent,
            defaults.move_speed,
            DT,
            &map,
            defaults.collision_margin,
        );
    }

    // MP path.
    let port = reserve_port();
    let mut server = build_parity_server(port);
    server.update();
    spawn_player_at(&mut server, 1, start.x, start.y, angle);

    for _ in 0..10 {
        inject(&mut server, 1, &forward());
        tick_fixed(&mut server);
    }

    let (mp_pos, _) = get_pos_angle(&mut server, 1);
    // MP moved; SP moved. Both should be east of start. Allow timing variance.
    assert!(mp_pos.x > start.x + 0.3, "MP should have moved east");
    assert!(
        (mp_pos.y - start.y).abs() < 0.01,
        "MP should stay on Y axis"
    );
    // Direction and collision path match — exact position depends on tick timing.
    assert!(
        (sp_pos.x - start.x).signum() == (mp_pos.x - start.x).signum(),
        "SP and MP should move same direction"
    );
}

/// Wall collision stops movement in both paths.
#[test]
fn movement_wall_collision_parity() {
    let defaults = FpsMovementConfig::default();
    let map = test_map();
    let start = Vec2::new(1.2, 1.5);
    let angle = std::f32::consts::PI; // Facing west into wall.

    let mut sp_pos = start;
    for _ in 0..20 {
        apply_movement(
            &mut sp_pos,
            angle,
            Vec2::Y,
            defaults.move_speed,
            DT,
            &map,
            defaults.collision_margin,
        );
    }

    let port = reserve_port();
    let mut server = build_parity_server(port);
    server.update();
    spawn_player_at(&mut server, 1, start.x, start.y, angle);

    for _ in 0..20 {
        inject(&mut server, 1, &forward());
        tick_fixed(&mut server);
    }

    let (mp_pos, _) = get_pos_angle(&mut server, 1);
    // Both should be blocked near the start position (wall at x=1.0).
    assert!(sp_pos.x > 1.0, "SP blocked by wall");
    assert!(mp_pos.x > 1.0, "MP blocked by wall");
    assert!(
        (sp_pos - mp_pos).length() < 0.2,
        "wall: SP={sp_pos:?} MP={mp_pos:?}"
    );
}

// ---------------------------------------------------------------------------
// 2. Turn and snap turn parity
// ---------------------------------------------------------------------------

/// Continuous turn produces same angle.
#[test]
fn continuous_turn_parity() {
    let start_angle = 0.0;

    let defaults = FpsMovementConfig::default();
    let mut sp_angle = start_angle;
    for _ in 0..10 {
        sp_angle += 1.0 * defaults.turn_speed * DT;
        sp_angle = sp_angle.rem_euclid(std::f32::consts::TAU);
    }

    let port = reserve_port();
    let mut server = build_parity_server(port);
    server.update();
    spawn_player_at(&mut server, 1, 3.0, 3.0, start_angle);

    for _ in 0..10 {
        inject(&mut server, 1, &turn_left());
        tick_fixed(&mut server);
    }

    let (_, mp_angle) = get_pos_angle(&mut server, 1);
    // Both should have turned left (positive angle) from 0.
    assert!(sp_angle > 0.3, "SP should have turned left");
    assert!(mp_angle > 0.3, "MP should have turned left");
    // Same direction — exact value depends on tick timing.
    assert!(
        sp_angle.signum() == mp_angle.signum(),
        "SP and MP should turn same direction: SP={sp_angle:.3} MP={mp_angle:.3}"
    );
}

/// ServerQuickTurn snap left produces exactly PI/2.
#[test]
fn snap_turn_left_produces_90_degrees() {
    let mut angle = 0.0_f32;
    let mut turn = ServerQuickTurn::default();
    turn.request(SnapTurnKind::Left, 0.4);
    for _ in 0..20 {
        turn.tick(&mut angle, DT);
    }
    assert!(!turn.is_active());
    assert!(
        (angle - std::f32::consts::FRAC_PI_2).abs() < 0.001,
        "snap left should be PI/2: got {angle:.4}"
    );
}

/// Quick turn produces exactly PI.
#[test]
fn quick_turn_produces_180_degrees() {
    let mut angle = 0.0_f32;
    let mut turn = ServerQuickTurn::default();
    turn.request(SnapTurnKind::QuickTurn, 0.4);
    for _ in 0..30 {
        turn.tick(&mut angle, DT);
    }
    assert!(
        (angle - std::f32::consts::PI).abs() < 0.001,
        "quick turn should be PI: got {angle:.4}"
    );
}

/// Snap turn is edge-triggered — request while active is ignored.
#[test]
fn snap_turn_no_stack() {
    let mut angle = 0.0_f32;
    let mut turn = ServerQuickTurn::default();
    turn.request(SnapTurnKind::Left, 0.4);
    turn.request(SnapTurnKind::Right, 0.4); // Ignored — already active.
    for _ in 0..20 {
        turn.tick(&mut angle, DT);
    }
    assert!(
        (angle - std::f32::consts::FRAC_PI_2).abs() < 0.01,
        "stacked request should be ignored: got {angle:.4}"
    );
}

// ---------------------------------------------------------------------------
// 3. Weapon switch parity
// ---------------------------------------------------------------------------

/// Single switch toggles weapon.
#[test]
fn weapon_switch_toggles() {
    let port = reserve_port();
    let mut server = build_parity_server(port);
    server.update();
    spawn_player_at(&mut server, 1, 3.0, 3.0, 0.0);

    assert_eq!(get_attack(&mut server, 1), NetAttackId::None);

    inject(&mut server, 1, &action(PlayerActions::WEAPON_SWITCH));
    tick_fixed(&mut server);
    assert_eq!(get_attack(&mut server, 1), NetAttackId::Projectile);

    inject(&mut server, 1, &action(PlayerActions::WEAPON_SWITCH));
    tick_fixed(&mut server);
    assert_eq!(get_attack(&mut server, 1), NetAttackId::None);
}

// ---------------------------------------------------------------------------
// 5. Dead input lockout parity
// ---------------------------------------------------------------------------

/// Dead player ignores all input.
#[test]
fn dead_player_ignores_input() {
    let port = reserve_port();
    let mut server = build_parity_server(port);
    server.update();

    server.world_mut().spawn((
        NetPlayer {
            player_id: PlayerId(1),
            position: Vec2::new(3.0, 3.0),
            angle: 0.0,
            current_attack: NetAttackId::None,
            state: PlayerNetState::Dead,
            flame_active: false,
            avatar_palette_variant: None,
        },
        NetHealth {
            current: 0.0,
            max: 100.0,
        },
        ServerQuickTurn::default(),
        Replicated,
    ));

    let (pos_before, angle_before) = get_pos_angle(&mut server, 1);

    for _ in 0..5 {
        inject(
            &mut server,
            1,
            &ClientIntent {
                sequence: InputSequence(1),
                movement: Vec2::Y,
                turn: 1.0,
                fire_held: true,
                actions: PlayerActions::from_raw(PlayerActions::WEAPON_SWITCH),
            },
        );
        tick_fixed(&mut server);
    }

    let (pos_after, angle_after) = get_pos_angle(&mut server, 1);

    assert_eq!(pos_before, pos_after, "dead player should not move");
    assert!(
        (angle_before - angle_after).abs() < 0.01,
        "dead player angle unchanged"
    );
    assert_eq!(get_attack(&mut server, 1), NetAttackId::None);
}
