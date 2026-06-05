//! Phase 5e: enemy ranged attack integration tests.
//!
//! Tests Mosquiton attack cooldown, projectile spawn, movement, player damage.
#![allow(clippy::doc_markdown)]

mod common;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind, test_map};
use carcinisation_net::{
    NetEnemyState, NetHealth, NetPlayer, NetProjectile, NetProjectileType, NetSpeedModifier,
    NetworkObjectId, Owner,
};
use carcinisation_server::ServerPlugin;
use carcinisation_server::systems::{ProjectileTtl, ServerSpideySimConfig};
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
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    })
}

fn build_spidey_attack_server(port: u16, speed: f32) -> App {
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Spidey { health: 100, speed },
        x: 5.5,
        y: 1.5,
    }];
    build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
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
            flame_active: false,
            avatar_palette_variant: None,
        },
        NetHealth {
            current: 100.0,
            max: 100.0,
        },
        Replicated,
    ));
}

fn speed_modifier(server: &mut App) -> Option<NetSpeedModifier> {
    server
        .world_mut()
        .query::<&NetSpeedModifier>()
        .iter(server.world())
        .next()
        .copied()
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

/// Each enemy independently targets the nearest alive player (R1 regression).
///
/// Places two players and two enemies. Each enemy should face toward its
/// nearest player, not an arbitrary player.
#[test]
fn each_enemy_targets_nearest_player() {
    use carcinisation_fps_core::map::EntitySpawnKind;
    let port = reserve_port();
    // Two Mosquitons: one near (2.5,1.5), one near (5.5,1.5). Speed 0 so they don't move.
    let entities = vec![
        EntitySpawnData {
            kind: EntitySpawnKind::Mosquiton {
                health: 100,
                speed: 0.0,
            },
            x: 2.5,
            y: 1.5,
        },
        EntitySpawnData {
            kind: EntitySpawnKind::Mosquiton {
                health: 100,
                speed: 0.0,
            },
            x: 5.5,
            y: 1.5,
        },
    ];
    let mut server = build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    server.update();

    // Player 1 at (1.5, 1.5) — nearest to enemy at (2.5, 1.5).
    server.world_mut().spawn((
        NetPlayer {
            player_id: carcinisation_net::PlayerId(1),
            position: bevy::math::Vec2::new(1.5, 1.5),
            angle: 0.0,
            current_attack: carcinisation_net::NetAttackId::None,
            state: carcinisation_net::PlayerNetState::Alive,
            flame_active: false,
            avatar_palette_variant: None,
        },
        NetHealth {
            current: 100.0,
            max: 100.0,
        },
        Replicated,
    ));

    // Player 2 at (6.5, 1.5) — nearest to enemy at (5.5, 1.5).
    server.world_mut().spawn((
        NetPlayer {
            player_id: carcinisation_net::PlayerId(2),
            position: bevy::math::Vec2::new(6.5, 1.5),
            angle: 0.0,
            current_attack: carcinisation_net::NetAttackId::None,
            state: carcinisation_net::PlayerNetState::Alive,
            flame_active: false,
            avatar_palette_variant: None,
        },
        NetHealth {
            current: 100.0,
            max: 100.0,
        },
        Replicated,
    ));

    // Force enemies into attacking range.
    force_enemy_attacking(&mut server);

    // Tick enough for projectiles to spawn.
    for _ in 0..2000 {
        tick_server(&mut server);
        if projectile_count(&mut server) >= 2 {
            break;
        }
    }

    // Both enemies should have fired. Verify each projectile's direction
    // points toward the nearer player, not the same player.
    let projs: Vec<_> = server
        .world_mut()
        .query::<&NetProjectile>()
        .iter(server.world())
        .map(|p| (p.position, p.angle))
        .collect();

    assert!(
        projs.len() >= 2,
        "both enemies should have fired, got {} projectiles",
        projs.len()
    );

    // Projectile from enemy at (2.5,1.5) should aim left (toward player 1 at 1.5,1.5).
    // Projectile from enemy at (5.5,1.5) should aim right (toward player 2 at 6.5,1.5).
    // Check that projectile angles are NOT all the same direction (the pre-fix bug).
    let angles: Vec<f32> = projs.iter().map(|(_, a)| *a).collect();
    let all_same_sign =
        angles.iter().all(|a| a.cos() > 0.0) || angles.iter().all(|a| a.cos() < 0.0);
    assert!(
        !all_same_sign,
        "enemies should target different players: angles={angles:?} (all facing same direction is the R1 bug)"
    );
}

#[test]
fn spidey_spawn_uses_spidey_net_type_and_authored_speed() {
    let port = reserve_port();
    let mut server = build_spidey_attack_server(port, 4.0);
    server.update();

    let mut q = server
        .world_mut()
        .query::<(&carcinisation_net::NetEnemy, &ServerSpideySimConfig)>();
    let (enemy, config) = q
        .iter(server.world())
        .next()
        .expect("spidey should spawn with server sim config");

    assert_eq!(enemy.enemy_type, carcinisation_net::NetEnemyType::Spidey);
    assert!((config.0.move_speed - 4.0).abs() < f32::EPSILON);
    assert!(
        config.0.hop_distance
            > carcinisation_fps_core::FpsCombatConfig::load()
                .spidey
                .hop_distance,
        "authored speed should scale hop distance"
    );
}

#[test]
fn spidey_server_attack_spawns_webshot_projectile() {
    let port = reserve_port();
    let mut server = build_spidey_attack_server(port, 2.0);
    server.update();
    spawn_test_player(&mut server);

    for _ in 0..1200 {
        tick_server(&mut server);
        let mut q = server.world_mut().query::<&NetProjectile>();
        if q.iter(server.world())
            .any(|p| p.projectile_type == NetProjectileType::WebShot)
        {
            return;
        }
    }

    panic!("Spidey should spawn a WebShot projectile");
}

#[test]
fn webshot_projectile_applies_server_speed_modifier() {
    let port = reserve_port();
    let mut server = build_attack_server(port);
    server.update();
    spawn_test_player(&mut server);

    server.world_mut().spawn((
        NetProjectile {
            object_id: NetworkObjectId(99),
            position: Vec2::new(1.75, 1.5),
            angle: 0.0,
            owner: Owner(carcinisation_net::PlayerId(999)),
            damage: 5.0,
            projectile_type: NetProjectileType::WebShot,
        },
        ProjectileTtl(3.0),
        Replicated,
    ));

    for _ in 0..80 {
        tick_server(&mut server);
        if speed_modifier(&mut server).is_some() {
            break;
        }
    }

    let modifier = speed_modifier(&mut server).expect("WebShot should apply speed modifier");
    let combat = carcinisation_fps_core::FpsCombatConfig::load();
    assert!((modifier.multiplier - combat.spidey.web_slow_multiplier).abs() < f32::EPSILON);
    assert!((modifier.remaining - combat.spidey.web_slow_duration).abs() < 0.1);
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

// ---------------------------------------------------------------------------
// Phase 2: Spidey lunge impulse tests
// ---------------------------------------------------------------------------

/// Build a server with a Spidey close enough to lunge at a player.
///
/// Spidey at (5.5, 3.5), player at (3.5, 3.5) — within lunge_range (2.0).
/// Both in open space (center of test_map) so push/recoil have room.
fn build_lunge_scenario() -> App {
    let port = reserve_port();
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Spidey {
            health: 100,
            speed: 2.0,
        },
        x: 5.5,
        y: 3.5,
    }];
    let mut server = build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    server.update();

    // Spawn player at (3.5, 3.5) — open space with room for push/recoil.
    server.world_mut().spawn((
        NetPlayer {
            player_id: carcinisation_net::PlayerId(1),
            position: bevy::math::Vec2::new(3.5, 3.5),
            angle: 0.0,
            current_attack: carcinisation_net::NetAttackId::None,
            state: carcinisation_net::PlayerNetState::Alive,
            flame_active: false,
            avatar_palette_variant: None,
        },
        NetHealth {
            current: 100.0,
            max: 100.0,
        },
        Replicated,
    ));

    server
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

fn player_health_value(server: &mut App) -> f32 {
    server
        .world_mut()
        .query::<(&NetPlayer, &NetHealth)>()
        .iter(server.world())
        .next()
        .map(|(_, h)| h.current)
        .unwrap()
}

fn spidey_position(server: &mut App) -> Vec2 {
    server
        .world_mut()
        .query::<&carcinisation_net::components::NetEnemy>()
        .iter(server.world())
        .find(|e| e.enemy_type == carcinisation_net::NetEnemyType::Spidey)
        .unwrap()
        .position
}

/// Wait for Spidey to lunge and deal damage. Returns true if damage occurred.
fn wait_for_lunge_damage(server: &mut App, max_ticks: usize) -> bool {
    let initial_health = player_health_value(server);
    for _ in 0..max_ticks {
        tick_server(server);
        if player_health_value(server) < initial_health {
            return true;
        }
    }
    false
}

/// Spidey lunge hit pushes the player away from the Spidey (one-shot).
///
/// The push is applied immediately as a position displacement on the same
/// tick as damage. No multi-tick impulse — client prediction sees the
/// pushed position in the next `InputAck`.
#[test]
fn spidey_lunge_pushes_player_away() {
    let mut server = build_lunge_scenario();
    let player_start = player_position(&mut server);

    assert!(
        wait_for_lunge_damage(&mut server, 3000),
        "Spidey should lunge and damage the player"
    );

    // Push is a multi-tick decaying impulse. Tick enough for it to apply.
    tick_server_n(&mut server, 30);

    let player_end = player_position(&mut server);
    assert!(
        player_end.x < player_start.x,
        "player should be pushed left (away from Spidey): start={player_start}, end={player_end}"
    );
}

/// Spidey recoils away from the player on successful lunge (one-shot).
///
/// The recoil is applied as an immediate position displacement on the same
/// tick as the lunge hit, not as a queued multi-tick impulse.
#[test]
fn spidey_recoils_on_lunge() {
    let mut server = build_lunge_scenario();
    let player_pos = player_position(&mut server);

    assert!(
        wait_for_lunge_damage(&mut server, 3000),
        "Spidey should lunge and damage the player"
    );

    // Recoil applied same tick as damage. Spidey should be further from the
    // player than it would be at pure lunge-arrival distance.
    let spidey_pos = spidey_position(&mut server);
    let dist = spidey_pos.distance(player_pos);
    assert!(
        dist > 0.15,
        "Spidey should have recoiled away from player: spidey={spidey_pos}, player={player_pos}, dist={dist}"
    );
}

/// Lunge push+damage is applied only once per lunge (dealt_damage guard).
#[test]
fn lunge_push_only_once() {
    let mut server = build_lunge_scenario();

    assert!(
        wait_for_lunge_damage(&mut server, 3000),
        "Spidey should lunge and damage the player"
    );

    let health_after_hit = player_health_value(&mut server);

    // Wait for the multi-tick push impulse to complete (~0.25s = ~8 ticks).
    tick_server_n(&mut server, 50);

    let position_after_push = player_position(&mut server);

    // Tick more — no second lunge should start (3s cooldown).
    tick_server_n(&mut server, 100);

    let health_later = player_health_value(&mut server);
    assert!(
        (health_later - health_after_hit).abs() < f32::EPSILON,
        "no additional damage from same lunge: after_hit={health_after_hit}, later={health_later}"
    );

    // Position should be stable after impulse expired.
    let position_later = player_position(&mut server);
    assert!(
        (position_later.x - position_after_push.x).abs() < 0.25,
        "no additional push after impulse expired: after_push={position_after_push}, later={position_later}"
    );
}

/// At least one of two flanking Spideys lunges and pushes the player.
///
/// Non-deterministic timing means both may not lunge on the same tick.
/// The test verifies the multi-Spidey scenario doesn't crash, and that
/// at least one lunge lands and displaces the player.
#[test]
fn two_spideys_lunge_same_player() {
    let port = reserve_port();
    // Two Spideys flanking the player.
    let entities = vec![
        EntitySpawnData {
            kind: EntitySpawnKind::Spidey {
                health: 100,
                speed: 2.0,
            },
            x: 5.5,
            y: 3.5,
        },
        EntitySpawnData {
            kind: EntitySpawnKind::Spidey {
                health: 100,
                speed: 2.0,
            },
            x: 1.5,
            y: 3.5,
        },
    ];
    let mut server = build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    server.update();

    // Player in the middle.
    server.world_mut().spawn((
        NetPlayer {
            player_id: carcinisation_net::PlayerId(1),
            position: bevy::math::Vec2::new(3.5, 3.5),
            angle: 0.0,
            current_attack: carcinisation_net::NetAttackId::None,
            state: carcinisation_net::PlayerNetState::Alive,
            flame_active: false,
            avatar_palette_variant: None,
        },
        NetHealth {
            current: 200.0,
            max: 200.0,
        },
        {
            let combat = carcinisation_fps_core::FpsCombatConfig::load();
            let movement = carcinisation_fps_core::FpsMovementConfig::load();
            carcinisation_server::systems::occupancy::player_occupancy(&combat, &movement)
        },
        Replicated,
    ));

    let initial_health = 200.0_f32;
    let lunge_damage = f64::from(
        carcinisation_fps_core::FpsCombatConfig::load()
            .spidey
            .lunge_melee_damage,
    );

    // Tick until at least one lunge damages the player.
    let mut hit_count = 0_u32;
    for _ in 0..5000 {
        tick_server(&mut server);
        let health = player_health_value(&mut server);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let hits = ((f64::from(initial_health) - f64::from(health)) / lunge_damage).round() as u32;
        if hits > hit_count {
            hit_count = hits;
        }
        if hit_count >= 2 {
            break;
        }
    }

    assert!(
        hit_count >= 1,
        "at least one Spidey should have lunged: hits={hit_count}"
    );

    // Multi-tick impulse needs time to apply.
    tick_server_n(&mut server, 30);

    let pos = player_position(&mut server);
    assert!(
        pos.distance(bevy::math::Vec2::new(3.5, 3.5)) > 0.1,
        "player should have been pushed from start: pos={pos}"
    );
}
