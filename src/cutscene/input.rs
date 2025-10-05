//! Cutscene-specific input actions (skip, etc.).

use crate::input::GBInput;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use leafwing_input_manager::Actionlike;
use leafwing_input_manager::{action_state::ActionState, input_map::InputMap};

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
/// Actions available while a cutscene plays.
pub enum CutsceneInput {
    Skip,
}

/// @system Registers the cutscene input map using the shared GB bindings.
pub fn init_input(mut commands: Commands) {
    let ms: Vec<(CutsceneInput, KeyCode)> = vec![(CutsceneInput::Skip, GBInput::Start.into())];
    commands.insert_resource(ActionState::<CutsceneInput>::default());
    commands.insert_resource(InputMap::<CutsceneInput>::new(ms));
}
