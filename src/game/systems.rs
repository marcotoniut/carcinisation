use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use crate::GBInput;

use super::GameState;

pub fn pause_game(mut game_state_next_state: ResMut<NextState<GameState>>) {
    game_state_next_state.set(GameState::Paused);
}

pub fn resume_game(mut game_state_next_state: ResMut<NextState<GameState>>) {
    game_state_next_state.set(GameState::Running);
}
