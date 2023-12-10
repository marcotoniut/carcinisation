use crate::stage::components::placement::Depth;
use crate::stage::components::{MovementStageStep, StopStageStep};
use crate::stage::data::*;
use crate::stage::destructible::data::{DestructibleSpawn, LampDepth};
use crate::stage::enemy::data::steps::EnemyStep;
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use lazy_static::lazy_static;
use std::sync::Arc;

const OBJECT_FIBERTREE_Y: f32 = 13.;
const OBJECT_LAMP_Y: f32 = -5.;

lazy_static! {
    pub static ref STAGE_PARK_DATA: Arc<StageData> = StageData {
        name: "Park".to_string(),
        music_path: assert_assets_path!("audio/music/stage_1.ogg").to_string(),
        background_path: assert_assets_path!("backgrounds/rugpark/background.png").to_string(),
        skybox: SkyboxData {
            path: assert_assets_path!("backgrounds/rugpark/skybox.png").to_string(),
            frames: 2,
        },
        start_coordinates: Some(Vec2::new(0.0, 0.0)),
        spawns: make_spawns(),
        steps: make_steps(),
    }
    .into();
}

pub fn make_spawns() -> Vec<StageSpawn> {
    vec![
        ObjectSpawn::rugpark_sign_base(400., 20.).into(),
        ObjectSpawn::fibertree_base(10., OBJECT_FIBERTREE_Y).into(),
        ObjectSpawn::fibertree_base(180., OBJECT_FIBERTREE_Y).into(),
        ObjectSpawn::bench_big_base(20., 65.).into(),
        ObjectSpawn::bench_big_base(200., 60.).into(),
        ObjectSpawn::bench_small_base(100., 65.).into(),
        DestructibleSpawn::lamp_base(70., OBJECT_LAMP_Y, LampDepth::Three)
            .drops(ContainerSpawn::Enemy(
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(79., 100.))
                    .with_elapsed(0.4)
                    .with_steps_vec(vec![
                        EnemyStep::linear_movement_base().depth_advance(1).into(),
                        EnemyStep::idle_base().with_duration(4.).into(),
                        EnemyStep::linear_movement_base()
                            .opposite_direction()
                            .into(),
                        EnemyStep::linear_movement_base().depth_retreat(1).into(),
                    ]),
            ))
            .into(),
        DestructibleSpawn::lamp_base(260.0, OBJECT_LAMP_Y, LampDepth::Three)
            .drops(ContainerSpawn::Pickup(PickupSpawn::big_healthpack_base()))
            .into(),
        PickupSpawn::big_healthpack_base()
            .with_coordinates(Vec2::new(30., 10.))
            .into(),
    ]
}

pub fn make_steps() -> Vec<StageStep> {
    vec![
        // MovementStageStep::base(0.0, 0.0).into(),
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
        MovementStageStep::base(50.0, 0.0)
            .with_floor_depths(
                vec![
                    (Depth::Six, 70.0),
                    (Depth::Five, 50.0),
                    (Depth::Four, 30.0),
                    (Depth::Three, 0.0),
                ]
                .into_iter()
                .collect(),
            )
            .add_spawns(vec![
                EnemySpawn::mosquito_variant_circle()
                    .with_coordinates(Vec2::new(60.0, 100.0))
                    .with_elapsed(5.4)
                    .drops(PickupSpawn::big_healthpack_base().into())
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(120.0, 100.0))
                    .with_elapsed(5.1)
                    .drops(PickupSpawn::big_healthpack_base().into())
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(130.0, 70.0))
                    .drops(PickupSpawn::big_healthpack_base().into())
                    .into(),
            ])
            .into(),
        StopStageStep::new()
            .with_max_duration(12.)
            .add_spawns(vec![
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(70.0, 70.0))
                    .with_elapsed(2.4)
                    .drops(PickupSpawn::big_healthpack_base().into())
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(90.0, 70.0))
                    .with_elapsed(10.8)
                    .into(),
                EnemySpawn::mosquito_variant_circle()
                    .with_coordinates(Vec2::new(12.0, 50.0))
                    .with_speed(2.0)
                    .with_elapsed(24.8)
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(170., 68.))
                    .with_elapsed(50.8)
                    .drops(PickupSpawn::big_healthpack_base().into())
                    .into(),
            ])
            .into(),
        MovementStageStep::base(220., 0.)
            .add_spawns(vec![
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(70.0, 60.0))
                    .with_elapsed(6.8)
                    .with_steps_vec(vec![EnemyStep::circle_around_base()
                        .opposite_direction()
                        .with_radius(3.)
                        .into()])
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(12.0, 50.0))
                    .with_elapsed(22.8)
                    .into(),
                EnemySpawn::tardigrade_base()
                    .with_coordinates(Vec2::new(30.0, 90.0))
                    .with_speed(2.0)
                    .with_elapsed(22.8)
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(30.0, 50.0))
                    .with_speed(2.0)
                    .with_elapsed(22.8)
                    .into(),
                PickupSpawn::big_healthpack_base()
                    .with_coordinates(Vec2::new(144.0, 68.0))
                    .with_elapsed(0.0)
                    .into(),
            ])
            .into(),
        StopStageStep::new().with_max_duration(10.).into(),
        MovementStageStep::base(280., 0.)
            .add_spawns(vec![
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(60.0, 80.0))
                    .with_elapsed(8.4)
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(100.0, 60.0))
                    .with_elapsed(6.1)
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(30.0, 80.0))
                    .with_elapsed(7.4)
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(120.0, 60.0))
                    .with_speed(5.0)
                    .with_elapsed(6.1)
                    // direction: MovementDirection::Left,
                    // radius: 25.0,
                    // time_offset: 2.0,
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(80.0, 80.0))
                    .with_speed(5.0)
                    .with_elapsed(7.4)
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(20.0, 60.0))
                    .with_speed(5.0)
                    .with_elapsed(8.1)
                    // direction: MovementDirection::Left,
                    // radius: 25.0,
                    // time_offset: 3.0,
                    .into(),
            ])
            .into(),
        MovementStageStep::base(500.0, 0.0)
            .add_spawns(vec![
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(100.0, 80.0))
                    .with_speed(5.0)
                    .with_elapsed(8.4)
                    // direction: MovementDirection::Right,
                    // radius: 15.0,
                    // time_offset: 0.5,
                    .drops(PickupSpawn::big_healthpack_base().into())
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(100.0, 100.0))
                    .with_speed(5.0)
                    .with_elapsed(6.1)
                    // direction: MovementDirection::Left,
                    // radius: 25.0,
                    // time_offset: 3.0,
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(60.0, 120.0))
                    .with_elapsed(7.4)
                    .with_steps_vec(vec![EnemyStep::circle_around_base()
                        .opposite_direction()
                        .with_radius(13.)
                        .into()])
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(280.0, 80.0))
                    .with_elapsed(5.6)
                    .into(),
                EnemySpawn::tardigrade_base()
                    .with_elapsed(6.4)
                    .with_coordinates(Vec2::new(320.0, 160.0))
                    .drops(PickupSpawn::big_healthpack_base().into())
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(400.0, 160.0))
                    .with_elapsed(7.4)
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(360.0, 160.0))
                    .with_speed(5.0)
                    .with_elapsed(8.1)
                    .with_steps_vec(vec![
                        // direction: MovementDirection::Left,
                        // radius: 25.0,
                        // time_offset: 3.0,
                    ])
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(410.0, 180.0))
                    .with_speed(5.0)
                    .with_elapsed(7.4)
                    .with_steps_vec(vec![
                        // direction: MovementDirection::Right,
                        // radius: 15.0,
                        // time_offset: 1.0,
                    ])
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(440.0, 190.0))
                    .with_speed(5.0)
                    .with_elapsed(6.1)
                    .with_steps_vec(vec![EnemyStep::circle_around_base()
                        .with_radius(23.)
                        .into()])
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(120.0, 160.0))
                    .with_elapsed(6.1)
                    .drops(PickupSpawn::big_healthpack_base().into())
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(140.0, 160.0))
                    .with_elapsed(7.4)
                    .with_steps_vec(vec![
                        // direction: MovementDirection::Right,
                        // radius: 15.0,
                        // time_offset: 0.5,
                    ])
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(520.0, 210.0))
                    .with_speed(5.0)
                    .with_elapsed(8.1)
                    .with_steps_vec(vec![EnemyStep::circle_around_base()
                        .with_radius(13.)
                        .opposite_direction()
                        .into()])
                    .drops(PickupSpawn::big_healthpack_base().into())
                    .into(),
            ])
            .into(),
        StopStageStep::new().with_max_duration(10.).into(),
        MovementStageStep::base(850., 155.)
            .add_spawns(vec![
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(55.0, 60.0))
                    .with_speed(5.0)
                    .with_elapsed(8.4)
                    .with_steps_vec(vec![EnemyStep::circle_around_base()
                        .with_radius(15.)
                        .opposite_direction()
                        .into()])
                    .drops(PickupSpawn::big_healthpack_base().into())
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(100.0, 60.0))
                    .with_speed(5.0)
                    .with_elapsed(6.1)
                    .with_steps_vec(vec![
                        // direction: MovementDirection::Left,
                        // radius: 25.0,
                        // time_offset: 3.0,
                    ])
                    .into(),
            ])
            .into(),
        StopStageStep::new().with_max_duration(10.).into(),
        MovementStageStep::base(920., 155.)
            .add_spawns(vec![
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(60.0, 80.0))
                    .with_speed(5.0)
                    .with_elapsed(8.4)
                    .with_steps_vec(vec![])
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(100.0, 60.0))
                    .with_elapsed(6.1)
                    .into(),
            ])
            .into(),
        StopStageStep::new()
            .with_max_duration(45.)
            .with_kill_all(false)
            .with_kill_boss(true)
            .into(),
        // TODO
        // .add_spawns(Boss),
    ]
}
