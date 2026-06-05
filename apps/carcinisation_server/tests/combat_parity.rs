//! SP/MP combat parity tests.
//!
//! Compares multiplayer server combat semantics against fps_core expected
//! behavior (shared hitscan, damage, death transitions).
//!
//! Uses `TimeUpdateStrategy::FixedTimesteps(1)` — each `app.update()` runs
//! exactly one FixedUpdate cycle at 30 Hz. No wall-clock dependency.
#![allow(clippy::doc_markdown, clippy::float_cmp)]

mod common;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::camera::Camera;
use carcinisation_fps_core::enemy::{Enemy, hitscan};
use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind, test_map};
use carcinisation_net::{
    ClientIntent, InputSequence, NetAttackId, NetEnemyState, NetHealth, NetPlayer, PlayerActions,
    PlayerId, PlayerNetState,
};
use carcinisation_server::ServerPlugin;
use carcinisation_server::systems::ServerQuickTurn;
use common::{build_deterministic_server_with_enemies, build_server_app};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn hitscan_damage() -> f32 {
    carcinisation_fps_core::FpsCombatConfig::default().hitscan_damage
}

fn build_combat_server(enemy_x: f32, enemy_y: f32) -> App {
    build_deterministic_server_with_enemies(test_map(), vec![(enemy_x, enemy_y, 200, 0.0)])
}

fn spawn_player(server: &mut App, pid: u32, x: f32, y: f32, angle: f32) {
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
    use carcinisation_server::systems::PlayerIntentBuffer;
    server
        .world_mut()
        .resource_mut::<PlayerIntentBuffer>()
        .set(PlayerId(pid), intent);
}

fn fire_intent() -> ClientIntent {
    ClientIntent {
        sequence: InputSequence(0),
        movement: Vec2::ZERO,
        turn: 0.0,
        fire_held: true,
        actions: PlayerActions::default(),
    }
}

const fn switch_intent() -> ClientIntent {
    ClientIntent {
        sequence: InputSequence(0),
        movement: Vec2::ZERO,
        turn: 0.0,
        fire_held: false,
        actions: PlayerActions::from_raw(PlayerActions::WEAPON_SWITCH),
    }
}

const fn switch_and_fire_intent() -> ClientIntent {
    ClientIntent {
        sequence: InputSequence(0),
        movement: Vec2::ZERO,
        turn: 0.0,
        fire_held: true,
        actions: PlayerActions::from_raw(PlayerActions::WEAPON_SWITCH),
    }
}

fn get_enemy_health(server: &mut App) -> f32 {
    common::get_enemy_health(server).unwrap()
}

fn get_enemy_state(server: &mut App) -> NetEnemyState {
    common::get_enemy_state(server).unwrap()
}

// ---------------------------------------------------------------------------
// 1. Pistol fire cooldown parity
// ---------------------------------------------------------------------------

/// fps_core hitscan produces same hit as MP server hitscan.
#[test]
fn hitscan_shared_function_parity() {
    // Both SP and MP use fps_core::hitscan with same types.
    let map = test_map();
    let cam = Camera {
        position: Vec2::new(1.5, 1.5),
        angle: 0.0, // Facing east.
        ..Default::default()
    };
    let enemies = vec![Enemy::new(Vec2::new(4.5, 1.5), 200, 0.0)];
    let result = hitscan(&cam, &enemies, &map);
    assert_eq!(
        result.enemy_idx,
        Some(0),
        "hitscan should hit enemy at (4.5,1.5)"
    );
    assert!(result.distance > 2.0 && result.distance < 4.0);
}

/// MP server pistol fires once per cooldown period, dealing exactly hitscan_damage().
#[test]
fn pistol_cooldown_parity() {
    // Enemy at (4.5, 1.5), player at (1.5, 1.5) facing east.
    let mut server = build_combat_server(4.5, 1.5);
    server.update();
    spawn_player(&mut server, 1, 1.5, 1.5, 0.0);

    let initial_hp = get_enemy_health(&mut server);

    // Hold fire for 30 fixed ticks (= 1 s at 30 Hz).
    // At 0.33 s cooldown, expect ~3 shots in 1 s.
    for _ in 0..30 {
        inject(&mut server, 1, &fire_intent());
        server.update();
    }

    let final_hp = get_enemy_health(&mut server);
    let total_damage = initial_hp - final_hp;

    let dmg = hitscan_damage();

    // Expected: ~3 shots × dmg. Allow 1 shot variance for edge timing.
    let expected_min = dmg * 2.0;
    let expected_max = dmg * 5.0;
    assert!(
        total_damage >= expected_min && total_damage <= expected_max,
        "pistol cooldown: damage={total_damage:.0} (expected {expected_min:.0}–{expected_max:.0})"
    );

    // Each shot does exactly dmg — total should be a multiple.
    let shot_count = (total_damage / dmg).round();
    let remainder = (total_damage - shot_count * dmg).abs();
    assert!(
        remainder < 1.0,
        "damage should be multiple of {dmg}: got {total_damage:.1} (remainder {remainder:.1})"
    );
}

// ---------------------------------------------------------------------------
// 2. Weapon switch then fire parity
// ---------------------------------------------------------------------------

/// Switch to flamethrower then fire — MP server applies flame DPS, not hitscan.
#[test]
fn switch_to_flamethrower_then_fire() {
    // Enemy at (3.5, 1.5) — within flame range (5.0) of spawn (1.5, 1.5).
    let mut server = build_combat_server(3.5, 1.5);
    server.update();
    spawn_player(&mut server, 1, 1.5, 1.5, 0.0);

    let initial_hp = get_enemy_health(&mut server);

    // Switch to flamethrower.
    inject(&mut server, 1, &switch_intent());
    server.update();

    // Hold fire for 10 fixed ticks (= 0.33 s at 30 Hz).
    for _ in 0..10 {
        inject(&mut server, 1, &fire_intent());
        server.update();
    }

    let hp_after = get_enemy_health(&mut server);
    let damage = initial_hp - hp_after;

    // Burn system deals progressive damage — much less than old 580 DPS.
    // After 10 ticks (~0.33 s), burn intensity is building; expect at least some damage.
    assert!(
        damage > 0.0,
        "flamethrower should deal continuous damage: got {damage:.0}"
    );
}

// ---------------------------------------------------------------------------
// 3. Flamethrower damage parity
// ---------------------------------------------------------------------------

/// Flamethrower damage uses progressive burn: intensity builds over exposure,
/// then damage accumulates proportionally. After sustained fire, DPS ramps up
/// toward `damage_per_sec_at_max` (70) + `direct_contact_dps` (10) = 80 DPS.
///
/// First few ticks produce little damage because intensity starts at 0 and
/// the damage accumulator needs to cross the integer threshold.
#[test]
fn flamethrower_dps_per_tick() {
    let mut server = build_combat_server(3.5, 1.5);
    server.update();
    spawn_player(&mut server, 1, 1.5, 1.5, 0.0);

    // Switch to flamethrower.
    inject(&mut server, 1, &switch_intent());
    server.update();

    let hp_before = get_enemy_health(&mut server);

    // Fire for exactly 30 fixed ticks (= 1 s at 30 Hz).
    for _ in 0..30 {
        inject(&mut server, 1, &fire_intent());
        server.update();
    }

    let hp_after = get_enemy_health(&mut server);
    let total_damage = hp_before - hp_after;

    // After 1 s of sustained flame, burn intensity reaches near max.
    // Expected total: ramp from 0 → ~80 DPS over 1 s ≈ 30–60 damage.
    assert!(
        total_damage > 10.0,
        "sustained flame should deal significant damage over 1 s: got {total_damage:.1}"
    );
    assert!(
        total_damage < 100.0,
        "sustained flame damage should be bounded: got {total_damage:.1}"
    );
}

/// Flamethrower blocked by wall does no damage (shared with guards_and_los.rs but
/// verifying here that flame line-distance LOS check matches fps_core raycast).
#[test]
fn flamethrower_wall_blocked_parity() {
    // Wall at (4,1) in LOS test map blocks (1.5,1.5) → (5.5,1.5).
    let map = carcinisation_fps_core::map::Map {
        width: 8,
        height: 8,
        cells: vec![
            1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0, 0, 1, // Wall at (4,1).
            1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0,
            0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        ],
    };
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 100,
            speed: 0.0,
        },
        x: 5.5,
        y: 1.5,
    }];
    let mut server = build_server_app(ServerPlugin {
        port: 0,
        map,
        entities,
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    server.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(1));
    server.update();
    spawn_player(&mut server, 1, 1.5, 1.5, 0.0);

    // Switch to flamethrower and fire.
    inject(&mut server, 1, &switch_intent());
    server.update();

    let hp_before = get_enemy_health(&mut server);
    for _ in 0..10 {
        inject(&mut server, 1, &fire_intent());
        server.update();
    }
    let hp_after = get_enemy_health(&mut server);

    assert_eq!(
        hp_before, hp_after,
        "flame blocked by wall should deal no damage"
    );
}

// ---------------------------------------------------------------------------
// 4. Death state parity
// ---------------------------------------------------------------------------

/// Lethal hitscan damage transitions enemy through Dying → Dead (matching SP
/// Enemy::take_damage → EnemyState::Dying → EnemyState::Dead flow).
#[test]
fn lethal_damage_dying_dead_parity() {
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 30,
            speed: 0.0,
        },
        x: 4.5,
        y: 1.5,
    }];
    let mut server = build_server_app(ServerPlugin {
        port: 0,
        map: test_map(),
        entities,
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    server.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(1));
    server.update();
    spawn_player(&mut server, 1, 1.5, 1.5, 0.0);

    // Fire once.
    inject(&mut server, 1, &fire_intent());
    server.update();

    let state = get_enemy_state(&mut server);
    // Should be Dying (not Dead yet — 0.5 s death timer).
    assert!(
        matches!(state, NetEnemyState::Dying { burn: false }),
        "lethal hitscan should transition to Dying: got {state:?}"
    );

    // SP comparison: fps_core Enemy::take_damage produces EnemyState::Dying { timer: 0.5 }.
    let mut sp_enemy = Enemy::new(Vec2::new(4.5, 1.5), 30, 0.0);
    sp_enemy.take_damage(37);
    assert!(
        matches!(
            sp_enemy.state,
            carcinisation_fps_core::EnemyState::Dying { .. }
        ),
        "SP Enemy::take_damage should produce Dying"
    );

    // Wait for death timer (0.5 s = 15 fixed ticks at 30 Hz).
    for _ in 0..20 {
        server.update();
    }

    let state_after = get_enemy_state(&mut server);
    assert!(
        matches!(state_after, NetEnemyState::Dead { burn: false }),
        "after death timer: should be Dead {{ burn: false }}, got {state_after:?}"
    );
}

/// Flamethrower lethal damage produces Dying { burn: true }.
#[test]
fn flame_lethal_damage_burn_parity() {
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 10,
            speed: 0.0,
        },
        x: 3.5,
        y: 1.5,
    }];
    let mut server = build_server_app(ServerPlugin {
        port: 0,
        map: test_map(),
        entities,
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    server.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(1));
    server.update();
    spawn_player(&mut server, 1, 1.5, 1.5, 0.0);

    // Switch to flamethrower and fire — burn builds intensity progressively.
    inject(&mut server, 1, &switch_and_fire_intent());
    for _ in 0..60 {
        inject(&mut server, 1, &fire_intent());
        server.update();
    }

    let state = get_enemy_state(&mut server);
    assert!(
        matches!(
            state,
            NetEnemyState::Dying { burn: true } | NetEnemyState::Dead { burn: true }
        ),
        "flame kill should produce burn=true: got {state:?}"
    );

    // SP comparison: fps_core Enemy::take_damage_from(Fire) produces BurningCorpse.
    let mut sp_enemy = Enemy::new(Vec2::new(3.5, 1.5), 10, 0.0);
    sp_enemy.take_damage_from(20, carcinisation_fps_core::DamageKind::Fire, 0.5);
    assert!(
        matches!(
            sp_enemy.state,
            carcinisation_fps_core::EnemyState::BurningCorpse { .. }
        ),
        "SP fire kill should produce BurningCorpse"
    );
}
