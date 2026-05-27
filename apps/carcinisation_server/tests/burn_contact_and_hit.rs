//! Tests for burning corpse contact damage and hitscan hit confirmation.
#![allow(clippy::doc_markdown, clippy::float_cmp)]

mod common;

use bevy::prelude::*;
use carcinisation_net::{NetEnemyState, NetProjectile, NetProjectileType, NetworkObjectId, Owner};
use common::{
    build_server_with_enemy, force_enemy_state, get_enemy_health, get_player_health, inject_fire,
    reserve_port, set_enemy_health, spawn_alive_player, tick_server, wait_for_server_condition,
};

// ---------------------------------------------------------------------------
// Burning corpse contact damage tests
// ---------------------------------------------------------------------------

/// Player near a burning corpse takes contact damage over time.
#[test]
fn burn_corpse_damages_nearby_player() {
    let port = reserve_port();
    // Enemy at (3.0, 1.5), player at (3.2, 1.5) — very close.
    let mut server = build_server_with_enemy(port, 3.0, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 3.2, 1.5);

    // Kill enemy with flamethrower flag to create burning corpse.
    force_enemy_state(&mut server, NetEnemyState::Dying { burn: true });
    set_enemy_health(&mut server, 0.0);

    let hp_before = get_player_health(&mut server, 1).unwrap();

    // Wait for burn contact to apply (early exit once damage detected).
    // 500 ticks at 2 ms ≈ 30 FixedUpdate cycles at 30 Hz ≈ 1 s game time.
    let damaged = wait_for_server_condition(&mut server, 500, |server| {
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
    let port = reserve_port();
    // Enemy at (1.5, 1.5), player at (6.5, 6.5) — far away.
    let mut server = build_server_with_enemy(port, 1.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 6.5, 6.5);

    // Create burning corpse.
    force_enemy_state(&mut server, NetEnemyState::Dying { burn: true });
    set_enemy_health(&mut server, 0.0);

    // 500 ticks at 2 ms ≈ 30 FixedUpdate cycles at 30 Hz ≈ 1 s game time.
    for _ in 0..500 {
        tick_server(&mut server);
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
    let port = reserve_port();
    let mut server = build_server_with_enemy(port, 3.5, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);
    force_enemy_state(&mut server, NetEnemyState::HoldingRange);

    // Tick until projectile spawns (cooldown 2 s + lead 0.1 s).
    // 2000 ticks at 2 ms ≈ 120 FixedUpdate cycles at 30 Hz ≈ 4 s game time.
    let found = wait_for_server_condition(&mut server, 2000, |server| {
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
    let port = reserve_port();
    // Enemy at (3.0, 1.5), player at (1.5, 1.5) facing east.
    let mut server = build_server_with_enemy(port, 3.0, 1.5);
    server.update();

    spawn_alive_player(&mut server, 1, 1.5, 1.5);

    let hp_before = get_enemy_health(&mut server).unwrap();

    inject_fire(&mut server, 1);
    // 50 ticks at 2 ms sleep ≈ 3 FixedUpdate cycles at 30 Hz
    for _ in 0..50 {
        tick_server(&mut server);
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
    let port = reserve_port();
    let mut server = build_server_with_enemy(port, 6.5, 6.5);
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

    // Tick until projectile reaches the player (2 units at speed 4 = 0.5 s).
    // 500 ticks at 2 ms ≈ 30 FixedUpdate cycles at 30 Hz ≈ 1 s game time.
    let damaged = wait_for_server_condition(&mut server, 500, |server| {
        get_player_health(server, 1).unwrap() < hp_before
    });

    assert!(
        damaged,
        "enemy projectile should damage player: hp started at {hp_before}"
    );
}
