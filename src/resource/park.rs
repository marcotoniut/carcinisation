use crate::cinemachine::scene_intro::*;
use crate::plugins::movement::structs::MovementDirection;
use crate::stage::components::StopStageStep;
use crate::stage::data::*;
use crate::stage::destructible::data::DestructibleSpawn;
use bevy::prelude::*;

use lazy_static::lazy_static;

const OBJECT_FIBERTREE_Y: f32 = 13.;
const OBJECT_LAMP_Y: f32 = -5.;

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
        StageSpawn::Object(ObjectSpawn::rugpark_sign_base(400., 20.)),
        StageSpawn::Object(ObjectSpawn::fibertree_base(10., OBJECT_FIBERTREE_Y)),
        StageSpawn::Object(ObjectSpawn::fibertree_base(180., OBJECT_FIBERTREE_Y)),
        StageSpawn::Object(ObjectSpawn::bench_big_base(20., 65.)),
        StageSpawn::Object(ObjectSpawn::bench_big_base(200., 60.)),
        StageSpawn::Object(ObjectSpawn::bench_small_base(100., 65.)),
        StageSpawn::Destructible(
            DestructibleSpawn::lamp_base(70., OBJECT_LAMP_Y).drops(ContainerSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(79., 100.))
                    .with_elapsed(0.4)
                    .with_steps_vec(vec![
                        EnemyStep::linear_movement_base().with_detph_movement(1),
                        EnemyStep::Idle { duration: 1. },
                        EnemyStep::linear_movement_base().opposite_direction(),
                        EnemyStep::linear_movement_base()
                            .with_radius(10.)
                            .with_detph_movement(-1),
                    ]),
            )),
        ),
        StageSpawn::Destructible(
            DestructibleSpawn::lamp_base(260.0, OBJECT_LAMP_Y)
                .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
        ),
        StageSpawn::Pickup(
            PickupSpawn::big_healthpack_base().with_coordinates(Vec2::new(30., 10.)),
        ),
    ]
}

pub fn make_steps() -> Vec<StageStep> {
    vec![
        StageStep::movement_base(0.0, 0.0),
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
        StageStep::movement_base(50.0, 0.0)
            .with_floor_depths(
                vec![(3, 70.0), (4, 50.0), (5, 30.0), (6, 0.0)]
                    .into_iter()
                    .collect(),
            )
            .add_spawns(vec![
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_variant_circle()
                        .with_coordinates(Vec2::new(60.0, 100.0))
                        .with_elapsed(5.4)
                        .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .with_coordinates(Vec2::new(120.0, 100.0))
                        .with_elapsed(5.1)
                        .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                ),
                StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .with_coordinates(Vec2::new(130.0, 70.0))
                        .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                ),
            ]),
        StageStep::Stop(StopStageStep::new().with_max_duration(30.).add_spawns(vec![
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(70.0, 70.0))
                    .with_elapsed(2.4)
                    .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(90.0, 70.0))
                    .with_elapsed(10.8),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_variant_circle()
                    .with_coordinates(Vec2::new(12.0, 50.0))
                    .with_speed(2.0)
                    .with_elapsed(24.8)
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(170., 68.))
                    .with_elapsed(50.8)
                    .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
            ),
        ])),
        StageStep::movement_base(220., 0.).add_spawns(vec![
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(70.0, 60.0))
                    .with_elapsed(6.8)
                    .with_steps_vec(vec![EnemyStep::Circle {
                        detph_movement: None,
                        direction: MovementDirection::Positive,
                        duration: 999.0,
                        radius: 3.0,
                    }]),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(12.0, 50.0))
                    .with_elapsed(22.8),
            ),
            StageSpawn::Enemy(
                EnemySpawn::tardigrade_base()
                    .with_coordinates(Vec2::new(30.0, 90.0))
                    .with_speed(2.0)
                    .with_elapsed(22.8),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(30.0, 50.0))
                    .with_speed(2.0)
                    .with_elapsed(22.8),
            ),
            StageSpawn::Pickup(
                PickupSpawn::big_healthpack_base()
                    .with_coordinates(Vec2::new(144.0, 68.0))
                    .with_elapsed(0.0),
            ),
        ]),
        StageStep::Stop(StopStageStep::new().with_max_duration(20.)),
        StageStep::movement_base(280., 0.).add_spawns(vec![
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(60.0, 80.0))
                    .with_elapsed(8.4),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(100.0, 60.0))
                    .with_elapsed(6.1),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(30.0, 80.0))
                    .with_elapsed(7.4),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(120.0, 60.0))
                    .with_speed(5.0)
                    .with_elapsed(6.1),
                // direction: MovementDirection::Left,
                // radius: 25.0,
                // time_offset: 2.0,
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(80.0, 80.0))
                    .with_speed(5.0)
                    .with_elapsed(7.4),
                // direction: MovementDirection::Right,
                // radius: 15.0,
                // time_offset: 0.5,
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(20.0, 60.0))
                    .with_speed(5.0)
                    .with_elapsed(8.1), // direction: MovementDirection::Left,
                                        // radius: 25.0,
                                        // time_offset: 3.0,
            ),
        ]),
        StageStep::movement_base(500.0, 0.0).add_spawns(vec![
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(100.0, 80.0))
                    .with_speed(5.0)
                    .with_elapsed(8.4)
                    // direction: MovementDirection::Right,
                    // radius: 15.0,
                    // time_offset: 0.5,
                    .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(100.0, 100.0))
                    .with_speed(5.0)
                    .with_elapsed(6.1), // direction: MovementDirection::Left,
                                        // radius: 25.0,
                                        // time_offset: 3.0,
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(60.0, 120.0))
                    .with_elapsed(7.4)
                    .with_steps_vec(vec![EnemyStep::Circle {
                        detph_movement: None,
                        duration: 999.0,
                        direction: MovementDirection::Positive,
                        radius: 13.0,
                    }]),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(280.0, 80.0))
                    .with_elapsed(5.6),
            ),
            StageSpawn::Enemy(
                EnemySpawn::tardigrade_base()
                    .with_elapsed(6.4)
                    .with_coordinates(Vec2::new(320.0, 160.0))
                    .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(400.0, 160.0))
                    .with_elapsed(7.4),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(360.0, 160.0))
                    .with_speed(5.0)
                    .with_elapsed(8.1)
                    .with_steps_vec(vec![
                        // direction: MovementDirection::Left,
                        // radius: 25.0,
                        // time_offset: 3.0,
                    ]),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(410.0, 180.0))
                    .with_speed(5.0)
                    .with_elapsed(7.4)
                    .with_steps_vec(vec![
                        // direction: MovementDirection::Right,
                        // radius: 15.0,
                        // time_offset: 1.0,
                    ]),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(440.0, 190.0))
                    .with_speed(5.0)
                    .with_elapsed(6.1)
                    .with_steps_vec(vec![EnemyStep::Circle {
                        detph_movement: None,
                        direction: MovementDirection::Positive,
                        duration: 999.0,
                        radius: 23.0,
                    }]),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(120.0, 160.0))
                    .with_elapsed(6.1)
                    .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(140.0, 160.0))
                    .with_elapsed(7.4)
                    .with_steps_vec(vec![
                        // direction: MovementDirection::Right,
                        // radius: 15.0,
                        // time_offset: 0.5,
                    ]),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(520.0, 210.0))
                    .with_speed(5.0)
                    .with_elapsed(8.1)
                    .with_steps_vec(vec![EnemyStep::Circle {
                        detph_movement: None,
                        duration: 999.0,
                        radius: 13.0,
                        direction: MovementDirection::Positive,
                    }])
                    .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
            ),
        ]),
        StageStep::Stop(StopStageStep::new().with_max_duration(30.)),
        StageStep::movement_base(850., 155.).add_spawns(vec![
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(55.0, 60.0))
                    .with_speed(5.0)
                    .with_elapsed(8.4)
                    .with_steps_vec(vec![EnemyStep::Circle {
                        detph_movement: None,
                        duration: 999.0,
                        radius: 15.0,
                        direction: MovementDirection::Positive,
                    }])
                    .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(100.0, 60.0))
                    .with_speed(5.0)
                    .with_elapsed(6.1)
                    .with_steps_vec(vec![
                        // direction: MovementDirection::Left,
                        // radius: 25.0,
                        // time_offset: 3.0,
                    ]),
            ),
        ]),
        StageStep::Stop(StopStageStep::new().with_max_duration(40.)),
        StageStep::movement_base(920., 155.).add_spawns(vec![
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(60.0, 80.0))
                    .with_speed(5.0)
                    .with_elapsed(8.4)
                    .with_steps_vec(vec![]),
            ),
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(100.0, 60.0))
                    .with_elapsed(6.1),
            ),
        ]),
        StageStep::Stop(StopStageStep::new().with_max_duration(45.)),
        StageStep::Stop(
            StopStageStep::new()
                .with_max_duration(45.)
                .with_kill_all(false)
                .with_kill_boss(true),
            // TODO
            // .add_spawns(Boss),
        ),
    ]
}
