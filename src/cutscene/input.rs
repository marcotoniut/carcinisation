use crate::GBInput;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use leafwing_input_manager::Actionlike;
use leafwing_input_manager::{action_state::ActionState, input_map::InputMap};

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum CutsceneInput {
    Skip,
}

pub fn init_input(mut commands: Commands) {
    let ys: Vec<(KeyCode, CutsceneInput)> = vec![(GBInput::Start.into(), CutsceneInput::Skip)];
    commands.insert_resource(ActionState::<CutsceneInput>::default());
    commands.insert_resource(InputMap::<CutsceneInput>::new(ys));
}
