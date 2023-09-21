use std::ops::{Mul, Sub};

use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use seldom_pixel::{
    prelude::{PxAssets, PxCamera, PxSubPosition},
    sprite::PxSprite,
};

use crate::{
    game::resources::{StageAction, StageData, StageDataHandle},
    systems::camera::CameraPos,
    GBInput,
};

use super::{
    bundles::*,
    resources::{GameProgress, StageTimer},
    GameState, StageState,
};

pub fn pause_game(mut game_state_next_state: ResMut<NextState<GameState>>) {
    game_state_next_state.set(GameState::Paused);
}

pub fn resume_game(mut game_state_next_state: ResMut<NextState<GameState>>) {
    game_state_next_state.set(GameState::Running);
}

pub fn toggle_game(
    gb_input_query: Query<&ActionState<GBInput>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let gb_input = gb_input_query.single();
    if gb_input.just_pressed(GBInput::Start) {
        if state.get().to_owned() == GameState::Running {
            next_state.set(GameState::Paused);
            println!("Game Paused.");
        } else {
            next_state.set(GameState::Running);
            println!("Game Running.");
        }
    }
}

pub fn run_timer(res: Res<StageTimer>, game_data: Res<Assets<StageData>>) {
    // let (_, data) = game_data.iter().next().unwrap();
}

pub fn setup_stage(mut commands: Commands, asset_server: Res<AssetServer>) {
    let stage_data_handle = StageDataHandle(asset_server.load("stages/asteroid.yaml"));
    
    commands.insert_resource(stage_data_handle);
}

pub fn spawn_current_stage_bundle(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    data_handle: Res<StageDataHandle>,
    data: Res<Assets<StageData>>,
    game_progress: Res<GameProgress>,
    mut state: ResMut<NextState<GameState>>,
) {
    let background_bundle =
        make_background_bundle(&mut assets_sprite, &data_handle, &data, &game_progress);
    let skybox_bundle = make_skybox_bundle(&mut assets_sprite, &data_handle, &data, &game_progress);
    commands.spawn(Name::new("Stage")).with_children(|parent| {
        if let Some(background_bundle_data) = background_bundle {
            parent.spawn(background_bundle_data);
        }
        if let Some(skybox_bundle_data) = skybox_bundle {
            parent.spawn(skybox_bundle_data);
        }
    });
    state.set(GameState::Running);
}

// pub fn init_stage_state(
//     mut commands: Commands,
//     mut assets_sprite: PxAssets<PxSprite>,
//     enemy_spawn_timer: Res<EnemySpawnTimer>,
// ) {
// }

pub fn check_timer(
    mut timer: ResMut<StageTimer>,
    data: Res<Assets<StageData>>,
    data_handle: Res<StageDataHandle>,
    game_progress: Res<GameProgress>,
) {
    if timer.timer.finished() {
        let stage = game_progress.stage_step;

        // game_progress.stage_step += 1;

        let handle_stage_data = data_handle.0.clone();
        if let Some(stage) = data.get(&handle_stage_data) {
            timer.timer.reset();
        }
    }
}

pub fn update_stage(
    state: Res<State<StageState>>,
    mut next_state: ResMut<NextState<StageState>>,
    mut camera_pos_query: Query<&mut PxSubPosition, With<CameraPos>>,
    mut camera: ResMut<PxCamera>,
    mut game_progress: ResMut<GameProgress>,
    time: Res<Time>,
    data: Res<Assets<StageData>>,
    data_handle: Res<StageDataHandle>,
) {
    match state.to_owned() {
        StageState::Initial => {
            next_state.set(StageState::Running);
        }
        StageState::Running => {
            let handle_stage_data = data_handle.0.clone();
            if let Some(stage) = data.get(&handle_stage_data) {
                if let Some(action) = stage.actions.get(game_progress.stage_step) {
                    match action {
                        StageAction::Movement {
                            coordinates,
                            base_speed,
                        } => {
                            let mut camera_pos = camera_pos_query.single_mut();

                            let origin = camera_pos.0;

                            let direction = coordinates.sub(origin).normalize();

                            **camera_pos += (time.delta_seconds() * base_speed) * direction;
                            if (direction.x > 0.0 && camera_pos.0.x > coordinates.x)
                                || (direction.x < 0.0 && camera_pos.0.x < coordinates.x)
                            {
                                let x = PxSubPosition(coordinates.clone());
                                *camera_pos = x;
                                game_progress.stage_step += 1;
                            }

                            **camera = camera_pos.round().as_ivec2();
                        }
                        StageAction::Stop {
                            condition,
                            max_duration,
                        } => {}
                    }
                }
            }
        }
    }
}
