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
use bevy::{asset::AssetContainer, prelude::*};

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
    game_progress: Res<GameProgress>,
    game_data: Res<GameData>,
    asset_server: Res<AssetServer>,
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
                    asset_server.load::<CutsceneData>(src);
                    // let data = Arc::new();
                    // cinematic_startup_event_writer.send(CutsceneStartupEvent { data });
                }
                GameStep::Transition(TransitionGameStep {}) => {
                    // TODO
                }
            }
        }
    }
}

#[derive(Resource)]
pub struct CutsceneAssetHandle {
    handle: Handle<CutsceneData>,
}

pub fn check_cutscene_data_loaded(
    asset_handle: Res<CutsceneAssetHandle>,
    cutscene_data_assets: Res<Assets<CutsceneData>>,
    mut cinematic_startup_event_writer: EventWriter<CutsceneStartupEvent>,
) {
    if let Some(cutscene_data) = cutscene_data_assets.get(&asset_handle.handle) {
        // Asset is loaded, you can now use cutscene_data
        println!("Cutscene data loaded: {:?}", cutscene_data);
        cinematic_startup_event_writer.send(CutsceneStartupEvent {
            // TODO do I need Arc for this? Can it not be handled by a simple pointer reference?
            data: Arc::new(cutscene_data.clone()),
        });
    } else {
        // Asset is not yet loaded
        println!("Cutscene data is still loading...");
    }
}
