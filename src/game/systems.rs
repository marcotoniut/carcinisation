use super::GamePluginUpdateState;
use crate::stage::StagePluginUpdateState;
use bevy::prelude::*;

pub fn pause_game(mut game_state_next_state: ResMut<NextState<GamePluginUpdateState>>) {
    game_state_next_state.set(GamePluginUpdateState::Active);
}

pub fn resume_game(mut game_state_next_state: ResMut<NextState<GamePluginUpdateState>>) {
    game_state_next_state.set(GamePluginUpdateState::Inactive);
}

pub fn start_stage(mut stage_state_next_state: ResMut<NextState<StagePluginUpdateState>>) {
    stage_state_next_state.set(StagePluginUpdateState::Active);
}
