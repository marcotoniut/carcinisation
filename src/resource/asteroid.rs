use crate::stage::components::{MovementStageStep, StopStageStep};
use crate::stage::data::*;
use crate::stage::destructible::data::DestructibleSpawn;
use crate::stage::enemy::data::steps::EnemyStep;
use bevy::prelude::*;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref STAGE_ASTEROID_DATA: StageData = StageData {
        name: "Asteroid".to_string(),
        background_path: "backgrounds/asteroid/background.png".to_string(),
        music_path: "audio/music/stage_3.ogg".to_string(),
        skybox: SkyboxData {
            path: "backgrounds/asteroid/skybox.png".to_string(),
            frames: 1,
        },
        start_coordinates: Some(Vec2::new(0.0, 0.0)),
        spawns: make_spawns(),
        steps: make_steps(),
    };
}

pub fn make_spawns() -> Vec<StageSpawn> {
    vec![
        DestructibleSpawn::crystal_base(30., 0.).into(),
        DestructibleSpawn::mushroom_base(60., 0.)
            .drops(ContainerSpawn::Enemy(
                EnemySpawn::tardigrade_base()
                    .with_elapsed(0.4)
                    .with_steps_vec(vec![
                        EnemyStep::idle_base().with_duration(1.).into(),
                        EnemyStep::attack_base().with_duration(3.).into(),
                        EnemyStep::linear_movement_base()
                            .with_direction(Vec2::new(0.5, -1.))
                            .into(),
                        EnemyStep::circle_around_base()
                            .opposite_direction()
                            .with_radius(4.)
                            .into(),
                    ]),
            ))
            .into(),
        DestructibleSpawn::mushroom_base(20.0, 0.0)
            .drops(PickupSpawn::big_healthpack_base().into())
            .into(),
        DestructibleSpawn::crystal_base(20., 0.)
            .drops(PickupSpawn::small_healthpack_base().into())
            .into(),
    ]
}

pub fn make_steps() -> Vec<StageStep> {
    vec![
        MovementStageStep::base(0.0, 0.0)
            .with_base_speed(8.0)
            .into(),
        StopStageStep::new().with_max_duration(10.).into(),
        MovementStageStep::base(50.0, 0.0)
            .with_base_speed(10.0)
            .add_spawns(vec![
                EnemySpawn::tardigrade_base()
                    .with_coordinates(Vec2::new(60.0, 100.0))
                    .with_elapsed(1.4)
                    .with_steps_vec(vec![EnemyStep::circle_around_base()
                        .with_radius(10.)
                        .with_duration(4.)
                        .into()])
                    .into(),
                EnemySpawn::tardigrade_base()
                    .with_coordinates(Vec2::new(120.0, 100.0))
                    .with_elapsed(4.2)
                    .into(),
                EnemySpawn::tardigrade_base()
                    .with_coordinates(Vec2::new(100.0, 70.0))
                    .with_elapsed(2.4)
                    .into(),
            ])
            .into(),
        MovementStageStep::base(34.0, 62.0)
            .with_base_speed(8.0)
            .into(),
        MovementStageStep::base(90.0, 0.0)
            .with_base_speed(4.0)
            .into(),
        MovementStageStep::base(0.0, 0.0)
            .with_base_speed(2.0)
            .into(),
        MovementStageStep::base(50.0, 0.0)
            .add_spawns(vec![
                EnemySpawn::tardigrade_base()
                    .with_coordinates(Vec2::new(60.0, 100.0))
                    .into(),
                EnemySpawn::tardigrade_base()
                    .with_coordinates(Vec2::new(120.0, 100.0))
                    .into(),
            ])
            .into(),
        StopStageStep::new()
            .with_max_duration(30.)
            .add_spawns(vec![EnemySpawn::tardigrade_base()
                .with_coordinates(Vec2::new(70.0, 70.0))
                .with_elapsed(4.)
                .into()])
            .into(),
        StopStageStep::new()
            .with_max_duration(40.)
            .add_spawns(vec![EnemySpawn::tardigrade_base()
                .with_coordinates(Vec2::new(70.0, 70.0))
                .with_elapsed(4.)
                .into()])
            .into(),
        StopStageStep::new()
            .with_kill_all(false)
            .with_kill_boss(true)
            .into(),
    ]
}
