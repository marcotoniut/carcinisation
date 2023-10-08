pub mod data;
pub mod events;
pub mod resources;
pub mod score;
pub mod systems;

use bevy::prelude::*;

use self::{
    events::*,
    score::{components::Score, ScorePlugin},
    systems::*,
};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ScorePlugin)
            .add_state::<GamePluginUpdateState>()
            .init_resource::<Score>()
            .add_state::<GameProgressState>()
            // DEBUG
            .add_event::<GameOverEvent>()
            .add_event::<GameStartupEvent>()
            .add_systems(PreUpdate, on_startup)
            .add_systems(
                Update,
                check_player_died
                    .run_if(in_state(GamePluginUpdateState::Active))
                    .run_if(in_state(GameProgressState::Running)),
            );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum GameProgressState {
    #[default]
    Loading,
    Running,
    Paused,
    Cutscene,
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum GamePluginUpdateState {
    #[default]
    Inactive,
    Active,
}
