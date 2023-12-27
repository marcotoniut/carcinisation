use crate::GBInput;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use leafwing_input_manager::Actionlike;
use leafwing_input_manager::{action_state::ActionState, input_map::InputMap};

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum MainMenuScreenInput {
    Up,
    Down,
    Switch,
    Select,
    Cancel,
}

pub fn init_input(mut commands: Commands) {
    let ys: Vec<(KeyCode, MainMenuScreenInput)> = vec![
        (GBInput::A.into(), MainMenuScreenInput::Select),
        (GBInput::B.into(), MainMenuScreenInput::Cancel),
        (GBInput::Down.into(), MainMenuScreenInput::Down),
        (GBInput::Up.into(), MainMenuScreenInput::Up),
        (GBInput::Select.into(), MainMenuScreenInput::Switch),
        (GBInput::Start.into(), MainMenuScreenInput::Select),
    ];
    commands.insert_resource(ActionState::<MainMenuScreenInput>::default());
    commands.insert_resource(InputMap::<MainMenuScreenInput>::new(ys));
}
