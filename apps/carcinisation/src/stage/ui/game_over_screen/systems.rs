use super::{input::GameOverScreenInput, messages::GameOverScreenShutdownMessage};
use crate::{
    game::{
        GamePlugin,
        data::STARTING_LIVES,
        resources::{GameProgress, Lives},
        score::components::Score,
    },
    main_menu::MainMenuPlugin,
    stage::{
        StagePlugin, StageProgressState,
        components::{Stage, StageEntity},
        data::StageData,
        resources::StageProgress,
        restart::despawn_stage_entities,
    },
};
use activable::{activate, deactivate};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

/// @system Writes a shutdown message when the back-to-menu input fires.
pub fn check_press_continue_input(
    mut screen_shutdown_event_writer: MessageWriter<GameOverScreenShutdownMessage>,
    input: Res<ActionState<GameOverScreenInput>>,
) {
    if input.just_pressed(&GameOverScreenInput::BackToMenu) {
        screen_shutdown_event_writer.write(GameOverScreenShutdownMessage);
    }
}

/// @system Returns to the main menu on game over confirmation.
///
/// Resets lives, progress, and plugin activation state so a fresh run
/// starts from the beginning.
#[allow(clippy::too_many_arguments)]
pub fn handle_game_over_screen_continue(
    mut event_reader: MessageReader<GameOverScreenShutdownMessage>,
    mut commands: Commands,
    mut stage_state: ResMut<NextState<StageProgressState>>,
    mut stage_progress: ResMut<StageProgress>,
    mut game_progress: ResMut<GameProgress>,
    mut lives: ResMut<Lives>,
    mut score: ResMut<Score>,
    stage_query: Query<Entity, With<Stage>>,
    stage_entity_query: Query<Entity, With<StageEntity>>,
) {
    if event_reader.read().next().is_some() {
        stage_progress.index = 0;
        game_progress.index = 0;
        lives.0 = STARTING_LIVES;
        score.value = 0;
        stage_state.set(StageProgressState::Initial);

        // Despawn all stage-owned entities (HUD, background, music, etc.)
        // before deactivating plugins and returning to the main menu.
        despawn_stage_entities(&mut commands, &stage_query, &stage_entity_query);

        deactivate::<StagePlugin>(&mut commands);
        deactivate::<GamePlugin>(&mut commands);
        activate::<MainMenuPlugin>(&mut commands);

        commands.remove_resource::<StageData>();
    }
}
