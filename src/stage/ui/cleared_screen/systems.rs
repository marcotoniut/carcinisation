use super::{events::ClearScreenShutdownEvent, resources::ClearScreenInput};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

pub fn check_press_continue_input(
    mut screen_shutdown_event_writer: EventWriter<ClearScreenShutdownEvent>,
    // mut main_menu_event_writer: EventWriter<MainMenuShutdownEvent>,
    input: Res<ActionState<ClearScreenInput>>,
) {
    if input.just_pressed(ClearScreenInput::Continue) {
        screen_shutdown_event_writer.send(ClearScreenShutdownEvent);
        // main_menu_event_writer.send(MainMenuShutdownEvent);
    }
}
