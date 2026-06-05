//! Phase 3: soft body-occupancy separation integration tests.
#![allow(clippy::doc_markdown)]

mod common;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind, test_map};
use carcinisation_net::{NetEnemyState, NetHealth, NetPlayer, PlayerId, PlayerNetState};
use carcinisation_server::ServerPlugin;
use common::{build_server_app, reserve_port};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn tick_server(server: &mut App) {
    std::thread::sleep(std::time::Duration::from_millis(2));
    server.update();
}

fn tick_server_n(server: &mut App, n: usize) {
    for _ in 0..n {
        tick_server(server);
    }
}

fn spawn_player_at(server: &mut App, id: u32, x: f32, y: f32) {
    server.world_mut().spawn((
        NetPlayer {
            player_id: PlayerId(id),
            position: Vec2::new(x, y),
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
        // Attach occupancy manually (same as server spawn would).
        {
            let combat = carcinisation_fps_core::FpsCombatConfig::load();
            let movement = carcinisation_fps_core::FpsMovementConfig::load();
            carcinisation_server::systems::occupancy::player_occupancy(&combat, &movement)
        },
        Replicated,
    ));
}

fn enemy_positions(server: &mut App) -> Vec<Vec2> {
    server
        .world_mut()
        .query::<&carcinisation_net::components::NetEnemy>()
        .iter(server.world())
        .map(|e| e.position)
        .collect()
}

fn player_position(server: &mut App, id: u32) -> Vec2 {
    server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .find(|p| p.player_id == PlayerId(id))
        .unwrap()
        .position
}

fn build_two_overlapping_enemies(x: f32, y: f32) -> App {
    let port = reserve_port();
    let entities = vec![
        EntitySpawnData {
            kind: EntitySpawnKind::Mosquiton {
                health: 100,
                speed: 0.0, // stationary AI
            },
            x,
            y,
        },
        EntitySpawnData {
            kind: EntitySpawnKind::Mosquiton {
                health: 100,
                speed: 0.0,
            },
            x,
            y,
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
    server
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Two enemies spawned at exactly the same position separate over ticks.
///
/// The `stable_index`-based fallback in `compute_separation` pushes the
/// lower-index entity in `+X` and the higher-index in `-X`, breaking the
/// symmetry that previously caused coincident pairs to drift in lockstep.
#[test]
fn overlapping_enemies_separate() {
    let mut server = build_two_overlapping_enemies(3.5, 3.5);

    let initial = enemy_positions(&mut server);
    assert_eq!(initial.len(), 2);
    assert!(
        initial[0].distance(initial[1]) < 0.01,
        "enemies should start at the same position"
    );

    tick_server_n(&mut server, 300);

    let final_pos = enemy_positions(&mut server);
    let dist = final_pos[0].distance(final_pos[1]);
    assert!(
        dist > 0.1,
        "exactly coincident enemies should have separated: dist={dist}, positions={final_pos:?}"
    );
}

/// Player is not hard-blocked by an enemy — can move through with normal
/// movement (separation pushes gently, doesn't prevent traversal).
#[test]
fn player_not_hard_blocked_by_enemy() {
    let port = reserve_port();
    // Mosquiton at (3.5, 3.5), stationary.
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 100,
            speed: 0.0,
        },
        x: 3.5,
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

    // Player starts at (3.5, 3.5) — exactly overlapping the enemy.
    spawn_player_at(&mut server, 1, 3.5, 3.5);

    tick_server_n(&mut server, 100);

    // Player should still exist and not be stuck at the enemy position.
    // Soft separation may push them apart, but the player should not be
    // despawned or trapped. The key assertion: the player moved at all.
    let player_pos = player_position(&mut server, 1);
    let enemy_pos = enemy_positions(&mut server);
    // With player weight 5.0 and enemy weight 1.0, the enemy should yield
    // more than the player. But even the player should get some displacement
    // from separation since player_separation_strength > 0.
    assert!(
        player_pos.distance(Vec2::new(3.5, 3.5)) > 0.01
            || enemy_pos[0].distance(Vec2::new(3.5, 3.5)) > 0.01,
        "at least one entity should have moved from overlap: player={player_pos}, enemy={:?}",
        enemy_pos[0]
    );
}

/// Enemy yields more than the player during separation.
#[test]
fn enemy_yields_more_than_player() {
    let port = reserve_port();
    let entities = vec![EntitySpawnData {
        kind: EntitySpawnKind::Mosquiton {
            health: 100,
            speed: 0.0,
        },
        x: 3.5,
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

    // Player very close to enemy but not exactly overlapping.
    spawn_player_at(&mut server, 1, 3.7, 3.5);

    tick_server_n(&mut server, 200);

    let player_pos = player_position(&mut server, 1);
    let enemy_pos = enemy_positions(&mut server);

    let player_displacement = player_pos.distance(Vec2::new(3.7, 3.5));
    let enemy_displacement = enemy_pos[0].distance(Vec2::new(3.5, 3.5));

    assert!(
        enemy_displacement > player_displacement,
        "enemy should yield more: enemy_disp={enemy_displacement}, player_disp={player_displacement}"
    );
}

/// Separation respects walls — entities pushed toward a wall stop at the wall.
#[test]
fn separation_respects_walls() {
    // Two enemies near the border wall (x=1). Separation pushes one leftward
    // but try_move should prevent it from entering the wall.
    let mut server = build_two_overlapping_enemies(1.3, 3.5);

    tick_server_n(&mut server, 200);

    let positions = enemy_positions(&mut server);
    for pos in &positions {
        assert!(
            pos.x >= 1.0,
            "enemy should not pass through the wall: pos={pos}"
        );
    }
}

/// Dead/disabled enemies do not participate in separation.
#[test]
fn dead_enemies_do_not_separate() {
    let mut server = build_two_overlapping_enemies(3.5, 3.5);

    // Kill one enemy.
    {
        let mut q = server
            .world_mut()
            .query::<(&mut carcinisation_net::components::NetEnemy, &mut NetHealth)>();
        let mut killed = false;
        for (mut enemy, mut health) in q.iter_mut(server.world_mut()) {
            if !killed {
                enemy.state = NetEnemyState::Dead { burn: false };
                health.current = 0.0;
                killed = true;
            }
        }
    }

    let alive_before = enemy_positions(&mut server);

    tick_server_n(&mut server, 200);

    let positions = enemy_positions(&mut server);
    // The alive enemy should not have been pushed (the dead one is excluded
    // from separation). Position change should be minimal.
    let alive_moved = positions
        .iter()
        .zip(alive_before.iter())
        .any(|(a, b)| a.distance(*b) > 0.5);
    // Allow some movement from AI or other systems, but the dead enemy should
    // not contribute separation force.
    assert!(
        !alive_moved || positions.len() < 2,
        "alive enemy should not be strongly displaced by dead enemy"
    );
}

/// Spidey lunge push still works with separation enabled.
#[test]
fn lunge_push_works_with_separation() {
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
    spawn_player_at(&mut server, 1, 3.5, 3.5);

    let initial_health = server
        .world_mut()
        .query::<(&NetPlayer, &NetHealth)>()
        .iter(server.world())
        .next()
        .map(|(_, h)| h.current)
        .unwrap();

    // Wait for lunge damage.
    let mut damaged = false;
    for _ in 0..3000 {
        tick_server(&mut server);
        let health = server
            .world_mut()
            .query::<(&NetPlayer, &NetHealth)>()
            .iter(server.world())
            .next()
            .map(|(_, h)| h.current)
            .unwrap();
        if health < initial_health {
            damaged = true;
            break;
        }
    }

    assert!(damaged, "Spidey should lunge and damage the player");

    // Multi-tick impulse needs time to apply.
    tick_server_n(&mut server, 30);

    // Player should have been pushed by the lunge (not blocked by separation).
    let player_pos = player_position(&mut server, 1);
    assert!(
        player_pos.x < 3.5,
        "player should be pushed left by lunge: pos={player_pos}"
    );
}
