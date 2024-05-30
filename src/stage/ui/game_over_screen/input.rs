use crate::GBInput;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use leafwing_input_manager::Actionlike;
use leafwing_input_manager::{action_state::ActionState, input_map::InputMap};

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum GameOverScreenInput {
    BackToMenu,
}

pub fn init_input(mut commands: Commands) {
    let ms: Vec<(GameOverScreenInput, KeyCode)> = vec![
        (GameOverScreenInput::BackToMenu, GBInput::B.into()),
        (GameOverScreenInput::BackToMenu, GBInput::A.into()),
        (GameOverScreenInput::BackToMenu, GBInput::Start.into()),
    ];
    commands.insert_resource(ActionState::<GameOverScreenInput>::default());
    commands.insert_resource(InputMap::<GameOverScreenInput>::new(ms));
}
