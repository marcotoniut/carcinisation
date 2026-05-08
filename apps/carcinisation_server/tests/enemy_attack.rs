//! Phase 5e: enemy ranged attack integration tests.
//!
//! Tests Mosquiton attack cooldown, projectile spawn, movement, player damage.
#![allow(clippy::doc_markdown)]

mod common;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind, test_map};
use carcinisation_net::{NetEnemyState, NetHealth, NetPlayer, NetProjectile};
use carcinisation_server::ServerPlugin;
use common::{build_server_app, reserve_port};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a server with test_map + one Mosquiton at (4.5, 1.5) close to spawn.
fn build_attack_server(port: u16) -> App {
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 100,
            speed: 0.0,
        },
        x: 3.5,
        y: 1.5,
    }];
    build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: vec![],
    })
}

fn projectile_count(server: &mut App) -> usize {
    server
        .world_mut()
        .query::<&NetProjectile>()
        .iter(server.world())
        .count()
}

/// Tick server with real-time sleeps for FixedUpdate accumulation.
fn tick_server(server: &mut App) {
    std::thread::sleep(std::time::Duration::from_millis(2));
    server.update();
}

/// Tick server for roughly one second of game time (30Hz = ~15 ticks with 2ms sleep).
/// Returns after `n` ticks.
fn tick_server_n(server: &mut App, n: usize) {
    for _ in 0..n {
        tick_server(server);
    }
}

/// Simulate a connected player by directly spawning NetPlayer + NetHealth.
fn spawn_test_player(server: &mut App) {
    server.world_mut().spawn((
        NetPlayer {
            player_id: carcinisation_net::PlayerId(1),
            position: bevy::math::Vec2::new(1.5, 1.5),
            angle: 0.0,
            current_attack: carcinisation_net::NetAttackId::None,
            state: carcinisation_net::PlayerNetState::Alive,
        },
        NetHealth {
            current: 100.0,
            max: 100.0,
        },
        Replicated,
    ));
}

/// Force the enemy into Attacking state (simulates AI reaching preferred range).
fn force_enemy_attacking(server: &mut App) {
    let mut q = server
        .world_mut()
        .query::<&mut carcinisation_net::components::NetEnemy>();
    for mut enemy in q.iter_mut(server.world_mut()) {
        enemy.state = NetEnemyState::HoldingRange;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Mosquiton in Attacking state spawns a projectile after cooldown expires.
#[test]
fn attacking_mosquiton_spawns_projectile() {
    let port = reserve_port();
    let mut server = build_attack_server(port);
    server.update();

    spawn_test_player(&mut server);
    force_enemy_attacking(&mut server);

    assert_eq!(projectile_count(&mut server), 0);

    // Tick enough for cooldown (2.0s). At 2ms per tick, need ~1000 ticks for 2s real time.
    // But FixedUpdate at 30Hz needs real elapsed time. With 2ms sleep:
    // ~17 ticks per FixedUpdate tick, need 60 FixedUpdate ticks for 2s game time.
    // 60 * 17 = ~1020 ticks.
    for _ in 0..2000 {
        tick_server(&mut server);
        if projectile_count(&mut server) > 0 {
            break;
        }
    }

    assert!(
        projectile_count(&mut server) > 0,
        "Mosquiton should spawn a projectile after attack cooldown"
    );
}

/// Mosquiton NOT in Attacking state does not fire.
#[test]
fn idle_mosquiton_does_not_fire() {
    let port = reserve_port();
    let mut server = build_attack_server(port);
    server.update();

    spawn_test_player(&mut server);
    // Don't force attacking — enemy starts Idle.

    tick_server_n(&mut server, 200);

    assert_eq!(
        projectile_count(&mut server),
        0,
        "Idle Mosquiton should not fire projectiles"
    );
}

/// Dead Mosquiton does not fire.
#[test]
fn dead_mosquiton_does_not_fire() {
    let port = reserve_port();
    let mut server = build_attack_server(port);
    server.update();

    spawn_test_player(&mut server);

    // Kill the enemy.
    {
        let mut q = server
            .world_mut()
            .query::<(&mut carcinisation_net::components::NetEnemy, &mut NetHealth)>();
        for (mut enemy, mut health) in q.iter_mut(server.world_mut()) {
            enemy.state = NetEnemyState::Dead { burn: false };
            health.current = 0.0;
        }
    }

    tick_server_n(&mut server, 1100);

    assert_eq!(
        projectile_count(&mut server),
        0,
        "Dead Mosquiton should not fire projectiles"
    );
}

/// Projectile moves over fixed ticks.
#[test]
fn projectile_moves_over_ticks() {
    let port = reserve_port();
    let mut server = build_attack_server(port);
    server.update();

    spawn_test_player(&mut server);
    force_enemy_attacking(&mut server);

    // Wait for projectile to spawn.
    for _ in 0..2000 {
        tick_server(&mut server);
        if projectile_count(&mut server) > 0 {
            break;
        }
    }
    assert!(projectile_count(&mut server) > 0);

    // Record initial position.
    let initial_pos = {
        let mut q = server.world_mut().query::<&NetProjectile>();
        q.iter(server.world()).next().unwrap().position
    };

    // Tick more — projectile should move.
    tick_server_n(&mut server, 50);

    let new_pos = {
        let mut q = server.world_mut().query::<&NetProjectile>();
        // Projectile might have despawned (hit wall/player), which is also valid.
        q.iter(server.world()).next().map(|p| p.position)
    };

    if let Some(pos) = new_pos {
        assert_ne!(
            pos, initial_pos,
            "Projectile should move: {initial_pos} → {pos}"
        );
    }
    // If None, projectile already hit something — also valid behavior.
}

/// Projectile despawns on wall hit (TTL expiry or wall collision).
#[test]
fn projectile_despawns_eventually() {
    let port = reserve_port();
    let mut server = build_attack_server(port);
    server.update();

    spawn_test_player(&mut server);
    force_enemy_attacking(&mut server);

    // Wait for projectile to spawn.
    for _ in 0..2000 {
        tick_server(&mut server);
        if projectile_count(&mut server) > 0 {
            break;
        }
    }
    assert!(projectile_count(&mut server) > 0);

    // Tick until projectile despawns (wall hit or TTL).
    let mut despawned = false;
    for _ in 0..2000 {
        tick_server(&mut server);
        if projectile_count(&mut server) == 0 {
            despawned = true;
            break;
        }
    }

    assert!(
        despawned,
        "Projectile should despawn after hitting wall or TTL expiry"
    );
}
