use crate::GBInput;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use leafwing_input_manager::Actionlike;
use leafwing_input_manager::{action_state::ActionState, input_map::InputMap};

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum ClearScreenInput {
    Continue,
}

pub fn init_input(mut commands: Commands) {
    let ys: Vec<(KeyCode, ClearScreenInput)> = vec![
        (GBInput::B.into(), ClearScreenInput::Continue),
        (GBInput::A.into(), ClearScreenInput::Continue),
        (GBInput::Start.into(), ClearScreenInput::Continue),
    ];
    commands.insert_resource(ActionState::<ClearScreenInput>::default());
    commands.insert_resource(InputMap::<ClearScreenInput>::new(ys));
}
