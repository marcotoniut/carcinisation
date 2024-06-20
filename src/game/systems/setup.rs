use std::sync::Arc;

use crate::{
    cutscene::{
        data::CutsceneData,
        events::{CutsceneShutdownEvent, CutsceneStartupEvent},
        CutscenePluginUpdateState,
    },
    game::{
        components::steps::*, data::*, events::GameStartupEvent, resources::*,
        GamePluginUpdateState,
    },
    progression::game::GAME_DATA,
    stage::{
        data::StageData,
        events::{StageClearedEvent, StageStartupEvent},
        StagePluginUpdateState,
    },
};
use bevy::prelude::*;

pub fn on_startup(
    mut event_reader: EventReader<GameStartupEvent>,
    mut next_state: ResMut<NextState<GamePluginUpdateState>>,
    mut commands: Commands,
) {
    for _ in event_reader.read() {
        next_state.set(GamePluginUpdateState::Active);
        commands.insert_resource::<GameProgress>(GameProgress { index: 0 });
        commands.insert_resource::<GameData>(GAME_DATA.clone());
        commands.insert_resource(Lives(STARTING_LIVES));
    }
}

// TODO Should I use stage_shutdown instead?
pub fn on_stage_cleared(
    mut event_reader: EventReader<StageClearedEvent>,
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

pub fn on_cutscene_shutdown(
    mut event_reader: EventReader<CutsceneShutdownEvent>,
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

pub fn progress(
    asset_server: Res<AssetServer>,
    game_progress: Res<GameProgress>,
    game_data: Res<GameData>,
    mut commands: Commands,
    mut stage_startup_event_writer: EventWriter<StageStartupEvent>,
) {
    if game_progress.is_added() || game_progress.is_changed() {
        if let Some(data) = game_data.steps.get(game_progress.index) {
            match data {
                GameStep::Stage(StageGameStep { data }) => {
                    stage_startup_event_writer.send(StageStartupEvent { data: data.clone() });
                }
                GameStep::Credits(CreditsGameStep {}) => {
                    // TODO
                }
                GameStep::Cinematic(CinematicGameStep { src, is_checkpoint }) => {
                    let handle = asset_server.load::<CutsceneData>(src);
                    commands.insert_resource(CutsceneAssetHandle { handle });
                }
                GameStep::Transition(TransitionGameStep {}) => {
                    // TODO
                }
            }
        }
    }
}

pub fn check_cutscene_data_loaded(
    cutscene_asset_handle: Res<CutsceneAssetHandle>,
    cutscene_data_assets: Res<Assets<CutsceneData>>,
    mut cinematic_startup_event_writer: EventWriter<CutsceneStartupEvent>,
    mut commands: Commands,
) {
    if let Some(cutscene_data) = cutscene_data_assets.get(&cutscene_asset_handle.handle) {
        println!("Cutscene data loaded: {:?}", cutscene_data);
        cinematic_startup_event_writer.send(CutsceneStartupEvent {
            // TODO do I need Arc for this? Can it not be handled by a simple pointer reference?
            data: Arc::new(cutscene_data.clone()),
        });
        commands.remove_resource::<CutsceneAssetHandle>();
    } else {
        // Asset is not yet loaded
        println!("Cutscene data is still loading...");
    }
}
