use super::{input::DeathScreenInput, messages::DeathScreenRestartMessage};
use crate::stage::messages::StageRestart;
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

/// @system Writes a restart message when the restart input fires on the death screen.
pub fn check_press_continue_input(
    mut screen_shutdown_event_writer: MessageWriter<DeathScreenRestartMessage>,
    input: Res<ActionState<DeathScreenInput>>,
) {
    if input.just_pressed(&DeathScreenInput::Restart) {
        screen_shutdown_event_writer.write(DeathScreenRestartMessage);
    }
}

/// @system Translates a death-screen continue press into a checkpoint restart.
pub fn handle_death_screen_continue(
    mut event_reader: MessageReader<DeathScreenRestartMessage>,
    mut restart_writer: MessageWriter<StageRestart>,
) {
    for _ in event_reader.read() {
        restart_writer.write(StageRestart {
            from_checkpoint: true,
        });
    }
}
