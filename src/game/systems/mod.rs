pub mod debug;
pub mod setup;

use super::GamePluginUpdateState;
use bevy::prelude::*;

pub fn pause_game(mut next_state: ResMut<NextState<GamePluginUpdateState>>) {
    next_state.set(GamePluginUpdateState::Active);
}

pub fn resume_game(mut next_state: ResMut<NextState<GamePluginUpdateState>>) {
    next_state.set(GamePluginUpdateState::Inactive);
}
