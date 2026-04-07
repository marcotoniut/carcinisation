//! Deterministic stage-spawn queue tests.

use std::{mem, time::Duration};

use bevy::prelude::*;
use carcinisation::stage::{
    check_step_spawn,
    components::placement::Depth,
    data::{EnemySpawn, GAME_BASE_SPEED, PickupSpawn, PickupType, StageSpawn},
    enemy::entity::EnemyType,
    messages::StageSpawnEvent,
    resources::{StageStepSpawner, StageTimeDomain},
};

#[derive(Resource, Default)]
struct ObservedStageSpawns(Vec<StageSpawn>);

fn build_spawn_test_app() -> App {
    let mut app = App::new();
    app.insert_resource(Time::<StageTimeDomain>::default());
    app.init_resource::<ObservedStageSpawns>();
    app.add_message::<StageSpawnEvent>();
    app.add_observer(
        |trigger: On<StageSpawnEvent>, mut observed: ResMut<ObservedStageSpawns>| {
            observed.0.push(trigger.event().spawn.clone());
        },
    );
    app.add_systems(Update, check_step_spawn);
    app
}

fn advance_stage(app: &mut App, duration: Duration) {
    app.world_mut()
        .resource_mut::<Time<StageTimeDomain>>()
        .advance_by(duration);
    app.update();
}

fn take_spawn_events(app: &mut App) -> Vec<StageSpawn> {
    mem::take(&mut app.world_mut().resource_mut::<ObservedStageSpawns>().0)
}

fn authored_elapsed(runtime_duration: Duration) -> Duration {
    Duration::from_secs_f32(runtime_duration.as_secs_f32() * GAME_BASE_SPEED)
}

fn get_spawner_queue_len(app: &mut App) -> usize {
    app.world_mut()
        .query::<&StageStepSpawner>()
        .iter(app.world())
        .next()
        .map(|spawner| spawner.spawns.len())
        .unwrap_or(0)
}

fn get_spawner_elapsed(app: &mut App) -> Duration {
    app.world_mut()
        .query::<&StageStepSpawner>()
        .iter(app.world())
        .next()
        .map(|spawner| spawner.elapsed)
        .unwrap_or(Duration::ZERO)
}

#[test]
fn spawn_queue_drains_all_spawns() {
    let mut app = build_spawn_test_app();

    let spawns = vec![
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: authored_elapsed(Duration::from_secs(1)),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: authored_elapsed(Duration::from_secs(2)),
            coordinates: Vec2::new(100., 0.),
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Pickup(PickupSpawn {
            pickup_type: PickupType::SmallHealthpack,
            elapsed: authored_elapsed(Duration::from_secs(1)),
            coordinates: Vec2::new(200., 0.),
            depth: Depth::Three,
            authored_depths: None,
        }),
    ];

    app.world_mut().spawn(StageStepSpawner::new(spawns.clone()));

    advance_stage(&mut app, Duration::from_secs(10));

    assert_eq!(get_spawner_queue_len(&mut app), 0);
    assert_eq!(take_spawn_events(&mut app).len(), spawns.len());
}

#[test]
fn spawns_trigger_at_authored_times() {
    let mut app = build_spawn_test_app();

    app.world_mut().spawn(StageStepSpawner::new(vec![
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: authored_elapsed(Duration::from_millis(500)),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: authored_elapsed(Duration::from_millis(1000)),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: authored_elapsed(Duration::from_millis(1500)),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
    ]));

    app.update();
    assert!(take_spawn_events(&mut app).is_empty());

    advance_stage(&mut app, Duration::from_millis(500));
    assert_eq!(take_spawn_events(&mut app).len(), 1);

    advance_stage(&mut app, Duration::from_millis(1000));
    assert_eq!(take_spawn_events(&mut app).len(), 1);

    advance_stage(&mut app, Duration::from_millis(1500));
    assert_eq!(take_spawn_events(&mut app).len(), 1);
    assert_eq!(get_spawner_queue_len(&mut app), 0);
}

#[test]
fn simultaneous_spawns_maintain_authored_order() {
    let mut app = build_spawn_test_app();
    let authored = vec![
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: authored_elapsed(Duration::from_secs(1)),
            coordinates: Vec2::new(0., 0.),
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Pickup(PickupSpawn {
            pickup_type: PickupType::SmallHealthpack,
            elapsed: Duration::ZERO,
            coordinates: Vec2::new(100., 0.),
            depth: Depth::Three,
            authored_depths: None,
        }),
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Tardigrade,
            elapsed: Duration::ZERO,
            coordinates: Vec2::new(200., 0.),
            depth: Depth::Three,
            ..Default::default()
        }),
    ];

    app.world_mut().spawn(StageStepSpawner::new(authored));
    advance_stage(&mut app, Duration::from_secs(1));

    let emitted = take_spawn_events(&mut app);
    assert_eq!(emitted.len(), 3);
    assert!(matches!(
        (&emitted[0], &emitted[1], &emitted[2]),
        (
            StageSpawn::Enemy(EnemySpawn {
                enemy_type: EnemyType::Mosquito,
                ..
            }),
            StageSpawn::Pickup(PickupSpawn {
                pickup_type: PickupType::SmallHealthpack,
                ..
            }),
            StageSpawn::Enemy(EnemySpawn {
                enemy_type: EnemyType::Tardigrade,
                ..
            }),
        )
    ));
}

#[test]
fn elapsed_accumulator_carries_over_correctly() {
    let mut app = build_spawn_test_app();

    app.world_mut()
        .spawn(StageStepSpawner::new(vec![StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: authored_elapsed(Duration::from_millis(500)),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        })]));

    advance_stage(&mut app, Duration::from_millis(600));

    assert_eq!(take_spawn_events(&mut app).len(), 1);
    assert_eq!(get_spawner_elapsed(&mut app), Duration::from_millis(100));
}

#[test]
fn large_time_jump_processes_every_intermediate_spawn() {
    let mut app = build_spawn_test_app();

    app.world_mut().spawn(StageStepSpawner::new(vec![
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: authored_elapsed(Duration::from_secs(1)),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: authored_elapsed(Duration::from_secs(1)),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
        StageSpawn::Enemy(EnemySpawn {
            enemy_type: EnemyType::Mosquito,
            elapsed: authored_elapsed(Duration::from_secs(1)),
            coordinates: Vec2::ZERO,
            depth: Depth::Three,
            ..Default::default()
        }),
    ]));

    advance_stage(&mut app, Duration::from_secs(10));

    assert_eq!(take_spawn_events(&mut app).len(), 3);
    assert_eq!(get_spawner_queue_len(&mut app), 0);
}
