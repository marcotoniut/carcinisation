//! Player interactions during cutscenes.

use crate::{
    cutscene::{
        data::{CutsceneData, CutsceneSkipMode},
        input::CutsceneInput,
        messages::CutsceneShutdownEvent,
    },
    input::GBInput,
};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

/// @system Skips the cutscene based on the configured skip mode.
pub fn check_press_start_input(
    mut commands: Commands,
    gb_input: Res<ActionState<CutsceneInput>>,
    data: Option<Res<CutsceneData>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let skip_mode = data
        .as_ref()
        .map_or(&CutsceneSkipMode::StartOnly, |d| &d.skip_mode);

    let should_skip = match skip_mode {
        CutsceneSkipMode::StartOnly => gb_input.just_pressed(&CutsceneInput::Skip),
        CutsceneSkipMode::AnyGameplayKey => {
            let gameplay_actions = [
                GBInput::A,
                GBInput::B,
                GBInput::Start,
                GBInput::Select,
                GBInput::Up,
                GBInput::Down,
                GBInput::Left,
                GBInput::Right,
            ];
            gameplay_actions
                .iter()
                .any(|a| keyboard.just_pressed(KeyCode::from(*a)))
        }
    };

    if should_skip {
        commands.trigger(CutsceneShutdownEvent);
    }
}
