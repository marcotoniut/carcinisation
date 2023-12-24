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
    game_progress: Res<GameProgress>,
    game_data: Res<GameData>,
    mut stage_startup_event_writer: EventWriter<StageStartupEvent>,
    mut cinematic_startup_event_writer: EventWriter<CutsceneStartupEvent>,
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
                GameStep::Cinematic(CinematicGameStep {
                    data,
                    is_checkpoint,
                }) => {
                    cinematic_startup_event_writer
                        .send(CutsceneStartupEvent { data: data.clone() });
                }
                GameStep::Transition(TransitionGameStep {}) => {
                    // TODO
                }
            }
        }
    }
}
