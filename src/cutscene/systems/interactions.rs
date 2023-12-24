use crate::cutscene::{events::CutsceneShutdownEvent, input::CutsceneInput};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

pub fn check_press_start_input(
    mut shutdown_event_writer: EventWriter<CutsceneShutdownEvent>,
    gb_input: Res<ActionState<CutsceneInput>>,
) {
    if gb_input.just_pressed(CutsceneInput::Skip) {
        shutdown_event_writer.send(CutsceneShutdownEvent);
    }
}
