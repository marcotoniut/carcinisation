pub mod setup;

use super::{
    data::DEATH_SCORE_PENALTY, events::GameOverEvent, resources::Lives, score::components::Score,
    GamePluginUpdateState,
};
use crate::stage::{components::interactive::Dead, player::components::Player};
use bevy::prelude::*;

pub fn pause_game(mut next_state: ResMut<NextState<GamePluginUpdateState>>) {
    next_state.set(GamePluginUpdateState::Active);
}

pub fn resume_game(mut next_state: ResMut<NextState<GamePluginUpdateState>>) {
    next_state.set(GamePluginUpdateState::Inactive);
}
