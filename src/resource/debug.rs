use crate::resource::CAMERA_BASE_SPEED;
use crate::stage::data::{
    ContainerSpawn, DestructibleSpawn, EnemySpawn, MovementDirection, ObjectSpawn, ObjectType,
    PickupSpawn, SkyboxData, StageData, StageStep,
};
use crate::stage::data::{
    DestructibleType, EnemyStep, EnemyType, PickupType, StageActionResumeCondition, StageSpawn,
};
use bevy::prelude::*;

use lazy_static::lazy_static;

const OBJECT_FIBERTREE_Y: f32 = 13.;

lazy_static! {
    pub static ref STAGE_DEBUG_DATA: StageData = StageData {
        name: "Debug".to_string(),
        music_path: "audio/music/stage_1.ogg".to_string(),
        background_path: "backgrounds/rugpark/background.png".to_string(),
        skybox: SkyboxData {
            path: "backgrounds/rugpark/skybox.png".to_string(),
            frames: 2,
        },
        start_coordinates: Some(Vec2::new(0.0, 0.0)),
        spawns: make_spawns(),
        steps: make_steps(),
    };
}

pub fn make_spawns() -> Vec<StageSpawn> {
    vec![
        StageSpawn::Object(ObjectSpawn {
            object_type: ObjectType::Fibertree,
            coordinates: Vec2 {
                x: 30.,
                y: OBJECT_FIBERTREE_Y,
            },
        }),
        StageSpawn::Object(ObjectSpawn {
            object_type: ObjectType::Fibertree,
            coordinates: Vec2 {
                x: 180.,
                y: OBJECT_FIBERTREE_Y,
            },
        }),
        StageSpawn::Object(ObjectSpawn {
            object_type: ObjectType::BenchBig,
            coordinates: Vec2 { x: 20., y: 65. },
        }),
        StageSpawn::Object(ObjectSpawn {
            object_type: ObjectType::BenchBig,
            coordinates: Vec2 { x: 200., y: 60. },
        }),
        StageSpawn::Destructible(DestructibleSpawn {
            coordinates: Vec2 { x: 30., y: 0. },
            contains: Some(Box::new(ContainerSpawn::Enemy(EnemySpawn {
                enemy_type: EnemyType::Mosquito,
                coordinates: Vec2 { x: 60., y: 100. },
                base_speed: 5.0,
                elapsed: 0.4 / CAMERA_BASE_SPEED,
                steps: vec![
                    EnemyStep::Idle { duration: 1. },
                    EnemyStep::Attack { duration: 1. },
                ],
                contains: None,
            }))),
            destructible_type: DestructibleType::Lamp,
        }),
        StageSpawn::Destructible(DestructibleSpawn {
            coordinates: Vec2 { x: 20.0, y: 0.0 },
            contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                pickup_type: PickupType::BigHealthpack,
                coordinates: Vec2 { x: 30.0, y: 10.0 },
                elapsed: 0.0 / CAMERA_BASE_SPEED,
            }))),
            destructible_type: DestructibleType::Lamp,
        }),
        StageSpawn::Pickup(PickupSpawn {
            pickup_type: PickupType::BigHealthpack,
            coordinates: Vec2 { x: 30.0, y: 10.0 },
            elapsed: 0.0 / CAMERA_BASE_SPEED,
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
            spawns: vec![] 
        },
        StageStep::Movement {
            coordinates: Vec2 { x: 50.0, y: 0.0 },
            base_speed: CAMERA_BASE_SPEED,
            spawns: vec![
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Tardigrade,
                    coordinates: Vec2 { x: 60.0, y: 100.0 },
                    base_speed: 5.0,
                    elapsed: 5.4 / CAMERA_BASE_SPEED,

                    steps: vec![
                        EnemyStep::Circle {
                            duration: 4.0,
                            radius: 10.0,
                            direction: MovementDirection::Left,
                        },
                        EnemyStep::Movement {
                            coordinates: Vec2 { x: 50.0, y: 0.0 },
                            attacking: true,
                            speed: 5.0,
                        },
                        EnemyStep::Idle { duration: 1.0 },
                        EnemyStep::Attack { duration: 1.0 },
                        EnemyStep::Movement {
                            coordinates: Vec2 { x: 10.0, y: 0.0 },
                            attacking: true,
                            speed: 3.0,
                        },
                    ],
                    contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                        pickup_type: PickupType::BigHealthpack,
                        ..default()
                    }))),
                }),
                StageSpawn::Enemy(EnemySpawn {
                    elapsed: 5.1 / CAMERA_BASE_SPEED,
                    contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                        pickup_type: PickupType::BigHealthpack,
                        ..default()
                    }))),
                    ..EnemySpawn::base_mosquito(CAMERA_BASE_SPEED, Vec2 { x: 120.0, y: 100.0 })
                }),
                StageSpawn::Enemy(EnemySpawn {
                    coordinates: Vec2 { x: 130.0, y: 70.0 },
                    elapsed: 5.8 / CAMERA_BASE_SPEED,
                    contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                        pickup_type: PickupType::BigHealthpack,
                        ..default()
                    }))),
                    ..EnemySpawn::base_mosquito(CAMERA_BASE_SPEED, Vec2 { x: 130.0, y: 70.0 })
                }),
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![StageActionResumeCondition::KillAll]),
            max_duration: Some(30. / CAMERA_BASE_SPEED),
            spawns: vec![StageSpawn::Enemy(EnemySpawn {
                coordinates: Vec2 { x: 70.0, y: 70.0 },
                elapsed: 2.4 / CAMERA_BASE_SPEED,
                contains: None,
                ..EnemySpawn::base_mosquito(CAMERA_BASE_SPEED, Vec2 { x: 70.0, y: 70.0 })
            })],
        },
    ]
}
