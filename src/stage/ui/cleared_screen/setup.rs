use super::input::ClearScreenInput;
use crate::GBInput;
use bevy::prelude::*;
use leafwing_input_manager::{action_state::ActionState, input_map::InputMap};

pub fn init_input(mut commands: Commands) {
    let ys: Vec<(KeyCode, ClearScreenInput)> = vec![
        (GBInput::B.into(), ClearScreenInput::Continue),
        (GBInput::A.into(), ClearScreenInput::Continue),
        (GBInput::Start.into(), ClearScreenInput::Continue),
    ];
    commands.insert_resource(ActionState::<ClearScreenInput>::default());
    commands.insert_resource(InputMap::<ClearScreenInput>::new(ys));
}
