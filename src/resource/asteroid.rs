use crate::stage::data::{ContainerSpawn, DestructibleSpawn, EnemySpawn, PowerupSpawn, StageStep};
use crate::stage::data::{
    DestructibleType, EnemyStep, EnemyType, PowerupType, StageActionResumeCondition, StageSpawn,
};
use bevy::prelude::*;

use super::asset_data::*;

pub const ASTEROID_DATA: AssetData<'static> = AssetData {
    name: "Asteroid",
    background: "backgrounds/stage_asteroid/cavern.png",
    skybox: Some("backgrounds/stage_asteroid/space.png"),
    start_coordinates: Some(Vec2::new(0.0, 0.0)),
    _get_spawns,
    _get_steps,
};

pub fn _get_spawns() -> Vec<StageSpawn> {
    let mut spawns = Vec::new();

    spawns.push(StageSpawn::Destructible(DestructibleSpawn {
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
    }));
    spawns.push(StageSpawn::Destructible(DestructibleSpawn {
        destructible_type: DestructibleType::Window,
        coordinates: Vec2 { x: 20.0, y: 0.0 },
        elapsed: 0.0,
        contains: Some(Box::new(ContainerSpawn::Powerup(PowerupSpawn {
            powerup_type: PowerupType::BigHealthpack,
            coordinates: Vec2 { x: 30.0, y: 10.0 },
            elapsed: 0.0,
        }))),
    }));
    spawns.push(StageSpawn::Powerup(PowerupSpawn {
        powerup_type: PowerupType::BigHealthpack,
        coordinates: Vec2 { x: 30.0, y: 10.0 },
        elapsed: 0.0,
    }));

    return spawns;
}

pub fn _get_steps() -> Vec<StageStep> {
    let mut steps = Vec::new();

    steps.push(StageStep::Movement {
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
    });
    steps.push(StageStep::Stop {
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
    });
    steps.push(StageStep::Movement {
        coordinates: Vec2 { x: 34.0, y: 62.0 },
        base_speed: 8.0,
        spawns: vec![],
    });
    steps.push(
        //TODO
        StageStep::Stop {
            resume_conditions: Some(vec![]),
            max_duration: Some(4),
            spawns: vec![],
        },
    );
    steps.push(StageStep::Movement {
        coordinates: Vec2 { x: 90.0, y: 0.0 },
        base_speed: 4.0,
        spawns: vec![],
    });
    steps.push(StageStep::Movement {
        coordinates: Vec2 { x: 0.0, y: 0.0 },
        base_speed: 2.0,
        spawns: vec![],
    });
    steps.push(
        //TODO
        StageStep::Stop {
            resume_conditions: Some(vec![
                StageActionResumeCondition::KillAll,
                StageActionResumeCondition::KillBoss,
            ]),
            max_duration: None,
            spawns: vec![],
        },
    );

    return steps;
}
