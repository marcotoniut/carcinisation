use crate::stage::data::{ContainerSpawn, DestructibleSpawn, EnemySpawn, PowerupSpawn, StageStep};
use crate::stage::data::{
    DestructibleType, EnemyStep, EnemyType, PowerupType, StageActionResumeCondition, StageSpawn,
};
use bevy::prelude::*;
use lazy_static::lazy_static;

use super::asset_data::*;

lazy_static! {
    pub static ref STAGE_ASTEROID_DATA: AssetData = AssetData {
        name: "Asteroid".to_string(),
        background: "backgrounds/asteroid/background.png".to_string(),
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
            destructible_type: DestructibleType::Lamp,
            coordinates: Vec2 { x: 30.0, y: 0.0 },
            elapsed: 0.0,
            contains: Some(Box::new(ContainerSpawn::Enemy(EnemySpawn {
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
                    EnemyStep::Stop { duration: 1.0 },
                    EnemyStep::Attack { duration: 1.0 },
                    EnemyStep::Movement {
                        coordinates: Vec2 { x: 10.0, y: 0.0 },
                        attacking: true,
                        speed: 3.0,
                    },
                    EnemyStep::CircleAround { duration: 4.0 },
                ],
                contains: None,
            }))),
        }),
        StageSpawn::Destructible(DestructibleSpawn {
            destructible_type: DestructibleType::Window,
            coordinates: Vec2 { x: 20.0, y: 0.0 },
            elapsed: 0.0,
            contains: Some(Box::new(ContainerSpawn::Powerup(PowerupSpawn {
                powerup_type: PowerupType::BigHealthpack,
                coordinates: Vec2 { x: 30.0, y: 10.0 },
                elapsed: 0.0,
            }))),
        }),
        StageSpawn::Powerup(PowerupSpawn {
            powerup_type: PowerupType::BigHealthpack,
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
                        EnemyStep::Stop { duration: 1.0 },
                        EnemyStep::Attack { duration: 1.0 },
                        EnemyStep::Movement {
                            coordinates: Vec2 { x: 10.0, y: 0.0 },
                            attacking: true,
                            speed: 3.0,
                        },
                        EnemyStep::CircleAround { duration: 4.0 },
                    ],
                    contains: None,
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
