use crate::{cutscene::events::CutsceneShutdownEvent, GBInput};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

pub fn check_press_start_input(
    mut shutdown_event_writer: EventWriter<CutsceneShutdownEvent>,
    gb_input: Res<ActionState<GBInput>>,
) {
    if gb_input.just_pressed(GBInput::Start)
        || gb_input.just_pressed(GBInput::A)
        || gb_input.just_pressed(GBInput::B)
    {
        shutdown_event_writer.send(CutsceneShutdownEvent);
    }
}
