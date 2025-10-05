//! Helper systems for toggling the game progression state.

pub mod debug;
pub mod setup;

use super::GamePluginUpdateState;
use bevy::prelude::*;

/// @system Places the game plugin into the active state (placeholder).
pub fn pause_game(mut next_state: ResMut<NextState<GamePluginUpdateState>>) {
    next_state.set(GamePluginUpdateState::Active);
}

/// @system Places the game plugin into the inactive state (placeholder).
pub fn resume_game(mut next_state: ResMut<NextState<GamePluginUpdateState>>) {
    next_state.set(GamePluginUpdateState::Inactive);
}
