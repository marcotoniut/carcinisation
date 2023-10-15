use crate::stage::components::StopStageStep;
use crate::stage::data::*;
use crate::stage::destructible::data::DestructibleSpawn;
use crate::stage::enemy::data::steps::EnemyStep;
use bevy::prelude::*;

use lazy_static::lazy_static;

const OBJECT_FIBERTREE_Y: f32 = 13.;
const OBJECT_LAMP_Y: f32 = -5.;

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
        StageSpawn::Object(ObjectSpawn::rugpark_sign_base(400., 20.)),
        StageSpawn::Destructible(DestructibleSpawn::trashcan_base(100., 67., 1)),
        StageSpawn::Destructible(DestructibleSpawn::trashcan_base(220., 67., 1)),
        // StageSpawn::Destructible(DestructibleSpawn::crystal_base(125., 32.)),
        // StageSpawn::Destructible(DestructibleSpawn::mushroom_base(60., 12.)),
        StageSpawn::Object(ObjectSpawn::fibertree_base(30., OBJECT_FIBERTREE_Y)),
        StageSpawn::Object(ObjectSpawn::fibertree_base(180., OBJECT_FIBERTREE_Y)),
        StageSpawn::Object(ObjectSpawn::bench_big_base(50., 65.)),
        StageSpawn::Object(ObjectSpawn::bench_big_base(200., 60.)),
        StageSpawn::Destructible(
            DestructibleSpawn::lamp_base(75., OBJECT_LAMP_Y).drops(ContainerSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(60., 100.))
                    .with_elapsed(0.4)
                    .with_steps_vec(vec![
                        EnemyStep::idle_base().with_duration(3.).into(),
                        EnemyStep::attack_base().with_duration(1.).into(),
                    ]),
            )),
        ),
        StageSpawn::Destructible(
            DestructibleSpawn::lamp_base(260., OBJECT_LAMP_Y)
                .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
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
        StageStep::Stop(
            StopStageStep::new()
                .with_floor_depths(
                    vec![(3, 70.0), (4, 50.0), (5, 30.0), (6, 0.0)]
                        .into_iter()
                        .collect(),
                )
                .with_max_duration(30.)
                .add_spawns(vec![
                    StageSpawn::Enemy(
                        EnemySpawn::mosquito_base()
                            .with_coordinates(Vec2::new(60.0, 100.0))
                            .with_elapsed(0.1)
                            .with_steps_vec(vec![
                                EnemyStep::linear_movement_base()
                                    .with_direction(Vec2::new(-1., -0.2))
                                    .with_trayectory(30.)
                                    .with_depth_movement(2)
                                    .into(),
                                EnemyStep::idle_base().with_duration(3.).into(),
                                EnemyStep::linear_movement_base()
                                    .with_direction(Vec2::new(1., -0.5))
                                    .with_trayectory(50.)
                                    .with_depth_movement(-1)
                                    .into(),
                                EnemyStep::linear_movement_base()
                                    .opposite_direction()
                                    .into(),
                            ])
                            .drops(ContainerSpawn::Pickup(PickupSpawn::small_healthpack_base())),
                    ),
                    StageSpawn::Enemy(
                        EnemySpawn::mosquito_variant_circle()
                            .with_coordinates(Vec2::new(30.0, 60.0))
                            .into(),
                    ),
                    // StageSpawn::Enemy(
                    //     EnemySpawn::tardigrade_base()
                    //         .with_coordinates(Vec2::new(90.0, 50.0))
                    //         .with_elapsed(34.),
                    // ),
                    // StageSpawn::Enemy(
                    //     EnemySpawn::tardigrade_base().with_coordinates(Vec2::new(120.0, 30.0)),
                    // ),
                    StageSpawn::Enemy(
                        EnemySpawn::mosquito_variant_linear()
                            .with_y(30.)
                            .with_elapsed(85.0)
                            .add_step(EnemyStep::circle_around_base().into())
                            .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                    ),
                    StageSpawn::Enemy(
                        EnemySpawn::mosquito_variant_linear_opposite()
                            .with_elapsed(45.1)
                            .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                    ),
                ]),
        ),
        StageStep::movement_base(100.0, 0.0).add_spawns(vec![
            StageSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(120.0, 100.0))
                    .with_elapsed(35.1)
                    .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
            ),
            // StageSpawn::Enemy(
            //     EnemySpawn::mosquito_variant_circle()
            //         .with_coordinates(Vec2::new(60.0, 70.0))
            //         .with_elapsed(23.8),
            // ),
            // StageSpawn::Enemy(
            //     EnemySpawn::mosquito_base()
            //         .with_coordinates(Vec2::new(130.0, 70.0))
            //         .with_elapsed(1.8)
            //         .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
            // ),
        ]),
        StageStep::Stop(StopStageStep::new().with_max_duration(15.).add_spawns(
            vec![StageSpawn::Enemy(
                    EnemySpawn::mosquito_base()
                        .with_coordinates(Vec2::new(130.0, 70.0))
                        .with_elapsed(35.)
                        .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base())),
                )],
        )),
        StageStep::Stop(StopStageStep::new().with_max_duration(100.)),
    ]
}
