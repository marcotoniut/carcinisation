use crate::stage::data::{
    ContainerSpawn, DestructibleSpawn, EnemySpawn, ObjectSpawn, ObjectType, PickupSpawn,
    SkyboxData, StageData, StageStep,
};
use crate::stage::data::{
    DestructibleType, EnemyStep, EnemyType, PickupType, StageActionResumeCondition, StageSpawn,
};
use bevy::prelude::*;

use lazy_static::lazy_static;

const OBJECT_FIBERTREE_Y: f32 = 10.;

lazy_static! {
    pub static ref STAGE_PARK_DATA: StageData = StageData {
        name: "Park".to_string(),
        background: "backgrounds/park/background.png".to_string(),
        skybox: SkyboxData {
            path: "backgrounds/park/skybox.png".to_string(),
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
            object_type: ObjectType::Fibertree,
            coordinates: Vec2 {
                x: 320.,
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
        StageSpawn::Object(ObjectSpawn {
            object_type: ObjectType::BenchSmall,
            coordinates: Vec2 { x: 100., y: 65. },
        }),
        StageSpawn::Destructible(DestructibleSpawn {
            coordinates: Vec2 { x: 30., y: 0. },
            contains: Some(Box::new(ContainerSpawn::Enemy(EnemySpawn {
                enemy_type: EnemyType::Mosquito,
                coordinates: Vec2 { x: 60., y: 100. },
                base_speed: 5.0,
                elapsed: 1.4,
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
                    EnemyStep::Circle { duration: 4. },
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
                elapsed: 0.0,
            }))),
            destructible_type: DestructibleType::Lamp,
        }),
        StageSpawn::Pickup(PickupSpawn {
            pickup_type: PickupType::BigHealthpack,
            coordinates: Vec2 { x: 30.0, y: 10.0 },
            elapsed: 0.0,
        }),
    ]
}

pub fn make_steps() -> Vec<StageStep> {
    vec![
        StageStep::Movement {
            coordinates: Vec2 { x: 50.0, y: 0.0 },
            base_speed: 10.0,
            spawns: vec![
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 60.0, y: 100.0 },
                    base_speed: 5.0,
                    elapsed: 1.4,
                    steps: vec![
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
                        EnemyStep::Circle { duration: 4.0 },
                    ],
                    contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                        pickup_type: PickupType::BigHealthpack,
                        ..default()
                    }))),
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 120.0, y: 100.0 },
                    base_speed: 5.0,
                    elapsed: 0.4,
                    steps: vec![],
                    contains: None,
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 130.0, y: 70.0 },
                    base_speed: 5.0,
                    elapsed: 2.3,
                    steps: vec![],
                    contains: None,
                }),
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![StageActionResumeCondition::KillAll]),
            max_duration: Some(3),
            spawns: vec![StageSpawn::Enemy(EnemySpawn {
                enemy_type: EnemyType::Mosquito,
                coordinates: Vec2 { x: 20.0, y: 0.0 },
                base_speed: 5.0,
                elapsed: 2.4,
                steps: vec![],
                contains: None,
            })],
        },
        StageStep::Movement {
            coordinates: Vec2 { x: 34.0, y: 62.0 },
            base_speed: 8.0,
            spawns: vec![],
        },
        //TODO
        StageStep::Stop {
            resume_conditions: Some(vec![]),
            max_duration: Some(4),
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
