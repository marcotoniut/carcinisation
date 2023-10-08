use crate::game::GamePluginUpdateState;
use bevy::prelude::*;

pub fn start_game(mut game_state_next_state: ResMut<NextState<GamePluginUpdateState>>) {
    game_state_next_state.set(GamePluginUpdateState::Active);
}
