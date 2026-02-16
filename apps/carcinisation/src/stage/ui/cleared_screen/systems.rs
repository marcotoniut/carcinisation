use super::{input::ClearScreenInput, messages::ClearScreenShutdownMessage};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

/// @system Writes a shutdown message when the continue input fires on the clear screen.
pub fn check_press_continue_input(
    mut screen_shutdown_event_writer: MessageWriter<ClearScreenShutdownMessage>,
    // mut main_menu_event_writer: MessageWriter<MainMenuShutdownEvent>,
    input: Res<ActionState<ClearScreenInput>>,
) {
    if input.just_pressed(&ClearScreenInput::Continue) {
        screen_shutdown_event_writer.write(ClearScreenShutdownMessage);
        // main_menu_event_writer.write(MainMenuShutdownEvent);
    }
}
