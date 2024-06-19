use crate::input::GBInput;
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
    let ms: Vec<(MainMenuScreenInput, KeyCode)> = vec![
        (MainMenuScreenInput::Select, GBInput::A.into()),
        (MainMenuScreenInput::Cancel, GBInput::B.into()),
        (MainMenuScreenInput::Down, GBInput::Down.into()),
        (MainMenuScreenInput::Up, GBInput::Up.into()),
        (MainMenuScreenInput::Switch, GBInput::Select.into()),
        (MainMenuScreenInput::Select, GBInput::Start.into()),
    ];
    commands.insert_resource(ActionState::<MainMenuScreenInput>::default());
    commands.insert_resource(InputMap::<MainMenuScreenInput>::new(ms));
}
