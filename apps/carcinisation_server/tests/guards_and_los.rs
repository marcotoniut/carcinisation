//! Regression tests for dead-player guards and line-of-sight blocking.
//!
//! Deterministic: each `app.update()` = exactly one FixedUpdate cycle at 30 Hz.
#![allow(clippy::float_cmp)]

mod common;

use bevy::prelude::*;
use carcinisation_fps_core::map::Map;
use carcinisation_net::{NetAttackId, NetEnemyState, PlayerNetState};
use common::{
    build_deterministic_server_with_enemies, build_deterministic_server_with_enemy,
    force_enemy_state, force_player_attack, get_enemy_health, get_player_health, inject_intent,
    spawn_alive_player, spawn_player_with_state,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// 8x8 open map with a wall column at (4, 1) blocking east LOS from (1.5, 1.5).
fn los_test_map() -> Map {
    #[rustfmt::skip]
    let cells = vec![
        1, 1, 1, 1, 1, 1, 1, 1,
        1, 0, 0, 0, 1, 0, 0, 1,  // wall at (4,1) blocks (1.5,1.5) → (5.5,1.5)
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 1, 1, 1, 1, 1, 1, 1,
    ];
    Map {
        width: 8,
        height: 8,
        cells,
    }
}

fn build_los_server(enemy_x: f32, enemy_y: f32) -> App {
    build_deterministic_server_with_enemies(los_test_map(), vec![(enemy_x, enemy_y, 100, 0.0)])
}

fn get_player_position(server: &mut App, pid: u32) -> Option<Vec2> {
    server
        .world_mut()
        .query::<&carcinisation_net::NetPlayer>()
        .iter(server.world())
        .find(|p| p.player_id.0 == pid)
        .map(|p| p.position)
}

// ---------------------------------------------------------------------------
// Dead-player guard tests (parameterised)
// ---------------------------------------------------------------------------

/// Dead player actions are no-ops. Covers: movement, pistol, flamethrower.
fn assert_dead_player_action_is_noop(
    label: &str,
    enemy_pos: (f32, f32),
    attack: NetAttackId,
    fire_held: bool,
    movement: Vec2,
    check_position: bool,
) {
    let mut server = build_deterministic_server_with_enemy(enemy_pos.0, enemy_pos.1);
    server.update();

    spawn_player_with_state(&mut server, 1, 1.5, 1.5, PlayerNetState::Dead);
    if attack != NetAttackId::None {
        force_player_attack(&mut server, 1, attack);
    }

    let pos_before = get_player_position(&mut server, 1).unwrap();
    let hp_before = get_enemy_health(&mut server).unwrap();

    inject_intent(&mut server, 1, movement, fire_held);
    // 3 fixed ticks at 30 Hz = 0.1 s
    for _ in 0..3 {
        server.update();
    }

    if check_position {
        let pos_after = get_player_position(&mut server, 1).unwrap();
        assert_eq!(
            pos_before, pos_after,
            "{label}: dead player should not move: {pos_before} → {pos_after}"
        );
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    assert_eq!(
        hp_before, hp_after,
        "{label}: dead player should not damage enemy: {hp_before} → {hp_after}"
    );
}

#[test]
fn dead_player_cannot_move() {
    assert_dead_player_action_is_noop(
        "movement",
        (6.5, 6.5), // enemy far away
        NetAttackId::None,
        false,
        Vec2::new(0.0, 1.0),
        true, // check position
    );
}

#[test]
fn dead_player_cannot_fire_pistol() {
    assert_dead_player_action_is_noop(
        "pistol",
        (4.5, 1.5), // enemy directly east
        NetAttackId::None,
        true,
        Vec2::ZERO,
        false,
    );
}

#[test]
fn dead_player_cannot_use_flamethrower() {
    assert_dead_player_action_is_noop(
        "flamethrower",
        (3.5, 1.5), // enemy within flame range
        NetAttackId::Projectile,
        true,
        Vec2::ZERO,
        false,
    );
}

// ---------------------------------------------------------------------------
// LOS blocking tests
// ---------------------------------------------------------------------------

/// Pistol hitscan blocked by wall does not damage enemy behind it.
#[test]
fn pistol_blocked_by_wall() {
    // Player at (1.5, 1.5), enemy at (5.5, 1.5), wall at (4, 1) between them.
    let mut server = build_los_server(5.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    let hp_before = get_enemy_health(&mut server).unwrap();

    inject_intent(&mut server, 1, Vec2::ZERO, true);
    // 3 fixed ticks at 30 Hz = 0.1 s
    for _ in 0..3 {
        server.update();
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    assert_eq!(
        hp_before, hp_after,
        "pistol should not damage enemy behind wall: {hp_before} → {hp_after}"
    );
}

/// Flamethrower blocked by wall does not damage enemy behind it.
#[test]
fn flamethrower_blocked_by_wall() {
    let mut server = build_los_server(5.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    force_player_attack(&mut server, 1, NetAttackId::Projectile);
    let hp_before = get_enemy_health(&mut server).unwrap();

    inject_intent(&mut server, 1, Vec2::ZERO, true);
    // 3 fixed ticks at 30 Hz = 0.1 s
    for _ in 0..3 {
        server.update();
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    assert_eq!(
        hp_before, hp_after,
        "flamethrower should not damage enemy behind wall: {hp_before} → {hp_after}"
    );
}

/// Enemy projectile hits wall and despawns before reaching player behind it.
#[test]
fn enemy_projectile_blocked_by_wall() {
    // Enemy at (5.5, 1.5), player at (1.5, 1.5), wall at (4, 1) between them.
    let mut server = build_los_server(5.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    force_enemy_state(&mut server, NetEnemyState::HoldingRange);

    // 120 fixed ticks at 30 Hz = 4 s — enough for projectile spawn + travel + wall hit.
    for _ in 0..120 {
        server.update();
    }

    let hp = get_player_health(&mut server, 1).unwrap();
    assert_eq!(
        hp, 100.0,
        "player behind wall should not take projectile damage: hp={hp}"
    );
}
