//! Game progression plugin: orchestrates stages, cutscenes, and score handling.

pub mod components;
pub mod data;
pub mod messages;
pub mod resources;
pub mod score;
mod systems;

use crate::core::event::on_trigger_write_event;
use activable::{Activable, ActivableAppExt};

use self::{messages::*, resources::GameProgress, score::ScorePlugin, systems::setup::*};
use bevy::prelude::*;
use resources::{CutsceneAssetHandle, StageAssetHandle};
use systems::debug::debug_on_game_over;

/// Registers the high-level game state machine and supporting systems.
#[derive(Activable)]
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ScorePlugin)
            .init_state::<GameProgressState>()
            .add_message::<GameOverEvent>()
            .add_observer(on_game_over)
            .add_observer(on_trigger_write_event::<GameOverEvent>)
            .add_message::<GameStartupEvent>()
            .add_observer(on_game_startup)
            .add_active_systems::<GamePlugin, _>(
                // Core progression loop: wait for assets, advance steps, react to stage events.
                ((
                    check_cutscene_data_loaded.run_if(resource_exists::<CutsceneAssetHandle>),
                    check_stage_data_loaded.run_if(resource_exists::<StageAssetHandle>),
                    progress,
                    on_stage_cleared,
                    on_cutscene_shutdown,
                )
                    .run_if(resource_exists::<GameProgress>),),
            );

        #[cfg(debug_assertions)]
        {
            app.add_observer(debug_on_game_over);
        }
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
/// Coarse game states used to drive menus/cutscenes/stage logic.
pub enum GameProgressState {
    #[default]
    Loading,
    Running,
    Paused,
    Cutscene,
}
