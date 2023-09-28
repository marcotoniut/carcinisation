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
        StageSpawn::Destructible(DestructibleSpawn {
            coordinates: Vec2 { x: 30., y: 0. },
            contains: Some(Box::new(ContainerSpawn::Enemy(EnemySpawn {
                enemy_type: EnemyType::Tardigrade,
                coordinates: Vec2 { x: 60., y: 100. },
                base_speed: 5.0,
                elapsed: 0.4 / CAMERA_BASE_SPEED,
                steps: vec![
                    EnemyStep::Movement {
                        coordinates: Vec2 { x: 50., y: 0. },
                        attacking: true,
                        speed: 5.0,
                    },
                    EnemyStep::Idle { duration: 1. },
                    EnemyStep::Attack { duration: 1. },
                    EnemyStep::Movement {
                        coordinates: Vec2 { x: 10., y: 0. },
                        attacking: true,
                        speed: 3.,
                    },
                    EnemyStep::Circle {
                        duration: 4.,
                        radius: 12.,
                        direction: MovementDirection::Negative,
                    },
                ],
                contains: None,
            }))),
            destructible_type: DestructibleType::Crystal,
        }),
        StageSpawn::Destructible(DestructibleSpawn {
            coordinates: Vec2 { x: 30., y: 0. },
            contains: Some(Box::new(ContainerSpawn::Enemy(EnemySpawn {
                enemy_type: EnemyType::Mosquito,
                coordinates: Vec2 { x: 60., y: 100. },
                base_speed: 5.0,
                elapsed: 0.4 / CAMERA_BASE_SPEED,
                steps: vec![
                    EnemyStep::Movement {
                        coordinates: Vec2 { x: 50., y: 0. },
                        attacking: true,
                        speed: 5.0,
                    },
                    EnemyStep::Idle { duration: 1. },
                    EnemyStep::Attack { duration: 1. },
                    EnemyStep::Movement {
                        coordinates: Vec2 { x: 10., y: 0. },
                        attacking: true,
                        speed: 3.,
                    },
                    EnemyStep::Circle {
                        duration: 4.,
                        radius: 12.,
                        direction: MovementDirection::Positive,
                    },
                ],
                contains: None,
            }))),
            destructible_type: DestructibleType::Mushroom,
        }),
        StageSpawn::Destructible(DestructibleSpawn {
            coordinates: Vec2 { x: 20.0, y: 0.0 },
            contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                pickup_type: PickupType::BigHealthpack,
                coordinates: Vec2 { x: 30.0, y: 10.0 },
                elapsed: 0.0 / CAMERA_BASE_SPEED,
            }))),
            destructible_type: DestructibleType::Mushroom,
        }),
        StageSpawn::Destructible(DestructibleSpawn {
            coordinates: Vec2 { x: 20.0, y: 0.0 },
            contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                pickup_type: PickupType::BigHealthpack,
                coordinates: Vec2 { x: 30.0, y: 10.0 },
                elapsed: 0.0 / CAMERA_BASE_SPEED,
            }))),
            destructible_type: DestructibleType::Crystal,
        }),
    ]
}

pub fn make_steps() -> Vec<StageStep> {
    vec![
        StageStep::Movement {
            coordinates: Vec2 { x: 0.0, y: 0.0 },
            base_speed: 8.0,
            spawns: vec![],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![]),
            max_duration: Some(3.0),
            spawns: vec![],
        },
        StageStep::Movement {
            coordinates: Vec2 { x: 50.0, y: 0.0 },
            base_speed: 10.0,
            spawns: vec![
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 60.0, y: 100.0 },
                    base_speed: 5.0,
                    elapsed: 1.4,
                    steps: vec![EnemyStep::Circle {
                        duration: 4.0,
                        radius: 10.0,
                        direction: MovementDirection::Negative,
                    }],
                    contains: None,
                }),
                StageSpawn::Enemy(EnemySpawn {
                    contains: None,
                    elapsed: 4.2,
                    ..EnemySpawn::base_tardigrade(CAMERA_BASE_SPEED, Vec2 { x: 120.0, y: 100.0 })
                }),
                StageSpawn::Enemy(EnemySpawn {
                    contains: None,
                    elapsed: 2.3,
                    ..EnemySpawn::base_tardigrade(CAMERA_BASE_SPEED, Vec2 { x: 100.0, y: 70.0 })
                }),
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![StageActionResumeCondition::KillAll]),
            max_duration: Some(3.0),
            spawns: vec![StageSpawn::base_mosquito(
                CAMERA_BASE_SPEED * 1.2,
                Vec2 { x: 20.0, y: 0.0 },
            )],
        },
        StageStep::Movement {
            coordinates: Vec2 { x: 34.0, y: 62.0 },
            base_speed: 8.0,
            spawns: vec![],
        },
        StageStep::Movement {
            coordinates: Vec2 { x: 90.0, y: 0.0 },
            base_speed: 4.0,
            spawns: vec![],
        },
        StageStep::Movement {
            coordinates: Vec2 { x: 0.0, y: 0.0 },
            base_speed: 2.0,
            spawns: vec![],
        },
        StageStep::Movement {
            coordinates: Vec2 { x: 50.0, y: 0.0 },
            base_speed: CAMERA_BASE_SPEED,
            spawns: vec![
                StageSpawn::base_tardigrade(CAMERA_BASE_SPEED, Vec2 { x: 60.0, y: 100.0 }),
                StageSpawn::base_mosquito(CAMERA_BASE_SPEED, Vec2 { x: 120.0, y: 100.0 }),
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![StageActionResumeCondition::KillAll]),
            max_duration: Some(30. / CAMERA_BASE_SPEED),
            spawns: vec![StageSpawn::Enemy(EnemySpawn {
                elapsed: 4.5 / CAMERA_BASE_SPEED,
                steps: vec![],
                contains: None,
                ..EnemySpawn::base_mosquito(CAMERA_BASE_SPEED, Vec2 { x: 70.0, y: 70.0 })
            })],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![StageActionResumeCondition::KillAll]),
            max_duration: Some(30. / CAMERA_BASE_SPEED),
            spawns: vec![StageSpawn::Enemy(EnemySpawn {
                enemy_type: EnemyType::Tardigrade,
                coordinates: Vec2 { x: 60.0, y: 60.0 },
                base_speed: CAMERA_BASE_SPEED,
                elapsed: 2.4 / CAMERA_BASE_SPEED,
                steps: vec![],
                contains: None,
            })],
        },
        //TODO
        StageStep::Stop {
            resume_conditions: Some(vec![
                StageActionResumeCondition::KillAll,
                StageActionResumeCondition::KillBoss,
            ]),
            max_duration: None,
            spawns: vec![],
        },
    ]
}
