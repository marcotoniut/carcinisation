pub mod components;
pub mod data;
pub mod events;
pub mod resources;
pub mod score;
mod systems;

use crate::core::event::on_trigger_write_event;

use self::{events::*, resources::GameProgress, score::ScorePlugin, systems::setup::*};
use bevy::prelude::*;
use resources::{CutsceneAssetHandle, StageAssetHandle};
use systems::debug::debug_on_game_over;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ScorePlugin)
            .init_state::<GamePluginUpdateState>()
            .init_state::<GameProgressState>()
            .add_event::<GameOverTrigger>()
            .observe(on_game_over)
            .observe(on_trigger_write_event::<GameOverTrigger>)
            .add_event::<GameStartupTrigger>()
            .observe(on_game_startup)
            .add_systems(
                Update,
                ((
                    check_cutscene_data_loaded.run_if(resource_exists::<CutsceneAssetHandle>),
                    check_stage_data_loaded.run_if(resource_exists::<StageAssetHandle>),
                    progress,
                    on_stage_cleared,
                    on_cutscene_shutdown,
                )
                    .run_if(resource_exists::<GameProgress>),)
                    .run_if(in_state(GamePluginUpdateState::Active)),
            );

        #[cfg(debug_assertions)]
        {
            app.observe(debug_on_game_over);
        }
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
