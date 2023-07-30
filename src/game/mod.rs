pub mod enemy;
pub mod player;
pub mod score;
pub mod star;
pub mod systems;
pub mod ui;

use bevy::prelude::*;

use self::{
    enemy::EnemyPlugin, player::PlayerPlugin, score::ScorePlugin, star::StarPlugin, systems::*,
    ui::GameUiPlugin,
};
use crate::{events::*, AppState};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameState>()
            .add_event::<GameOver>()
            .add_systems(OnEnter(AppState::Game), pause_game)
            .add_plugins(EnemyPlugin)
            .add_plugins(PlayerPlugin)
            .add_plugins(ScorePlugin)
            .add_plugins(StarPlugin)
            .add_plugins(GameUiPlugin)
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
