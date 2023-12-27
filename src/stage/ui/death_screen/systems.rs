use super::{events::DeathScreenRestartEvent, input::DeathScreenInput};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

pub fn check_press_continue_input(
    mut screen_shutdown_event_writer: EventWriter<DeathScreenRestartEvent>,
    input: Res<ActionState<DeathScreenInput>>,
) {
    if input.just_pressed(DeathScreenInput::Restart) {
        screen_shutdown_event_writer.send(DeathScreenRestartEvent);
    }
}
