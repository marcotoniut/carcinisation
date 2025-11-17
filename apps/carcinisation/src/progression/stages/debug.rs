use std::sync::Arc;

use crate::stage::components::placement::Depth;
use crate::stage::components::{StopStageStep, TweenStageStep};
use crate::stage::data::*;
use crate::stage::destructible::data::{DestructibleSpawn, LampDepth, TrashcanDepth};
use crate::stage::enemy::data::steps::EnemyStep;
use crate::stage::enemy::entity::EnemyType;
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use lazy_static::lazy_static;

const OBJECT_FIBERTREE_Y: f32 = 13.;
const OBJECT_LAMP_Y: f32 = -5.;

lazy_static! {
    pub static ref STAGE_DEBUG_DATA: Arc<StageData> = StageData {
        name: "Debug".to_string(),
        music_path: assert_assets_path!("audio/music/stage_1.ogg").to_string(),
        background_path: assert_assets_path!("backgrounds/rugpark/background.png").to_string(),
        skybox: SkyboxData {
            path: assert_assets_path!("backgrounds/rugpark/skybox.png").to_string(),
            frames: 2,
        },
        start_coordinates: Vec2::new(0.0, 0.0),
        spawns: make_spawns(),
        steps: make_steps(),
        on_start_transition_o: None,
        on_end_transition_o: None,
    }
    .into();
}

pub fn make_spawns() -> Vec<StageSpawn> {
    vec![
        ObjectSpawn::rugpark_sign_base(400., 20.).into(),
        DestructibleSpawn::trashcan_base(100., 67., TrashcanDepth::Six).into(),
        DestructibleSpawn::trashcan_base(220., 67., TrashcanDepth::Six).into(),
        // DestructibleSpawn::crystal_base(125., 32.).into(),
        // DestructibleSpawn::mushroom_base(60., 12.).into(),
        ObjectSpawn::fibertree_base(30., OBJECT_FIBERTREE_Y).into(),
        ObjectSpawn::fibertree_base(180., OBJECT_FIBERTREE_Y).into(),
        ObjectSpawn::bench_big_base(50., 65.).into(),
        ObjectSpawn::bench_big_base(200., 60.).into(),
        DestructibleSpawn::lamp_base(75., OBJECT_LAMP_Y, LampDepth::Three)
            .drops(
                EnemyDropSpawn {
                    enemy_type: EnemyType::Mosquito,
                    contains: None,
                    steps: vec![
                        EnemyStep::idle_base().with_duration(3.).into(),
                        EnemyStep::attack_base().with_duration(1.).into(),
                    ]
                    .into(),
                    ..default()
                }
                .into(),
            )
            .into(),
        DestructibleSpawn::lamp_base(260., OBJECT_LAMP_Y, LampDepth::Three)
            .drops(PickupDropSpawn::new(PickupType::BigHealthpack).into())
            .into(),
    ]
}

pub fn make_steps() -> Vec<StageStep> {
    vec![
        TweenStageStep::base(0.0, 0.0).into(),
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
        StopStageStep::new()
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
            .with_max_duration(12.)
            .add_spawns(vec![
                EnemySpawn::tardigrade_base()
                    .with_coordinates(Vec2::new(40.0, 70.0))
                    .into(),
                EnemySpawn::mosquito_base()
                    .with_coordinates(Vec2::new(60.0, 100.0))
                    .with_elapsed_f32(0.1)
                    .with_steps_vec(vec![
                        EnemyStep::linear_movement_base()
                            .with_direction(-1., -0.2)
                            .with_trayectory(30.)
                            .depth_advance(2)
                            .into(),
                        EnemyStep::idle_base().with_duration(3.).into(),
                        EnemyStep::linear_movement_base()
                            .with_direction(1., -0.5)
                            .with_trayectory(50.)
                            .depth_retreat(1)
                            .into(),
                        EnemyStep::linear_movement_base()
                            .opposite_direction()
                            .into(),
                    ])
                    .drops(PickupDropSpawn::new(PickupType::SmallHealthpack).into())
                    .into(),
                EnemySpawn::mosquito_variant_circle()
                    .with_coordinates(Vec2::new(30.0, 60.0))
                    .into(),
                // EnemySpawn::tardigrade_base()
                //     .with_coordinates(Vec2::new(90.0, 50.0))
                //     .with_elapsed(34.)
                //     .into(),
                // EnemySpawn::tardigrade_base()
                //     .with_coordinates(Vec2::new(120.0, 30.0))
                //     .into(),
                EnemySpawn::mosquito_variant_linear()
                    .with_y(30.)
                    .with_elapsed_f32(85.0)
                    .add_step(EnemyStep::circle_around_base().into())
                    .drops(PickupDropSpawn::new(PickupType::BigHealthpack).into())
                    .into(),
                EnemySpawn::mosquito_variant_linear_opposite()
                    .with_elapsed_f32(45.1)
                    .drops(PickupDropSpawn::new(PickupType::SmallHealthpack).into())
                    .into(),
                EnemySpawn::mosquito_variant_approacher()
                    .with_coordinates(Vec2::new(140.0, 130.0))
                    .with_elapsed_f32(2.1)
                    .drops(PickupDropSpawn::new(PickupType::BigHealthpack).into())
                    .into(),
            ])
            .into(),
        TweenStageStep::base(100.0, 0.0)
            .add_spawns(vec![
                EnemySpawn::mosquito_variant_approacher()
                    .with_coordinates(Vec2::new(140.0, 130.0))
                    .with_elapsed_f32(2.1)
                    .drops(PickupDropSpawn::new(PickupType::BigHealthpack).into())
                    .into(),
                // EnemySpawn::mosquito_variant_circle()
                //     .with_coordinates(Vec2::new(60.0, 70.0))
                //     .with_elapsed(23.8)
                //     .into(),
                // EnemySpawn::mosquito_base()
                //     .with_coordinates(Vec2::new(130.0, 70.0))
                //     .with_elapsed(1.8)
                //     .drops(PickupSpawn::big_healthpack_base().into())
                //     .into(),
            ])
            .into(),
        StopStageStep::new()
            .with_max_duration(15.)
            .add_spawns(vec![EnemySpawn::mosquito_base()
                .with_coordinates(Vec2::new(130.0, 70.0))
                .with_elapsed_f32(35.)
                .drops(PickupDropSpawn::new(PickupType::BigHealthpack).into())
                .into()])
            .into(),
        StopStageStep::new().with_max_duration(100.).into(),
    ]
}
