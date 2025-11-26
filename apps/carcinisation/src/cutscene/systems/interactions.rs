//! Player interactions during cutscenes.

use crate::cutscene::{input::CutsceneInput, messages::CutsceneShutdownEvent};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

/// @system Skips the cutscene when the skip input is pressed.
pub fn check_press_start_input(mut commands: Commands, gb_input: Res<ActionState<CutsceneInput>>) {
    if gb_input.just_pressed(&CutsceneInput::Skip) {
        commands.trigger(CutsceneShutdownEvent);
    }
}
