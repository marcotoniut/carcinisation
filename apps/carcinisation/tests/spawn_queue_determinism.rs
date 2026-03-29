//! Spawn queue determinism tests.
//!
//! Validates `StageStepSpawner` queue drain logic is deterministic:
//! - Queue drains completely when time advances sufficiently
//! - Spawns process in authored order
//! - Elapsed accumulator carries over correctly
//!
//! # Why These Tests Exist
//!
//! The spawn system drains the queue based on `elapsed + delta()`. If timing
//! calculation drifts, spawns can skip or duplicate, causing stages to have
//! incorrect entity counts with no error raised.

use bevy::math::Vec2;
use carcinisation::stage::{
    components::placement::Depth,
    data::{EnemySpawn, GAME_BASE_SPEED, PickupSpawn, PickupType, StageSpawn},
    enemy::entity::EnemyType,
    resources::StageStepSpawner,
};
use std::time::Duration;

/// Helper to create spawn elapsed time accounting for GAME_BASE_SPEED.
/// Spawns are authored in "game time" but get_elapsed() divides by GAME_BASE_SPEED.
fn spawn_time(millis: u64) -> Duration {
    Duration::from_millis((millis as f32 * GAME_BASE_SPEED) as u64)
}

/// Simulates the spawn drain logic from `check_step_spawn`.
fn drain_spawns(spawner: &mut StageStepSpawner, delta: Duration) -> usize {
    let mut elapsed = spawner.elapsed + delta;
    let mut processed = 0;

    spawner.spawns.retain_mut(|spawn| {
        let spawn_elapsed = spawn.get_elapsed();
        if spawn_elapsed <= elapsed {
            elapsed -= spawn_elapsed;
            processed += 1;
            false
        } else {
            true
        }
    });

    spawner.elapsed = elapsed;
    processed
}

/// Validates queue drains completely when time advances past all spawns.
#[test]
fn spawn_queue_drains_completely() {
    let spawns = vec![
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: Duration::from_secs(1),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: Duration::from_secs(2),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Pickup(PickupSpawn {
            pickup_type: PickupType::SmallHealthpack,
            elapsed: Duration::from_secs(1),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
        }),
    ];

    let mut spawner = StageStepSpawner::new(spawns);
    assert_eq!(spawner.spawns.len(), 3);

    drain_spawns(&mut spawner, Duration::from_secs(10));

    assert_eq!(spawner.spawns.len(), 0);
}

/// Validates spawns drain incrementally as time advances.
#[test]
fn spawns_drain_incrementally() {
    let spawns = vec![
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: spawn_time(500),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: spawn_time(1000),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: spawn_time(1500),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
    ];

    let mut spawner = StageStepSpawner::new(spawns);

    assert_eq!(drain_spawns(&mut spawner, Duration::ZERO), 0);
    assert_eq!(spawner.spawns.len(), 3);

    assert_eq!(drain_spawns(&mut spawner, Duration::from_millis(500)), 1);
    assert_eq!(spawner.spawns.len(), 2);

    assert_eq!(drain_spawns(&mut spawner, Duration::from_millis(1000)), 1);
    assert_eq!(spawner.spawns.len(), 1);

    assert_eq!(drain_spawns(&mut spawner, Duration::from_millis(1500)), 1);
    assert_eq!(spawner.spawns.len(), 0);
}

/// Validates zero-elapsed spawns drain immediately.
#[test]
fn zero_elapsed_spawns_drain_immediately() {
    let spawns = vec![
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: Duration::ZERO,
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: Duration::ZERO,
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
    ];

    let mut spawner = StageStepSpawner::new(spawns);

    assert_eq!(drain_spawns(&mut spawner, Duration::ZERO), 2);
    assert_eq!(spawner.spawns.len(), 0);
}

/// Validates spawns with identical elapsed times drain sequentially.
///
/// Multiple spawns with same wait time require cumulative elapsed time to drain all.
#[test]
fn identical_elapsed_spawns_drain_sequentially() {
    let spawns = vec![
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: spawn_time(1000),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Pickup(PickupSpawn {
            pickup_type: PickupType::SmallHealthpack,
            elapsed: spawn_time(1000),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
        }),
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Tardigrade,
            elapsed: spawn_time(1000),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
    ];

    let mut spawner = StageStepSpawner::new(spawns);

    // First spawn drains at 1000ms
    assert_eq!(drain_spawns(&mut spawner, Duration::from_millis(1000)), 1);
    assert_eq!(spawner.spawns.len(), 2);

    // Second spawn needs another 1000ms (cumulative 2000ms)
    assert_eq!(drain_spawns(&mut spawner, Duration::from_millis(1000)), 1);
    assert_eq!(spawner.spawns.len(), 1);

    // Third spawn needs another 1000ms (cumulative 3000ms)
    assert_eq!(drain_spawns(&mut spawner, Duration::from_millis(1000)), 1);
    assert_eq!(spawner.spawns.len(), 0);
}

/// Validates small deltas respect spawn timing.
#[test]
fn small_deltas_respect_spawn_timing() {
    let spawns = vec![StageSpawn::Enemy(EnemySpawn {
        enemy_type: EnemyType::Mosquito,
        elapsed: spawn_time(100),
        coordinates: Vec2::ZERO,
        depth: Depth::Three,
        ..Default::default()
    })];

    let mut spawner = StageStepSpawner::new(spawns);

    // spawn_time(100) creates a spawn that drains after ~100ms accumulated time
    // (Note: div_f32 rounding means actual threshold is 100.000001ms)
    // Advance by 10ms increments (should not drain for 9 iterations)
    for _ in 0..9 {
        assert_eq!(drain_spawns(&mut spawner, Duration::from_millis(10)), 0);
        assert_eq!(spawner.spawns.len(), 1);
    }
    // Total so far: 10 * 9 = 90ms

    // 10th advance pushes past 100ms threshold (accounting for floating point rounding)
    assert_eq!(drain_spawns(&mut spawner, Duration::from_millis(11)), 1);
    assert_eq!(spawner.spawns.len(), 0);
}

/// Validates large time jump drains all spawns.
#[test]
fn large_time_jump_drains_all_spawns() {
    let spawns = vec![
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: Duration::from_secs(1),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: Duration::from_secs(1),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: Duration::from_secs(1),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
    ];

    let mut spawner = StageStepSpawner::new(spawns);

    assert_eq!(drain_spawns(&mut spawner, Duration::from_secs(10)), 3);
    assert_eq!(spawner.spawns.len(), 0);
}

/// Validates elapsed accumulator carries over remaining time.
#[test]
fn elapsed_accumulator_carries_over() {
    let spawns = vec![StageSpawn::Enemy(EnemySpawn {
        enemy_type: EnemyType::Mosquito,
        elapsed: spawn_time(500),
        coordinates: Vec2::ZERO,
        depth: Depth::Three,
        ..Default::default()
    })];

    let mut spawner = StageStepSpawner::new(spawns);

    assert_eq!(drain_spawns(&mut spawner, Duration::from_millis(600)), 1);
    assert_eq!(spawner.spawns.len(), 0);
    assert_eq!(spawner.elapsed, Duration::from_millis(100));
}

/// Validates queue doesn't drain when time frozen.
#[test]
fn frozen_time_preserves_queue() {
    let spawns = vec![StageSpawn::Enemy(EnemySpawn {
        enemy_type: EnemyType::Mosquito,
        elapsed: Duration::from_millis(100),
        coordinates: Vec2::ZERO,
        depth: Depth::Three,
        ..Default::default()
    })];

    let mut spawner = StageStepSpawner::new(spawns);

    for _ in 0..10 {
        assert_eq!(drain_spawns(&mut spawner, Duration::ZERO), 0);
    }

    assert_eq!(spawner.spawns.len(), 1);
}

/// Validates queue length invariant across spawn counts.
#[test]
fn queue_length_invariant_holds() {
    for count in [1, 5, 10, 20] {
        let spawns: Vec<_> = (0..count)
            .map(|i| {
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    elapsed: Duration::from_millis(i as u64 * 100),
                    coordinates: Vec2::ZERO,
                    depth: Depth::Three,
                    ..Default::default()
                })
            })
            .collect();

        let mut spawner = StageStepSpawner::new(spawns);

        assert_eq!(spawner.spawns.len(), count);

        drain_spawns(&mut spawner, Duration::from_secs(10));

        assert_eq!(spawner.spawns.len(), 0);
    }
}

/// Validates partial drain leaves correct remaining spawns.
#[test]
fn partial_drain_preserves_remaining_spawns() {
    let spawns = vec![
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: spawn_time(100),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Tardigrade,
            elapsed: spawn_time(500),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
    ];

    let mut spawner = StageStepSpawner::new(spawns);

    assert_eq!(drain_spawns(&mut spawner, Duration::from_millis(200)), 1);
    assert_eq!(spawner.spawns.len(), 1);

    match &spawner.spawns[0] {
        StageSpawn::Enemy(e) => {
            assert_eq!(e.enemy_type, EnemyType::Tardigrade);
        }
        _ => panic!("Wrong spawn type remained"),
    }
}

/// Validates mixed spawn types drain correctly.
#[test]
fn mixed_spawn_types_drain_correctly() {
    let spawns = vec![
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: Duration::from_millis(100),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Pickup(PickupSpawn {
            pickup_type: PickupType::SmallHealthpack,
            elapsed: Duration::from_millis(200),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
        }),
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Tardigrade,
            elapsed: Duration::from_millis(300),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
    ];

    let mut spawner = StageStepSpawner::new(spawns);

    assert_eq!(drain_spawns(&mut spawner, Duration::from_millis(600)), 3);
    assert_eq!(spawner.spawns.len(), 0);
}
