use crate::plugins::movement::structs::MovementDirection;
use crate::stage::data::*;
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
        StageSpawn::Destructible(DestructibleSpawn::crystal_base(30., 0.)),
        StageSpawn::Destructible(
            DestructibleSpawn::mushroom_base(60., 0.).drops(ContainerSpawn::Enemy(
                EnemySpawn::tardigrade_base()
                    .set_elapsed(0.4)
                    .set_steps_vec(vec![
                        EnemyStep::Idle { duration: 1. },
                        EnemyStep::Attack { duration: 1. },
                        EnemyStep::linear_movement_base().set_linear_direction(Vec2::new(0.5, -1.)),
                        EnemyStep::Circle {
                            duration: 4.,
                            radius: 12.,
                            direction: MovementDirection::Positive,
                        },
                    ]),
            )),
        ),
        StageSpawn::Destructible(
            DestructibleSpawn::mushroom_base(20.0, 0.0)
                .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
        ),
        StageSpawn::Destructible(
            DestructibleSpawn::crystal_base(20., 0.)
                .drops(ContainerSpawn::Pickup(PickupSpawn::small_healthpack_base())),
        ),
    ]
}

pub fn make_steps() -> Vec<StageStep> {
    vec![
        StageStep::movement_base(0.0, 0.0).set_base_speed(8.0),
        StageStep::Stop(StageStepStop::new().set_max_duration(10.)),
        StageStep::movement_base(50.0, 0.0)
            .set_base_speed(10.0)
            .add_spawns(vec![
                StageSpawn::Enemy(
                    EnemySpawn::tardigrade_base()
                        .set_coordinates(Vec2::new(60.0, 100.0))
                        .set_elapsed(1.4)
                        .set_steps_vec(vec![EnemyStep::Circle {
                            duration: 4.0,
                            radius: 10.0,
                            direction: MovementDirection::Negative,
                        }]),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::tardigrade_base()
                        .set_coordinates(Vec2::new(120.0, 100.0))
                        .set_elapsed(4.2),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::tardigrade_base()
                        .set_coordinates(Vec2::new(100.0, 70.0))
                        .set_elapsed(2.4),
                ),
            ]),
        StageStep::movement_base(34.0, 62.0).set_base_speed(8.0),
        StageStep::movement_base(90.0, 0.0).set_base_speed(4.0),
        StageStep::movement_base(0.0, 0.0).set_base_speed(2.0),
        StageStep::movement_base(50.0, 0.0).add_spawns(vec![
            StageSpawn::Enemy(
                EnemySpawn::tardigrade_base().set_coordinates(Vec2::new(60.0, 100.0)),
            ),
            StageSpawn::Enemy(
                EnemySpawn::tardigrade_base().set_coordinates(Vec2::new(120.0, 100.0)),
            ),
        ]),
        StageStep::Stop(StageStepStop::new().set_max_duration(30.).add_spawns(
            vec![StageSpawn::Enemy(
                EnemySpawn::tardigrade_base()
                    .set_coordinates(Vec2::new(70.0, 70.0))
                    .set_elapsed(4.),
            )],
        )),
        StageStep::Stop(StageStepStop::new().set_max_duration(40.).add_spawns(
            vec![StageSpawn::Enemy(
                EnemySpawn::tardigrade_base()
                    .set_coordinates(Vec2::new(70.0, 70.0))
                    .set_elapsed(4.),
            )],
        )),
        StageStep::Stop(StageStepStop::new().set_kill_all(false).set_kill_boss(true)),
    ]
}
