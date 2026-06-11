//! Regression tests for dead-player guards and line-of-sight blocking.
//!
//! Deterministic: each `app.update()` = exactly one `FixedUpdate` cycle at 30 Hz.
#![allow(clippy::float_cmp)]

mod common;

use bevy::prelude::*;
use carcinisation_fps_core::map::Map;
use carcinisation_net::{NetAttackId, NetEnemyState, PlayerNetState};
use common::{
    build_deterministic_server_with_basic_enemy, build_deterministic_server_with_enemies,
    build_deterministic_server_with_enemy, force_enemy_state, force_player_attack,
    get_enemy_health, get_enemy_state, get_player_health, inject_fire, inject_intent,
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

// ---------------------------------------------------------------------------
// Phase 3.5: authoritative enemy facing
// ---------------------------------------------------------------------------

/// Read the first enemy's (position, angle) from replicated `NetEnemy`.
fn get_enemy_pose(server: &mut App) -> Option<(Vec2, f32)> {
    server
        .world_mut()
        .query::<&carcinisation_net::NetEnemy>()
        .iter(server.world())
        .next()
        .map(|e| (e.position, e.angle))
}

#[test]
fn enemy_faces_engaged_player_after_tick() {
    use std::f32::consts::{PI, TAU};

    // Enemy east of player; both on the same row. Once the Mosquiton engages,
    // its authoritative `NetEnemy.angle` should orient toward the player.
    let mut server = build_deterministic_server_with_enemy(5.5, 3.5);
    server.update();
    spawn_alive_player(&mut server, 1, 2.5, 3.5);

    let (_, angle_before) = get_enemy_pose(&mut server).unwrap();
    assert_eq!(angle_before, 0.0, "angle defaults to 0 before engaging");

    // Tick enough for AI to run and orient toward the player.
    for _ in 0..5 {
        server.update();
    }

    let player_pos = get_player_position(&mut server, 1).unwrap();
    let (enemy_pos, angle) = get_enemy_pose(&mut server).unwrap();

    let to_player = player_pos - enemy_pos;
    let expected = to_player.y.atan2(to_player.x);
    // Shortest angular difference.
    let diff = ((angle - expected + PI).rem_euclid(TAU) - PI).abs();
    assert!(
        diff < 0.2,
        "enemy should face engaged player: angle={angle:.3}, expected={expected:.3}"
    );
}

#[test]
fn pistol_damages_basic_enemy_via_kind_path() {
    // A stationary Basic enemy directly east of the player. An unobstructed
    // centre shot must damage it, proving the authoritative
    // `NetEnemy.enemy_type` → `collision_set(Basic)` path resolves to a
    // hittable fixture inside the real server combat system (not just the
    // unit-level `fps_kind_from_net` mapping).
    let mut server = build_deterministic_server_with_basic_enemy(4.5, 1.5);
    server.update();

    // Player at (1.5, 1.5) faces east (angle 0) → enemy on the +X fire axis.
    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    let hp_before = get_enemy_health(&mut server).unwrap();

    // `inject_fire` sets aim_held=true so the shot fires in both Legacy and
    // AimCommitment combat modes.
    inject_fire(&mut server, 1);
    for _ in 0..3 {
        server.update();
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    assert!(
        hp_after < hp_before,
        "basic enemy should take pistol damage via per-kind path: {hp_before} -> {hp_after}"
    );
}

// ---------------------------------------------------------------------------
// Phase 11: weapon-only hit reactions
// ---------------------------------------------------------------------------

/// A pistol hit knocks the enemy back along the shot direction, through the
/// real server combat → sim pipeline (reaction queued in CombatSet, consumed
/// by the shared sim in EnemyAttackSet next tick, applied via try_move).
#[test]
fn pistol_hit_knocks_back_enemy() {
    // Mosquiton spawned with speed 0.0 → no AI movement of its own, so any
    // position change along +X is the knockback impulse.
    let mut server = build_deterministic_server_with_enemy(4.5, 1.5);
    server.update();
    spawn_alive_player(&mut server, 1, 1.5, 1.5); // west of enemy, fires east

    let (pos_before, _) = get_enemy_pose(&mut server).unwrap();

    inject_fire(&mut server, 1);
    // ~10 ticks at 30 Hz: one pistol shot (0.33s cooldown) + the full
    // knockback impulse decay (0.12s).
    for _ in 0..10 {
        server.update();
    }

    let (pos_after, _) = get_enemy_pose(&mut server).unwrap();
    assert!(
        pos_after.x > pos_before.x + 0.03,
        "enemy knocked back along shot direction: {} -> {}",
        pos_before.x,
        pos_after.x
    );
    assert!(
        (pos_after.y - pos_before.y).abs() < 0.05,
        "knockback is along the shot axis"
    );
}

#[test]
fn lethal_pistol_hit_clears_pending_reaction() {
    let mut server = build_deterministic_server_with_enemies(
        carcinisation_fps_core::map::test_map(),
        vec![(4.5, 1.5, 1, 0.0)],
    );
    server.update();
    spawn_alive_player(&mut server, 1, 1.5, 1.5);

    inject_fire(&mut server, 1);
    server.update();

    assert!(
        matches!(
            get_enemy_state(&mut server),
            Some(NetEnemyState::Dying { .. })
        ),
        "one pistol hit should kill the low-health Mosquiton"
    );
    let world = server.world_mut();
    let mut sims = world.query::<&carcinisation_server::systems::ServerMosquitonSim>();
    let sim = sims.single(world).expect("spawned Mosquiton sim");
    assert!(
        sim.reaction.pending.is_none() && sim.reaction.pending_next.is_none(),
        "lethal hits must not leave stale reaction state"
    );
}

/// Phase 2 stagger visibility: the server mirrors the shared sim's hit-stun into
/// the replicated `NetEnemy.stunned` so clients can render a stagger cue. Drives
/// the real combat → reaction → sim pipeline with sustained pistol fire and
/// asserts (a) `NetEnemy.stunned` equals `sim.reaction.is_stunned()` every tick
/// (the presentation mirror is exact, never drifts), and (b) a poise break
/// actually occurs end-to-end (4 pistol hits at 25 poise cross the 100
/// threshold). Presentation-only: the stun flag is never read back by the sim.
#[test]
fn server_replicates_hit_stun_to_net_enemy() {
    use carcinisation_net::NetEnemy;
    use carcinisation_server::systems::ServerMosquitonSim;

    // High-HP stationary Mosquiton so it survives long enough to stagger.
    let mut server = build_deterministic_server_with_enemies(
        carcinisation_fps_core::map::test_map(),
        vec![(4.5, 1.5, 1000, 0.0)],
    );
    server.update();
    spawn_alive_player(&mut server, 1, 1.5, 1.5);

    let mut saw_stun = false;
    for _ in 0..150 {
        inject_fire(&mut server, 1);
        server.update();

        let world = server.world_mut();
        let mut q = world.query::<(&NetEnemy, &ServerMosquitonSim)>();
        let Ok((enemy, sim)) = q.single(world) else {
            break; // enemy despawned (should not happen at 1000 HP)
        };
        // The replicated flag is an exact mirror of the authoritative sim state.
        assert_eq!(
            enemy.stunned,
            sim.reaction.is_stunned(),
            "NetEnemy.stunned must mirror EnemyReactionState::is_stunned() each tick"
        );
        if enemy.stunned {
            saw_stun = true;
            break;
        }
    }

    assert!(
        saw_stun,
        "sustained pistol fire crosses the poise threshold and staggers the enemy"
    );
}
