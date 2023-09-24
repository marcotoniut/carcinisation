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

lazy_static! {
    pub static ref STAGE_ASTEROID_DATA: StageData = StageData {
        name: "Asteroid".to_string(),
        music_path: "audio/music/stage_3.ogg".to_string(),
        background_path: "backgrounds/asteroid/background.png".to_string(),
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
    vec![]
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
                    direction: MovementDirection::Right,
                    radius: 15.,
                    time_offset: 1.5,

                    steps: vec![EnemyStep::Circle {
                        duration: 4.0,
                        radius: 10.0,
                        direction: MovementDirection::Left,
                    }],
                    contains: None,
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 120.0, y: 100.0 },
                    base_speed: 5.0,
                    elapsed: 0.4,
                    steps: vec![],
                    direction: MovementDirection::Right,
                    radius: 15.,
                    time_offset: 1.5,

                    contains: None,
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 130.0, y: 70.0 },
                    base_speed: 5.0,
                    elapsed: 2.3,
                    steps: vec![],
                    direction: MovementDirection::Right,
                    radius: 15.,
                    time_offset: 1.5,

                    contains: None,
                }),
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![StageActionResumeCondition::KillAll]),
            max_duration: Some(40. / CAMERA_BASE_SPEED),
            spawns: vec![StageSpawn::Enemy(EnemySpawn {
                enemy_type: EnemyType::Mosquito,
                coordinates: Vec2 { x: 20.0, y: 0.0 },
                base_speed: 5.0,
                elapsed: 2.4,
                steps: vec![],
                direction: MovementDirection::Right,
                radius: 15.,
                time_offset: 1.5,

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
            max_duration: Some(4. / CAMERA_BASE_SPEED),
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
