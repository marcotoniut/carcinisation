use crate::cinemachine::scene_intro::*;
use crate::plugins::movement::structs::MovementDirection;
use crate::resource::CAMERA_BASE_SPEED;
use crate::stage::data::{
    ContainerSpawn, DestructibleSpawn, EnemySpawn, ObjectSpawn, ObjectType, PickupSpawn,
    SkyboxData, StageData, StageStep,
};
use crate::stage::data::{DestructibleType, EnemyStep, StageActionResumeCondition, StageSpawn};
use bevy::prelude::*;

use lazy_static::lazy_static;

const OBJECT_FIBERTREE_Y: f32 = 13.;

lazy_static! {
    pub static ref STAGE_PARK_DATA: StageData = StageData {
        name: "Park".to_string(),
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
            coordinates: Vec2::new(20., 65.),
        }),
        StageSpawn::Object(ObjectSpawn {
            object_type: ObjectType::BenchBig,
            coordinates: Vec2::new(200., 60.),
        }),
        StageSpawn::Object(ObjectSpawn {
            object_type: ObjectType::BenchSmall,
            coordinates: Vec2::new(100., 65.),
        }),
        StageSpawn::Destructible(DestructibleSpawn {
            coordinates: Vec2::new(30., 0.),
            contains: Some(Box::new(ContainerSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .set_coordinates(Vec2::new(60., 100.))
                    .set_elapsed(0.4)
                    .set_steps_vec(vec![
                        EnemyStep::LinearMovement {
                            coordinates: Vec2::new(50., 0.),
                            attacking: true,
                            speed: 2.,
                        },
                        EnemyStep::Idle { duration: 1. },
                        EnemyStep::Attack { duration: 1. },
                        EnemyStep::LinearMovement {
                            coordinates: Vec2::new(10., 0.),
                            attacking: true,
                            speed: 3.,
                        },
                        EnemyStep::Circle {
                            duration: 4.0,
                            radius: 10.0,
                            direction: MovementDirection::Negative,
                        },
                    ]),
            ))),
            destructible_type: DestructibleType::Lamp,
        }),
        StageSpawn::Destructible(DestructibleSpawn {
            coordinates: Vec2::new(20.0, 0.0),
            contains: Some(Box::new(ContainerSpawn::Pickup(
                PickupSpawn::big_healthpack_base(),
            ))),
            destructible_type: DestructibleType::Lamp,
        }),
        StageSpawn::Pickup(PickupSpawn::big_healthpack_base().set_coordinates(Vec2::new(30., 10.))),
    ]
}

pub fn make_steps() -> Vec<StageStep> {
    vec![
        StageStep::Movement {
            coordinates: Vec2::new(0.0, 0.0),
            base_speed: 8.0,
            spawns: vec![].into(),
        },
        // StageStep::Cinematic {
        //     cinematic: INTRO_ANIMATIC_0.clone(),
        // },
        // StageStep::Cinematic {
        //     cinematic: INTRO_ANIMATIC_1.clone(),
        // },
        // StageStep::Cinematic {
        //     cinematic: INTRO_ANIMATIC_2.clone(),
        // },
        // StageStep::Cinematic {
        //     cinematic: INTRO_ANIMATIC_3.clone(),
        // },
        // StageStep::Cinematic {
        //     cinematic: INTRO_ANIMATIC_4.clone(),
        // },
        StageStep::Stop {
            resume_conditions: Some(vec![].into()),
            max_duration: Some(0.1),
            spawns: vec![].into(),
        },
        StageStep::Movement {
            coordinates: Vec2::new(50.0, 0.0),
            base_speed: CAMERA_BASE_SPEED,
            spawns: vec![
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(60.0, 100.0))
                        .set_elapsed(5.4)
                        .set_steps_vec(vec![
                            EnemyStep::Circle {
                                duration: 4.0,
                                radius: 10.0,
                                direction: MovementDirection::Negative,
                            },
                            EnemyStep::LinearMovement {
                                coordinates: Vec2::new(10.0, 0.0),
                                attacking: true,
                                speed: 3.0,
                            },
                        ])
                        .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(120.0, 100.0))
                        .set_elapsed(5.1)
                        .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(130.0, 70.0))
                        .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                ),
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![StageActionResumeCondition::KillAll]),
            max_duration: Some(30.),
            spawns: vec![
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(70.0, 70.0))
                        .set_elapsed(2.4)
                        .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(90.0, 70.0))
                        .set_elapsed(10.8),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(12.0, 50.0))
                        .set_speed(2.0)
                        .set_elapsed(24.8)
                        .set_steps_vec(vec![EnemyStep::Circle {
                            duration: 999.0,
                            radius: 3.0,
                            direction: MovementDirection::Positive,
                        }]),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(170.0, 68.0))
                        .set_elapsed(50.8)
                        .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                ),
            ],
        },
        StageStep::Movement {
            coordinates: Vec2::new(220.0, 0.0),
            base_speed: CAMERA_BASE_SPEED,
            spawns: vec![
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(70.0, 60.0))
                        .set_elapsed(6.8)
                        .set_steps_vec(vec![EnemyStep::Circle {
                            duration: 999.0,
                            radius: 3.0,
                            direction: MovementDirection::Positive,
                        }]),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(12.0, 50.0))
                        .set_elapsed(22.8),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::tardigrade_base()
                        .set_coordinates(Vec2::new(30.0, 90.0))
                        .set_speed(2.0)
                        .set_elapsed(22.8),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(30.0, 50.0))
                        .set_speed(2.0)
                        .set_elapsed(22.8),
                ),
                StageSpawn::Pickup(
                    PickupSpawn::big_healthpack_base()
                        .set_coordinates(Vec2::new(144.0, 68.0))
                        .set_elapsed(0.0),
                ),
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![].into()),
            max_duration: Some(40.),
            spawns: vec![].into(),
        },
        StageStep::Movement {
            coordinates: Vec2::new(280.0, 0.0),
            base_speed: CAMERA_BASE_SPEED,
            spawns: vec![
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(60.0, 80.0))
                        .set_elapsed(8.4),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(100.0, 60.0))
                        .set_elapsed(6.1),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(30.0, 80.0))
                        .set_elapsed(7.4),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(120.0, 60.0))
                        .set_speed(5.0)
                        .set_elapsed(6.1),
                    // direction: MovementDirection::Left,
                    // radius: 25.0,
                    // time_offset: 2.0,
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(80.0, 80.0))
                        .set_speed(5.0)
                        .set_elapsed(7.4),
                    // direction: MovementDirection::Right,
                    // radius: 15.0,
                    // time_offset: 0.5,
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(20.0, 60.0))
                        .set_speed(5.0)
                        .set_elapsed(8.1), // direction: MovementDirection::Left,
                                           // radius: 25.0,
                                           // time_offset: 3.0,
                ),
            ],
        },
        StageStep::Movement {
            coordinates: Vec2::new(500.0, 0.0),
            base_speed: CAMERA_BASE_SPEED,
            spawns: vec![
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(100.0, 80.0))
                        .set_speed(5.0)
                        .set_elapsed(8.4)
                        // direction: MovementDirection::Right,
                        // radius: 15.0,
                        // time_offset: 0.5,
                        .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(100.0, 100.0))
                        .set_speed(5.0)
                        .set_elapsed(6.1), // direction: MovementDirection::Left,
                                           // radius: 25.0,
                                           // time_offset: 3.0,
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(60.0, 120.0))
                        .set_elapsed(7.4)
                        .set_steps_vec(vec![EnemyStep::Circle {
                            duration: 999.0,
                            radius: 13.0,
                            direction: MovementDirection::Positive,
                        }]),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(280.0, 80.0))
                        .set_elapsed(5.6),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::tardigrade_base()
                        .set_elapsed(6.4)
                        .set_coordinates(Vec2::new(320.0, 160.0))
                        .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(400.0, 160.0))
                        .set_elapsed(7.4),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(360.0, 160.0))
                        .set_speed(5.0)
                        .set_elapsed(8.1)
                        .set_steps_vec(vec![
                            // direction: MovementDirection::Left,
                            // radius: 25.0,
                            // time_offset: 3.0,
                        ]),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(410.0, 180.0))
                        .set_speed(5.0)
                        .set_elapsed(7.4)
                        .set_steps_vec(vec![
                            // direction: MovementDirection::Right,
                            // radius: 15.0,
                            // time_offset: 1.0,
                        ]),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(440.0, 190.0))
                        .set_speed(5.0)
                        .set_elapsed(6.1)
                        .set_steps_vec(vec![EnemyStep::Circle {
                            duration: 999.0,
                            radius: 23.0,
                            direction: MovementDirection::Positive,
                        }]),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(120.0, 160.0))
                        .set_elapsed(6.1)
                        .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(140.0, 160.0))
                        .set_elapsed(7.4)
                        .set_steps_vec(vec![
                            // direction: MovementDirection::Right,
                            // radius: 15.0,
                            // time_offset: 0.5,
                        ]),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(520.0, 210.0))
                        .set_speed(5.0)
                        .set_elapsed(8.1)
                        .set_steps_vec(vec![EnemyStep::Circle {
                            duration: 999.0,
                            radius: 13.0,
                            direction: MovementDirection::Positive,
                        }])
                        .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                ),
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![].into()),
            max_duration: Some(40.),
            spawns: vec![],
        },
        StageStep::Movement {
            coordinates: Vec2::new(850.0, 155.0),
            base_speed: CAMERA_BASE_SPEED,
            spawns: vec![
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(55.0, 60.0))
                        .set_speed(5.0)
                        .set_elapsed(8.4)
                        .set_steps_vec(vec![EnemyStep::Circle {
                            duration: 999.0,
                            radius: 15.0,
                            direction: MovementDirection::Positive,
                        }])
                        .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(100.0, 60.0))
                        .set_speed(5.0)
                        .set_elapsed(6.1)
                        .set_steps_vec(vec![
                            // direction: MovementDirection::Left,
                            // radius: 25.0,
                            // time_offset: 3.0,
                        ]),
                ),
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![].into()),
            max_duration: Some(40.),
            spawns: vec![].into(),
        },
        StageStep::Movement {
            coordinates: Vec2::new(920.0, 155.0),
            base_speed: CAMERA_BASE_SPEED,
            spawns: vec![
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(60.0, 80.0))
                        .set_speed(5.0)
                        .set_elapsed(8.4)
                        .set_steps_vec(vec![]),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .set_coordinates(Vec2::new(100.0, 60.0))
                        .set_elapsed(6.1),
                ),
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![].into()),
            max_duration: Some(40.),
            spawns: vec![].into(),
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
