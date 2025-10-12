use super::{events::GameOverScreenShutdownEvent, input::GameOverScreenInput};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

pub fn check_press_continue_input(
    mut screen_shutdown_event_writer: EventWriter<GameOverScreenShutdownEvent>,
    input: Res<ActionState<GameOverScreenInput>>,
) {
    if input.just_pressed(&GameOverScreenInput::BackToMenu) {
        screen_shutdown_event_writer.send(GameOverScreenShutdownEvent);
    }
}
