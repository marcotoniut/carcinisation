pub mod player;
pub mod systems;

use bevy::prelude::*;

use self::{player::PlayerPlugin, systems::*};
use crate::{events::*, AppState};

pub struct StagePlugin;

impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameState>()
            .add_event::<GameOver>()
            .add_systems(OnEnter(AppState::Game), pause_game)
            .add_plugins(PlayerPlugin)
            .add_systems(Update, toggle_game.run_if(in_state(AppState::Game)))
            .add_systems(OnEnter(AppState::Game), resume_game);
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Running,
    Paused,
}