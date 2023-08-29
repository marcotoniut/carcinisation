pub mod enemy;
pub mod player;
pub mod score;
pub mod star;
pub mod systems;
pub mod ui;

use bevy::prelude::*;

use self::{
    enemy::EnemyPlugin,
    player::PlayerPlugin,
    score::{components::Score, ScorePlugin},
    star::StarPlugin,
    systems::*,
    ui::StageUiPlugin,
};
use crate::{events::*, AppState};

pub struct StagePlugin;

impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameState>()
            .add_event::<GameOver>()
            .init_resource::<Score>()
            .add_systems(OnEnter(AppState::Game), pause_game)
            .add_plugins(EnemyPlugin)
            .add_plugins(PlayerPlugin)
            .add_plugins(ScorePlugin)
            .add_plugins(StageUiPlugin)
            .add_plugins(StarPlugin)
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
