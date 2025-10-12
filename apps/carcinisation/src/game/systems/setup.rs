//! Startup/shutdown handling and progression logic for the main game loop.

use std::sync::Arc;

use crate::{
    cutscene::{
        data::CutsceneData,
        events::{CutsceneShutdownTrigger, CutsceneStartupTrigger},
        CutscenePluginUpdateState,
    },
    debug::plugin::{debug_print_shutdown, debug_print_startup},
    game::{
        components::steps::*, data::*, events::GameStartupTrigger, resources::*, GameOverTrigger,
        GamePluginUpdateState,
    },
    progression::game::GAME_DATA,
    stage::{
        data::StageData,
        events::{StageClearedTrigger, StageStartupTrigger},
        StagePluginUpdateState,
    },
};
use bevy::prelude::*;

const DEBUG_MODULE: &str = "Game";

/// @trigger Initialises game resources and enables the progression plugin.
pub fn on_game_startup(
    _trigger: Trigger<GameStartupTrigger>,
    mut next_state: ResMut<NextState<GamePluginUpdateState>>,
    mut commands: Commands,
) {
    #[cfg(debug_assertions)]
    debug_print_startup(DEBUG_MODULE);

    next_state.set(GamePluginUpdateState::Active);
    commands.insert_resource::<GameProgress>(GameProgress { index: 0 });
    commands.insert_resource::<GameData>(GAME_DATA.clone());
    commands.insert_resource(Lives(STARTING_LIVES));
}

// pub fn on_game_shutdown(
//     _trigger: Trigger<GameShutdownTrigger>,
//     mut next_state: ResMut<NextState<GamePluginUpdateState>>,
// ) {
//     #[cfg(debug_assertions)]
//     debug_print_shutdown(DEBUG_MODULE);

//     next_state.set(GamePluginUpdateState::Inactive);
// }

/// @trigger Placeholder hook for future game-over cleanup.
pub fn on_game_over(_trigger: Trigger<GameOverTrigger>) {}

/// @trigger Advances progress when a stage reports it has been cleared.
pub fn on_stage_cleared(
    mut event_reader: EventReader<StageClearedTrigger>,
    mut commands: Commands,
    mut next_update_state: ResMut<NextState<StagePluginUpdateState>>,
    mut progress: ResMut<GameProgress>,
) {
    for _ in event_reader.read() {
        progress.index += 1;
        next_update_state.set(StagePluginUpdateState::Inactive);
        commands.remove_resource::<StageData>();
    }
}

/// @trigger Advances progress when a cutscene finishes.
pub fn on_cutscene_shutdown(
    mut event_reader: EventReader<CutsceneShutdownTrigger>,
    mut commands: Commands,
    mut next_update_state: ResMut<NextState<CutscenePluginUpdateState>>,
    mut progress: ResMut<GameProgress>,
) {
    for _ in event_reader.read() {
        progress.index += 1;
        // TODO should this be handled inside of the plugin instead?
        next_update_state.set(CutscenePluginUpdateState::Inactive);
        commands.remove_resource::<CutsceneData>();
    }
}

/// @system Reacts to game progression changes, triggering the next step.
pub fn progress(
    asset_server: Res<AssetServer>,
    game_progress: Res<GameProgress>,
    game_data: Res<GameData>,
    mut commands: Commands,
    // mut cutscene_startup_event_writer: EventWriter<CutsceneStartupEvent>,
    mut stage_startup_event_writer: EventWriter<StageStartupTrigger>,
) {
    if game_progress.is_added() || game_progress.is_changed() {
        if let Some(data) = game_data.steps.get(game_progress.index) {
            match data {
                GameStep::Credits(CreditsGameStep {}) => {
                    // TODO
                }
                GameStep::Cutscene(CutsceneGameStep {
                    data,
                    is_checkpoint,
                }) => {
                    commands.trigger(CutsceneStartupTrigger { data: data.clone() });
                    // cutscene_startup_event_writer.send();
                }
                GameStep::CutsceneAsset(CinematicAssetGameStep { src, is_checkpoint }) => {
                    commands.insert_resource(CutsceneAssetHandle {
                        handle: asset_server.load::<CutsceneData>(src),
                    });
                }
                GameStep::Stage(StageGameStep { data }) => {
                    stage_startup_event_writer.send(StageStartupTrigger { data: data.clone() });
                }
                GameStep::StageAsset(StageAssetGameStep(src)) => {
                    commands.insert_resource(StageAssetHandle {
                        handle: asset_server.load::<StageData>(src),
                    });
                }
                GameStep::Transition(TransitionGameStep {}) => {
                    // TODO
                }
            }
        }
    }
}

/// @system Triggers cutscene startup once the associated asset finishes loading.
pub fn check_cutscene_data_loaded(
    asset_handle: Res<CutsceneAssetHandle>,
    data_assets: Res<Assets<CutsceneData>>,
    mut commands: Commands,
) {
    if let Some(data) = data_assets.get(&asset_handle.handle) {
        #[cfg(debug_assertions)]
        println!("Cutscene data loaded: {:?}", data);
        commands.remove_resource::<CutsceneAssetHandle>();
        commands.trigger(CutsceneStartupTrigger {
            // TODO do I need Arc for this? Can it not be handled by a simple pointer reference?
            data: Arc::new(data.clone()),
        });
    } else {
        #[cfg(debug_assertions)]
        println!("Cutscene data is still loading...");
    }
}

/// @system Triggers stage startup once the associated asset finishes loading.
pub fn check_stage_data_loaded(
    asset_handle: Res<StageAssetHandle>,
    data_assets: Res<Assets<StageData>>,
    mut commands: Commands,
) {
    if let Some(data) = data_assets.get(&asset_handle.handle) {
        #[cfg(debug_assertions)]
        println!("Stage data loaded: {:?}", data);
        commands.remove_resource::<StageAssetHandle>();
        commands.trigger(StageStartupTrigger {
            // TODO do I need Arc for this? Can it not be handled by a simple pointer reference?
            data: Arc::new(data.clone()),
        });
    } else {
        #[cfg(debug_assertions)]
        println!("Stage data is still loading...");
    }
}
