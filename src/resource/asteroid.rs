use crate::plugins::movement::structs::MovementDirection;
use crate::resource::CAMERA_BASE_SPEED;
use crate::stage::data::{
    ContainerSpawn, DestructibleSpawn, EnemySpawn, PickupSpawn, SkyboxData, StageData, StageStep,
};
use crate::stage::data::{
    DestructibleType, EnemyStep, EnemyType, PickupType, StageActionResumeCondition, StageSpawn,
};
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
                    .set_coordinates(Vec2::new(60., 100.))
                    .set_elapsed(0.4)
                    .set_steps_vec(vec![
                        EnemyStep::Idle { duration: 1. },
                        EnemyStep::Attack { duration: 1. },
                        EnemyStep::LinearMovement {
                            coordinates: Vec2::new(10., 0.),
                            attacking: true,
                            speed: 3.,
                        },
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
        StageStep::Movement {
            coordinates: Vec2::new(0.0, 0.0),
            base_speed: 8.0,
            spawns: vec![].into(),
        },
        StageStep::Stop {
            resume_conditions: Some(vec![].into()),
            max_duration: Some(3.0),
            spawns: vec![].into(),
        },
        StageStep::Movement {
            coordinates: Vec2::new(50.0, 0.0),
            base_speed: 10.0,
            spawns: vec![
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
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![StageActionResumeCondition::KillAll]),
            max_duration: Some(3.0),
            spawns: vec![StageSpawn::Enemy(EnemySpawn::tardigrade_base())].into(),
        },
        StageStep::Movement {
            coordinates: Vec2::new(34.0, 62.0),
            base_speed: 8.0,
            spawns: vec![].into(),
        },
        StageStep::Movement {
            coordinates: Vec2::new(90.0, 0.0),
            base_speed: 4.0,
            spawns: vec![].into(),
        },
        StageStep::Movement {
            coordinates: Vec2::new(0.0, 0.0),
            base_speed: 2.0,
            spawns: vec![].into(),
        },
        StageStep::Movement {
            coordinates: Vec2::new(50.0, 0.0),
            base_speed: CAMERA_BASE_SPEED,
            spawns: vec![
                StageSpawn::Enemy(
                    EnemySpawn::tardigrade_base().set_coordinates(Vec2::new(60.0, 100.0)),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::tardigrade_base().set_coordinates(Vec2::new(120.0, 100.0)),
                ),
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![StageActionResumeCondition::KillAll]),
            max_duration: Some(30.),
            spawns: vec![StageSpawn::Enemy(
                EnemySpawn::tardigrade_base()
                    .set_coordinates(Vec2::new(70.0, 70.0))
                    .set_elapsed(4.),
            )],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![StageActionResumeCondition::KillAll]),
            max_duration: Some(30.),
            spawns: vec![StageSpawn::Enemy(
                EnemySpawn::tardigrade_base()
                    .set_coordinates(Vec2::new(60.0, 60.0))
                    .set_speed(CAMERA_BASE_SPEED)
                    .set_elapsed(2.4),
            )],
        },
        //TODO
        StageStep::Stop {
            resume_conditions: Some(vec![
                StageActionResumeCondition::KillAll,
                StageActionResumeCondition::KillBoss,
            ]),
            max_duration: None,
            spawns: vec![].into(),
        },
    ]
}
