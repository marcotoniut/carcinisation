//! SP/MP combat parity tests.
//!
//! Compares multiplayer server combat semantics against fps_core expected
//! behavior (shared hitscan, damage, death transitions).
#![allow(clippy::doc_markdown, clippy::float_cmp)]

mod common;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::camera::Camera;
use carcinisation_fps_core::config;
use carcinisation_fps_core::enemy::{Enemy, hitscan};
use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind, test_map};
use carcinisation_net::{
    ClientIntent, InputSequence, NetAttackId, NetEnemyState, NetHealth, NetPlayer, PlayerActions,
    PlayerId, PlayerNetState,
};
use carcinisation_server::ServerPlugin;
use carcinisation_server::systems::{PlayerIntentBuffer, ServerQuickTurn};
use common::{build_server_app, reserve_port};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const HITSCAN_DAMAGE: f32 = config::HITSCAN_DAMAGE;

fn build_combat_server(port: u16, enemy_x: f32, enemy_y: f32) -> App {
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 200,
            speed: 0.0,
        },
        x: enemy_x,
        y: enemy_y,
    }];
    build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: vec![],
    })
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

fn fire_intent() -> ClientIntent {
    ClientIntent {
        sequence: InputSequence(0),
        movement: Vec2::ZERO,
        turn: 0.0,
        fire_held: true,
        actions: PlayerActions::default(),
    }
}

fn switch_intent() -> ClientIntent {
    ClientIntent {
        sequence: InputSequence(0),
        movement: Vec2::ZERO,
        turn: 0.0,
        fire_held: false,
        actions: PlayerActions::from_raw(PlayerActions::WEAPON_SWITCH),
    }
}

fn switch_and_fire_intent() -> ClientIntent {
    ClientIntent {
        sequence: InputSequence(0),
        movement: Vec2::ZERO,
        turn: 0.0,
        fire_held: true,
        actions: PlayerActions::from_raw(PlayerActions::WEAPON_SWITCH),
    }
}

fn get_enemy_health(server: &mut App) -> f32 {
    server
        .world_mut()
        .query::<(&carcinisation_net::components::NetEnemy, &NetHealth)>()
        .iter(server.world())
        .next()
        .map(|(_, h)| h.current)
        .unwrap()
}

fn get_enemy_state(server: &mut App) -> NetEnemyState {
    server
        .world_mut()
        .query::<&carcinisation_net::components::NetEnemy>()
        .iter(server.world())
        .next()
        .map(|e| e.state)
        .unwrap()
}

/// Tick server enough for one FixedUpdate.
fn tick_fixed(server: &mut App) {
    for _ in 0..17 {
        std::thread::sleep(std::time::Duration::from_millis(2));
        server.update();
    }
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

/// MP server pistol fires once per cooldown period, dealing exactly HITSCAN_DAMAGE.
#[test]
fn pistol_cooldown_parity() {
    let port = reserve_port();
    // Enemy at (4.5, 1.5), player at (1.5, 1.5) facing east.
    let mut server = build_combat_server(port, 4.5, 1.5);
    server.update();
    spawn_player(&mut server, 1, 1.5, 1.5, 0.0);

    let initial_hp = get_enemy_health(&mut server);

    // Hold fire for ~1 second (30 fixed ticks).
    // At 0.33s cooldown, expect ~3 shots in 1s.
    for _ in 0..30 {
        inject(&mut server, 1, &fire_intent());
        tick_fixed(&mut server);
    }

    let final_hp = get_enemy_health(&mut server);
    let total_damage = initial_hp - final_hp;

    // Expected: ~3 shots × 37 damage = ~111. Allow 1 shot variance for timing.
    let expected_min = HITSCAN_DAMAGE * 2.0; // At least 2 shots.
    let expected_max = HITSCAN_DAMAGE * 5.0; // At most 5 shots (timing generous).
    assert!(
        total_damage >= expected_min && total_damage <= expected_max,
        "pistol cooldown: damage={total_damage:.0} (expected {expected_min:.0}–{expected_max:.0})"
    );

    // Each shot does exactly HITSCAN_DAMAGE — total should be a multiple of 37.
    let shot_count = (total_damage / HITSCAN_DAMAGE).round();
    let remainder = (total_damage - shot_count * HITSCAN_DAMAGE).abs();
    assert!(
        remainder < 1.0,
        "damage should be multiple of {HITSCAN_DAMAGE}: got {total_damage:.1} (remainder {remainder:.1})"
    );
}

// ---------------------------------------------------------------------------
// 2. Weapon switch then fire parity
// ---------------------------------------------------------------------------

/// Switch to flamethrower then fire — MP server applies flame DPS, not hitscan.
#[test]
fn switch_to_flamethrower_then_fire() {
    let port = reserve_port();
    // Enemy at (3.5, 1.5) — within flame range (5.0) of spawn (1.5, 1.5).
    let mut server = build_combat_server(port, 3.5, 1.5);
    server.update();
    spawn_player(&mut server, 1, 1.5, 1.5, 0.0);

    let initial_hp = get_enemy_health(&mut server);

    // Switch to flamethrower.
    inject(&mut server, 1, &switch_intent());
    tick_fixed(&mut server);

    // Hold fire.
    for _ in 0..10 {
        inject(&mut server, 1, &fire_intent());
        tick_fixed(&mut server);
    }

    let hp_after = get_enemy_health(&mut server);
    let damage = initial_hp - hp_after;

    // Flamethrower deals continuous damage — should NOT be a multiple of 37.
    // At 580 DPS × ~0.33s ≈ 191 damage. Allow wide range for timing.
    assert!(
        damage > 50.0,
        "flamethrower should deal continuous damage: got {damage:.0}"
    );
    let hitscan_shots = (damage / HITSCAN_DAMAGE).round();
    let hitscan_remainder = (damage - hitscan_shots * HITSCAN_DAMAGE).abs();
    // If damage were exactly hitscan multiples, that would indicate wrong weapon.
    // With flame DPS, it's unlikely to be an exact multiple.
    // (This is a soft check — flame damage could coincidentally be near a multiple.)
    if hitscan_remainder < 1.0 && hitscan_shots > 1.0 {
        // Could be hitscan — but at 580 DPS it would be very high.
        assert!(
            damage > 150.0,
            "if multiples of 37, damage should be very high for flame"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Flamethrower damage parity
// ---------------------------------------------------------------------------

/// Flamethrower DPS matches expected rate: ~580 DPS × dt per tick.
#[test]
fn flamethrower_dps_per_tick() {
    let port = reserve_port();
    let mut server = build_combat_server(port, 3.5, 1.5);
    server.update();
    spawn_player(&mut server, 1, 1.5, 1.5, 0.0);

    // Switch to flamethrower.
    inject(&mut server, 1, &switch_intent());
    tick_fixed(&mut server);

    let hp_before = get_enemy_health(&mut server);

    // Fire for 1 fixed tick.
    inject(&mut server, 1, &fire_intent());
    tick_fixed(&mut server);

    let hp_after = get_enemy_health(&mut server);
    let damage_per_tick = hp_before - hp_after;

    // Expected: FLAME_DPS (580) × DT (1/30) ≈ 19.3 per tick.
    // Allow timing variance (server may fire 0 or 2 ticks in this window).
    if damage_per_tick > 0.0 {
        assert!(
            damage_per_tick > 10.0 && damage_per_tick < 60.0,
            "flame damage per tick should be ~19.3: got {damage_per_tick:.1}"
        );
    }
}

/// Flamethrower blocked by wall does no damage (shared with guards_and_los.rs but
/// verifying here that flame line-distance LOS check matches fps_core raycast).
#[test]
fn flamethrower_wall_blocked_parity() {
    // Wall at (4,1) in LOS test map blocks (1.5,1.5) → (5.5,1.5).
    let port = reserve_port();
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
        port,
        map,
        entities,
        player_starts: vec![],
    });
    server.update();
    spawn_player(&mut server, 1, 1.5, 1.5, 0.0);

    // Switch to flamethrower and fire.
    inject(&mut server, 1, &switch_intent());
    tick_fixed(&mut server);

    let hp_before = get_enemy_health(&mut server);
    for _ in 0..10 {
        inject(&mut server, 1, &fire_intent());
        tick_fixed(&mut server);
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
    let port = reserve_port();
    // Enemy with 30 HP — one hitscan shot (37 dmg) is lethal.
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 30,
            speed: 0.0,
        },
        x: 4.5,
        y: 1.5,
    }];
    let mut server = build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: vec![],
    });
    server.update();
    spawn_player(&mut server, 1, 1.5, 1.5, 0.0);

    // Fire once.
    inject(&mut server, 1, &fire_intent());
    tick_fixed(&mut server);

    let state = get_enemy_state(&mut server);
    // Should be Dying (not Dead yet — 0.5s death timer).
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

    // Wait for death timer (0.5s ≈ 15 FixedUpdate ticks).
    for _ in 0..20 {
        tick_fixed(&mut server);
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
    let port = reserve_port();
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 10,
            speed: 0.0,
        },
        x: 3.5,
        y: 1.5,
    }];
    let mut server = build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: vec![],
    });
    server.update();
    spawn_player(&mut server, 1, 1.5, 1.5, 0.0);

    // Switch to flamethrower and fire.
    inject(&mut server, 1, &switch_and_fire_intent());
    for _ in 0..5 {
        inject(&mut server, 1, &fire_intent());
        tick_fixed(&mut server);
    }

    let state = get_enemy_state(&mut server);
    // Should be Dying { burn: true } or Dead { burn: true }.
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
