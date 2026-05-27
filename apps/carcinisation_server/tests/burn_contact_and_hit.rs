//! Tests for burning corpse contact damage and hitscan hit confirmation.
//!
//! Deterministic: each `app.update()` = exactly one FixedUpdate cycle at 30 Hz.
#![allow(clippy::doc_markdown, clippy::float_cmp)]

mod common;

use bevy::prelude::*;
use carcinisation_net::{NetEnemyState, NetProjectile, NetProjectileType, NetworkObjectId, Owner};
use common::{
    build_deterministic_server_with_enemy, force_enemy_state, get_enemy_health, get_player_health,
    inject_fire, set_enemy_health, spawn_alive_player, wait_for_deterministic,
};

// ---------------------------------------------------------------------------
// Burning corpse contact damage tests
// ---------------------------------------------------------------------------

/// Player near a burning corpse takes contact damage over time.
#[test]
fn burn_corpse_damages_nearby_player() {
    // Enemy at (3.0, 1.5), player at (3.2, 1.5) — very close.
    let mut server = build_deterministic_server_with_enemy(3.0, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 3.2, 1.5);

    // Kill enemy with flamethrower flag to create burning corpse.
    force_enemy_state(&mut server, NetEnemyState::Dying { burn: true });
    set_enemy_health(&mut server, 0.0);

    let hp_before = get_player_health(&mut server, 1).unwrap();

    // 30 fixed ticks at 30 Hz = 1 s — enough for burn contact.
    let damaged = wait_for_deterministic(&mut server, 30, |server| {
        get_player_health(server, 1).unwrap() < hp_before
    });

    assert!(
        damaged,
        "player near burning corpse should take contact damage"
    );
}

/// Player far from burning corpse takes no contact damage.
#[test]
fn burn_corpse_does_not_damage_distant_player() {
    // Enemy at (1.5, 1.5), player at (6.5, 6.5) — far away.
    let mut server = build_deterministic_server_with_enemy(1.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 6.5, 6.5);

    // Create burning corpse.
    force_enemy_state(&mut server, NetEnemyState::Dying { burn: true });
    set_enemy_health(&mut server, 0.0);

    // 30 fixed ticks at 30 Hz = 1 s
    for _ in 0..30 {
        server.update();
    }

    let hp = get_player_health(&mut server, 1).unwrap();
    assert_eq!(
        hp, 100.0,
        "distant player should not take burn contact damage"
    );
}

// ---------------------------------------------------------------------------
// Pending projectile / shoot lead tests
// ---------------------------------------------------------------------------

/// Pending projectile system spawns NetProjectile after delay.
#[test]
fn pending_projectile_spawns_after_delay() {
    let mut server = build_deterministic_server_with_enemy(3.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    force_enemy_state(&mut server, NetEnemyState::HoldingRange);

    // Cooldown 2 s + lead 0.1 s = ~63 fixed ticks. Wait up to 120 (4 s).
    let found = wait_for_deterministic(&mut server, 120, |server| {
        server
            .world_mut()
            .query::<&carcinisation_net::NetProjectile>()
            .iter(server.world())
            .count()
            > 0
    });

    assert!(found, "pending projectile should eventually spawn");
}

// ---------------------------------------------------------------------------
// Hitscan hit confirmation test
// ---------------------------------------------------------------------------

/// Hitscan hit on enemy produces exactly hitscan_damage (37).
#[test]
fn hitscan_hit_damages_enemy() {
    // Enemy at (3.0, 1.5), player at (1.5, 1.5) facing east.
    let mut server = build_deterministic_server_with_enemy(3.0, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);

    let hp_before = get_enemy_health(&mut server).unwrap();

    inject_fire(&mut server, 1);
    // 3 fixed ticks at 30 Hz = 0.1 s
    for _ in 0..3 {
        server.update();
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    let damage = hp_before - hp_after;
    assert!(
        (damage - carcinisation_fps_core::FpsCombatConfig::default().hitscan_damage).abs() < 1.0,
        "hitscan should deal {} damage: got {damage}",
        carcinisation_fps_core::FpsCombatConfig::default().hitscan_damage
    );
}

// ---------------------------------------------------------------------------
// Projectile-player collision test
// ---------------------------------------------------------------------------

/// Enemy projectile hitting a player reduces their health.
#[test]
fn enemy_projectile_damages_player() {
    let mut server = build_deterministic_server_with_enemy(6.5, 6.5);
    server.update();

    spawn_alive_player(&mut server, 1, 3.0, 1.5);

    // Spawn a projectile aimed directly at the player from the east.
    server.world_mut().spawn((
        NetProjectile {
            object_id: NetworkObjectId(99),
            position: Vec2::new(5.0, 1.5),
            angle: std::f32::consts::PI, // Facing west toward player at (3.0, 1.5).
            owner: Owner(carcinisation_net::PlayerId(0)),
            damage: 15.0,
            projectile_type: NetProjectileType::BloodShot,
        },
        carcinisation_server::systems::ProjectileTtl(3.0),
        bevy_replicon::prelude::Replicated,
    ));

    let hp_before = get_player_health(&mut server, 1).unwrap();

    // 2 units at speed 4 = 0.5 s = 15 fixed ticks. Wait up to 30.
    let damaged = wait_for_deterministic(&mut server, 30, |server| {
        get_player_health(server, 1).unwrap() < hp_before
    });

    assert!(
        damaged,
        "enemy projectile should damage player: hp started at {hp_before}"
    );
}
