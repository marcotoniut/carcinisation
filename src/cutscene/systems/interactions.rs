use crate::cutscene::{events::CutsceneShutdownTrigger, input::CutsceneInput};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

pub fn check_press_start_input(mut commands: Commands, gb_input: Res<ActionState<CutsceneInput>>) {
    if gb_input.just_pressed(&CutsceneInput::Skip) {
        commands.trigger(CutsceneShutdownTrigger);
    }
}
