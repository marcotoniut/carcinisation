use super::{input::GameOverScreenInput, messages::GameOverScreenShutdownMessage};
use crate::{
    game::{
        GamePlugin,
        data::STARTING_LIVES,
        resources::{GameProgress, Lives},
    },
    main_menu::MainMenuPlugin,
    stage::{
        StagePlugin, StageProgressState,
        data::StageData,
        messages::StageRestart,
        resources::{StageActionTimer, StageProgress, StageTimeDomain},
    },
};
use activable::{activate, deactivate};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

pub fn check_press_continue_input(
    mut screen_shutdown_event_writer: MessageWriter<GameOverScreenShutdownMessage>,
    input: Res<ActionState<GameOverScreenInput>>,
) {
    if input.just_pressed(&GameOverScreenInput::BackToMenu) {
        screen_shutdown_event_writer.write(GameOverScreenShutdownMessage);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_game_over_screen_continue(
    mut event_reader: MessageReader<GameOverScreenShutdownMessage>,
    mut commands: Commands,
    mut stage_state: ResMut<NextState<StageProgressState>>,
    mut stage_progress: ResMut<StageProgress>,
    mut game_progress: ResMut<GameProgress>,
    mut lives: ResMut<Lives>,
    mut stage_time: ResMut<Time<StageTimeDomain>>,
    mut stage_action_timer: ResMut<StageActionTimer>,
    mut stage_restart_writer: MessageWriter<StageRestart>,
) {
    if event_reader.read().next().is_some() {
        stage_progress.index = 0;

        if lives.0 > 0 {
            *stage_time = Time::default();
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
