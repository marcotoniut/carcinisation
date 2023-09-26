use crate::cinemachine::scene_intro::*;
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
                        duration: 4.0,
                        radius: 10.0,
                        direction: MovementDirection::Left,
                    },
                ],
                // direction: MovementDirection::Left,
                // radius: 15.,
                // time_offset: 0.5,
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
            resume_conditions: Some(vec![]),
            max_duration: Some(3.0),
            spawns: vec![],
        },
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
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 120.0, y: 100.0 },
                    elapsed: 5.1 / CAMERA_BASE_SPEED,
                    contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                        pickup_type: PickupType::BigHealthpack,
                        ..default()
                    }))),
                    ..EnemySpawn::base_mosquito(
                        5.1 / CAMERA_BASE_SPEED,
                        Vec2 { x: 120.0, y: 100.0 },
                    )
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                        pickup_type: PickupType::BigHealthpack,
                        ..default()
                    }))),
                    ..EnemySpawn::base_mosquito(5.8 / CAMERA_BASE_SPEED, Vec2 { x: 130.0, y: 70.0 })
                }),
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![StageActionResumeCondition::KillAll]),
            max_duration: Some(30. / CAMERA_BASE_SPEED),
            spawns: vec![
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 70.0, y: 70.0 },
                    base_speed: CAMERA_BASE_SPEED,
                    elapsed: 2.4 / CAMERA_BASE_SPEED,
                    steps: vec![],

                    contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                        pickup_type: PickupType::BigHealthpack,
                        ..default()
                    }))),
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    elapsed: 10.8 / CAMERA_BASE_SPEED,
                    steps: vec![],
                    contains: None,
                    ..EnemySpawn::base_mosquito(40. / CAMERA_BASE_SPEED, Vec2 { x: 90.0, y: 70.0 })
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    base_speed: 2.0,
                    elapsed: 24.8 / CAMERA_BASE_SPEED,

                    contains: None,
                    steps: vec![EnemyStep::Circle {
                        duration: 999.0,
                        radius: 3.0,
                        direction: MovementDirection::Right,
                    }],
                    ..EnemySpawn::base_mosquito(40. / CAMERA_BASE_SPEED, Vec2 { x: 12.0, y: 50.0 })
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 170.0, y: 68.0 },
                    elapsed: 50.8 / CAMERA_BASE_SPEED,
                    steps: vec![],
                    contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                        pickup_type: PickupType::BigHealthpack,
                        ..default()
                    }))),
                    ..EnemySpawn::base_mosquito(21. / CAMERA_BASE_SPEED, Vec2 { x: 12.0, y: 50.0 })
                }),
            ],
        },
        //TEST down
        StageStep::Movement {
            coordinates: Vec2 { x: 220.0, y: 0.0 },
            base_speed: CAMERA_BASE_SPEED,
            spawns: vec![
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Tardigrade,
                    elapsed: 6.8 / CAMERA_BASE_SPEED,
                    contains: None,
                    steps: vec![EnemyStep::Circle {
                        duration: 999.0,
                        radius: 3.0,
                        direction: MovementDirection::Right,
                    }],

                    ..EnemySpawn::base_mosquito(20. / CAMERA_BASE_SPEED, Vec2 { x: 70.0, y: 60.0 })
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 12.0, y: 50.0 },
                    elapsed: 22.8 / CAMERA_BASE_SPEED,
                    contains: None,
                    ..EnemySpawn::base_mosquito(50. / CAMERA_BASE_SPEED, Vec2 { x: 12.0, y: 50.0 })
                }),
                StageSpawn::Enemy(EnemySpawn {
                    coordinates: Vec2 { x: 30.0, y: 90.0 },
                    base_speed: 2.0,
                    elapsed: 22.8 / CAMERA_BASE_SPEED,
                    contains: None,
                    ..EnemySpawn::base_tardigrade(
                        40. / CAMERA_BASE_SPEED,
                        Vec2 { x: 30.0, y: 90.0 },
                    )
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 30.0, y: 50.0 },
                    base_speed: 2.0,
                    elapsed: 22.8 / CAMERA_BASE_SPEED,
                    steps: vec![],

                    contains: None,
                }),
                StageSpawn::Pickup(PickupSpawn {
                    pickup_type: PickupType::BigHealthpack,
                    coordinates: Vec2 { x: 144.0, y: 68.0 },
                    elapsed: 0.0 / CAMERA_BASE_SPEED,
                }),
            ],
        },
        //TEST up
        StageStep::Stop {
            resume_conditions: Some(vec![]),
            max_duration: Some(40. / CAMERA_BASE_SPEED),
            spawns: vec![],
        },
        StageStep::Movement {
            coordinates: Vec2 { x: 280.0, y: 0.0 },
            base_speed: CAMERA_BASE_SPEED,
            spawns: vec![
                StageSpawn::Enemy(EnemySpawn {
                    coordinates: Vec2 { x: 60.0, y: 80.0 },
                    elapsed: 8.4 / CAMERA_BASE_SPEED,
                    contains: None,
                    ..EnemySpawn::base_mosquito(100. / CAMERA_BASE_SPEED, Vec2 { x: 12.0, y: 50.0 })
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 100.0, y: 60.0 },
                    base_speed: 5.0,
                    elapsed: 6.1 / CAMERA_BASE_SPEED,
                    steps: vec![],
                    contains: None,
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 30.0, y: 80.0 },
                    base_speed: 5.0,
                    elapsed: 7.4 / CAMERA_BASE_SPEED,
                    steps: vec![],
                    contains: None,
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 130.0, y: 60.0 },
                    base_speed: 5.0,
                    steps: vec![EnemyStep::Circle {
                        duration: 999.0,
                        radius: 3.0,
                        direction: MovementDirection::Right,
                    }],

                    elapsed: 5.1 / CAMERA_BASE_SPEED,

                    // direction: MovementDirection::Left,
                    // radius: 25.0,
                    // time_offset: 2.0,
                    contains: None,
                    ..EnemySpawn::base_mosquito(100. / CAMERA_BASE_SPEED, Vec2 { x: 12.0, y: 50.0 })
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 120.0, y: 80.0 },
                    base_speed: 5.0,
                    elapsed: 6.4 / CAMERA_BASE_SPEED,
                    steps: vec![],
                    // direction: MovementDirection::Right,
                    // radius: 15.0,
                    // time_offset: 0.5,
                    contains: None,
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Tardigrade,
                    coordinates: Vec2 { x: 60.0, y: 60.0 },
                    base_speed: 5.0,
                    elapsed: 8.1 / CAMERA_BASE_SPEED,
                    steps: vec![],
                    // direction: MovementDirection::Left,
                    // radius: 25.0,
                    // time_offset: 3.0,
                    contains: None,
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 100.0, y: 80.0 },
                    base_speed: 5.0,
                    elapsed: 7.4 / CAMERA_BASE_SPEED,
                    contains: None,
                    ..EnemySpawn::base_mosquito(2.1 / CAMERA_BASE_SPEED, Vec2 { x: 100.0, y: 80.0 })
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 120.0, y: 60.0 },
                    base_speed: 5.0,
                    elapsed: 6.1 / CAMERA_BASE_SPEED,

                    // direction: MovementDirection::Left,
                    // radius: 25.0,
                    // time_offset: 2.0,
                    contains: None,
                    ..EnemySpawn::base_mosquito(2.1 / CAMERA_BASE_SPEED, Vec2 { x: 120.0, y: 60.0 })
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 80.0, y: 80.0 },
                    base_speed: 5.0,
                    elapsed: 7.4 / CAMERA_BASE_SPEED,

                    // direction: MovementDirection::Right,
                    // radius: 15.0,
                    // time_offset: 0.5,
                    contains: None,
                    ..EnemySpawn::base_mosquito(2.1 / CAMERA_BASE_SPEED, Vec2 { x: 80.0, y: 80.0 })
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 20.0, y: 60.0 },
                    base_speed: 5.0,
                    elapsed: 8.1 / CAMERA_BASE_SPEED,
                    steps: vec![],
                    // direction: MovementDirection::Left,
                    // radius: 25.0,
                    // time_offset: 3.0,
                    contains: None,
                }),
            ],
        },
        StageStep::Movement {
            coordinates: Vec2 { x: 500.0, y: 0.0 },
            base_speed: CAMERA_BASE_SPEED,
            spawns: vec![
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 100.0, y: 80.0 },
                    base_speed: 5.0,
                    elapsed: 8.4 / CAMERA_BASE_SPEED,
                    // direction: MovementDirection::Right,
                    // radius: 15.0,
                    // time_offset: 0.5,
                    contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                        pickup_type: PickupType::BigHealthpack,
                        ..default()
                    }))),
                    ..EnemySpawn::base_mosquito(2.1 / CAMERA_BASE_SPEED, Vec2 { x: 100.0, y: 80.0 })
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 100.0, y: 100.0 },
                    base_speed: 5.0,
                    elapsed: 6.1 / CAMERA_BASE_SPEED,
                    // direction: MovementDirection::Left,
                    // radius: 25.0,
                    // time_offset: 3.0,
                    contains: None,
                    ..EnemySpawn::base_mosquito(
                        2.1 / CAMERA_BASE_SPEED,
                        Vec2 { x: 100.0, y: 100.0 },
                    )
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    elapsed: 7.4 / CAMERA_BASE_SPEED,
                    steps: vec![EnemyStep::Circle {
                        duration: 999.0,
                        radius: 13.0,
                        direction: MovementDirection::Right,
                    }],
                    contains: None,
                    ..EnemySpawn::base_mosquito(
                        120.1 / CAMERA_BASE_SPEED,
                        Vec2 { x: 60.0, y: 120.0 },
                    )
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 280.0, y: 160.0 },
                    base_speed: 5.0,
                    elapsed: 5.1 / CAMERA_BASE_SPEED,

                    // direction: MovementDirection::Left,
                    // radius: 25.0,
                    // time_offset: 2.0,
                    contains: None,
                    ..EnemySpawn::base_mosquito(2.1 / CAMERA_BASE_SPEED, Vec2 { x: 280.0, y: 80.0 })
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Tardigrade,
                    elapsed: 6.4 / CAMERA_BASE_SPEED,

                    // direction: MovementDirection::Right,
                    // radius: 15.0,
                    // time_offset: 0.5,
                    contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                        pickup_type: PickupType::BigHealthpack,
                        ..default()
                    }))),
                    ..EnemySpawn::base_mosquito(
                        80. / CAMERA_BASE_SPEED,
                        Vec2 { x: 320.0, y: 160.0 },
                    )
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 360.0, y: 160.0 },
                    base_speed: 5.0,
                    elapsed: 8.1 / CAMERA_BASE_SPEED,
                    steps: vec![],
                    // direction: MovementDirection::Left,
                    // radius: 25.0,
                    // time_offset: 3.0,
                    contains: None,
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 400.0, y: 170.0 },
                    elapsed: 7.4 / CAMERA_BASE_SPEED,
                    contains: None,
                    ..EnemySpawn::base_mosquito(
                        65. / CAMERA_BASE_SPEED,
                        Vec2 { x: 400.0, y: 160.0 },
                    )
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 410.0, y: 180.0 },
                    base_speed: 5.0,
                    elapsed: 7.4 / CAMERA_BASE_SPEED,
                    steps: vec![],
                    // direction: MovementDirection::Right,
                    // radius: 15.0,
                    // time_offset: 1.0,
                    contains: None,
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 440.0, y: 190.0 },
                    base_speed: 5.0,
                    elapsed: 6.1 / CAMERA_BASE_SPEED,
                    steps: vec![EnemyStep::Circle {
                        duration: 999.0,
                        radius: 23.0,
                        direction: MovementDirection::Right,
                    }],
                    contains: None,
                }),
                StageSpawn::Enemy(EnemySpawn {
                    elapsed: 6.1 / CAMERA_BASE_SPEED,
                    contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                        pickup_type: PickupType::BigHealthpack,
                        ..default()
                    }))),
                    ..EnemySpawn::base_mosquito(
                        325. / CAMERA_BASE_SPEED,
                        Vec2 { x: 120.0, y: 160.0 },
                    )
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 480.0, y: 200.0 },
                    base_speed: 5.0,
                    elapsed: 7.4 / CAMERA_BASE_SPEED,
                    steps: vec![],
                    // direction: MovementDirection::Right,
                    // radius: 15.0,
                    // time_offset: 0.5,
                    contains: None,
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 520.0, y: 210.0 },
                    base_speed: 5.0,
                    elapsed: 8.1 / CAMERA_BASE_SPEED,
                    steps: vec![EnemyStep::Circle {
                        duration: 999.0,
                        radius: 13.0,
                        direction: MovementDirection::Right,
                    }],
                    contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                        pickup_type: PickupType::BigHealthpack,
                        ..default()
                    }))),
                }),
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![]),
            max_duration: Some(40. / CAMERA_BASE_SPEED),
            spawns: vec![
                StageSpawn::Enemy(EnemySpawn {
                    base_speed: 5.0,
                    elapsed: 8.1 / CAMERA_BASE_SPEED,
                    contains: None,
                    ..EnemySpawn::base_mosquito(
                        123. / CAMERA_BASE_SPEED,
                        Vec2 { x: 100.0, y: 60.0 },
                    )
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 530.0, y: 200.0 },
                    base_speed: 5.0,
                    elapsed: 8.1 / CAMERA_BASE_SPEED,
                    steps: vec![],
                    // direction: MovementDirection::Right,
                    // radius: 25.0,
                    // time_offset: 3.0,
                    contains: None,
                }),
            ],
        },
        StageStep::Movement {
            coordinates: Vec2 { x: 850.0, y: 155.0 },
            base_speed: CAMERA_BASE_SPEED,
            spawns: vec![
                StageSpawn::Enemy(EnemySpawn {
                    base_speed: 5.0,
                    elapsed: 8.4 / CAMERA_BASE_SPEED,

                    steps: vec![EnemyStep::Circle {
                        duration: 999.0,
                        radius: 15.0,
                        direction: MovementDirection::Right,
                    }],

                    contains: Some(Box::new(ContainerSpawn::Pickup(PickupSpawn {
                        pickup_type: PickupType::BigHealthpack,
                        ..default()
                    }))),
                    ..EnemySpawn::base_mosquito(223. / CAMERA_BASE_SPEED, Vec2 { x: 55.0, y: 60.0 })
                }),
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 100.0, y: 60.0 },
                    base_speed: 5.0,
                    elapsed: 6.1 / CAMERA_BASE_SPEED,
                    steps: vec![],
                    // direction: MovementDirection::Left,
                    // radius: 25.0,
                    // time_offset: 3.0,
                    contains: None,
                }),
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![]),
            max_duration: Some(40. / CAMERA_BASE_SPEED),
            spawns: vec![],
        },
        StageStep::Movement {
            coordinates: Vec2 { x: 920.0, y: 155.0 },
            base_speed: CAMERA_BASE_SPEED,
            spawns: vec![
                StageSpawn::Enemy(EnemySpawn {
                    enemy_type: EnemyType::Mosquito,
                    coordinates: Vec2 { x: 60.0, y: 80.0 },
                    base_speed: 5.0,
                    elapsed: 8.4 / CAMERA_BASE_SPEED,
                    steps: vec![],
                    contains: None,
                }),
                StageSpawn::Enemy(EnemySpawn {
                    coordinates: Vec2 { x: 100.0, y: 60.0 },
                    elapsed: 6.1 / CAMERA_BASE_SPEED,
                    contains: None,
                    ..EnemySpawn::base_mosquito(
                        223. / CAMERA_BASE_SPEED,
                        Vec2 { x: 100.0, y: 60.0 },
                    )
                }),
            ],
        },
        StageStep::Stop {
            resume_conditions: Some(vec![]),
            max_duration: Some(40. / CAMERA_BASE_SPEED),
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
