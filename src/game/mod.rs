pub mod resources;
pub mod systems;

use bevy::prelude::*;

use self::systems::*;
use crate::{events::*, AppState};
use resources::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        // let data = init_stages_resource();

        app.add_state::<GameState>()
            .add_event::<GameOver>()
            //.add_systems(Update, toggle_game.run_if(in_state(AppState::Game)))
            .add_systems(OnEnter(AppState::Game), resume_game);
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Running,
    Paused,
}
