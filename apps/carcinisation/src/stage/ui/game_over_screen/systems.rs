use super::{events::GameOverScreenShutdownEvent, input::GameOverScreenInput};
use crate::{
    game::{
        data::STARTING_LIVES,
        resources::{GameProgress, Lives},
        GamePlugin,
    },
    main_menu::MainMenuPlugin,
    stage::{
        data::StageData,
        events::StageRestart,
        resources::{StageActionTimer, StageProgress, StageTime},
        StagePlugin, StageProgressState,
    },
};
use activable::{activate, deactivate};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use std::time::Duration;

pub fn check_press_continue_input(
    mut screen_shutdown_event_writer: MessageWriter<GameOverScreenShutdownEvent>,
    input: Res<ActionState<GameOverScreenInput>>,
) {
    if input.just_pressed(&GameOverScreenInput::BackToMenu) {
        screen_shutdown_event_writer.write(GameOverScreenShutdownEvent);
    }
}

pub fn handle_game_over_screen_continue(
    mut event_reader: MessageReader<GameOverScreenShutdownEvent>,
    mut commands: Commands,
    mut stage_state: ResMut<NextState<StageProgressState>>,
    mut stage_progress: ResMut<StageProgress>,
    mut game_progress: ResMut<GameProgress>,
    mut lives: ResMut<Lives>,
    mut stage_time: ResMut<StageTime>,
    mut stage_action_timer: ResMut<StageActionTimer>,
    mut stage_restart_writer: MessageWriter<StageRestart>,
) {
    if event_reader.read().next().is_some() {
        stage_progress.index = 0;

        if lives.0 > 0 {
            stage_time.elapsed = Duration::ZERO;
            stage_time.delta = Duration::ZERO;
            stage_action_timer.timer.reset();
            stage_action_timer.stop();
            stage_state.set(StageProgressState::Initial);
            stage_restart_writer.write(StageRestart);
            return;
        }

        game_progress.index = 0;
        lives.0 = STARTING_LIVES;
        stage_state.set(StageProgressState::Initial);

        deactivate::<StagePlugin>(&mut commands);
        deactivate::<GamePlugin>(&mut commands);
        activate::<MainMenuPlugin>(&mut commands);

        commands.remove_resource::<StageData>();
    }
}
