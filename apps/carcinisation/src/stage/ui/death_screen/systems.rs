use super::{input::DeathScreenInput, messages::DeathScreenRestartMessage};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

pub fn check_press_continue_input(
    mut screen_shutdown_event_writer: MessageWriter<DeathScreenRestartMessage>,
    input: Res<ActionState<DeathScreenInput>>,
) {
    if input.just_pressed(&DeathScreenInput::Restart) {
        screen_shutdown_event_writer.write(DeathScreenRestartMessage);
    }
}
