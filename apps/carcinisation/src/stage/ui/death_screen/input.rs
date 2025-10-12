use crate::input::GBInput;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use leafwing_input_manager::Actionlike;
use leafwing_input_manager::{action_state::ActionState, input_map::InputMap};

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum DeathScreenInput {
    Restart,
}

pub fn init_input(mut commands: Commands) {
    let ms: Vec<(DeathScreenInput, KeyCode)> = vec![
        (DeathScreenInput::Restart, GBInput::B.into()),
        (DeathScreenInput::Restart, GBInput::A.into()),
        (DeathScreenInput::Restart, GBInput::Start.into()),
    ];
    commands.insert_resource(ActionState::<DeathScreenInput>::default());
    commands.insert_resource(InputMap::<DeathScreenInput>::new(ms));
}
