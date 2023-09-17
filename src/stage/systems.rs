use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use seldom_pixel::{prelude::PxAssets, sprite::PxSprite};

use crate::{
    game::resources::{StageData, StageDataHandle},
    GBInput,
};

use super::{
    bundles::make_current_stage_bundle,
    resources::{GameProgress, StageTimer},
    GameState,
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
    let stage_data_handle = StageDataHandle(asset_server.load("stages/asteroid.toml"));
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
    let bundle = make_current_stage_bundle(&mut assets_sprite, &data_handle, &data, &game_progress);
    commands.spawn(bundle.unwrap());
    state.set(GameState::Running);
}
