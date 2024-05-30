pub mod components;
pub mod data;
pub mod events;
pub mod resources;
pub mod score;
pub mod systems;

use self::{events::*, resources::GameProgress, score::ScorePlugin, systems::setup::*};
use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ScorePlugin)
            .init_state::<GamePluginUpdateState>()
            .init_state::<GameProgressState>()
            // DEBUG
            .add_event::<GameOverEvent>()
            .add_event::<GameStartupEvent>()
            .add_systems(PreUpdate, on_startup)
            .add_systems(
                Update,
                ((progress, on_stage_cleared, on_cutscene_shutdown)
                    .run_if(resource_exists::<GameProgress>),)
                    .run_if(in_state(GamePluginUpdateState::Active)),
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
