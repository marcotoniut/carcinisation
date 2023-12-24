use crate::GBInput;
use bevy::prelude::*;
use leafwing_input_manager::{action_state::ActionState, input_map::InputMap};

use super::input::GameOverScreenInput;

pub fn init_input(mut commands: Commands) {
    let ys: Vec<(KeyCode, GameOverScreenInput)> = vec![
        (GBInput::B.into(), GameOverScreenInput::BackToMenu),
        (GBInput::A.into(), GameOverScreenInput::BackToMenu),
        (GBInput::Start.into(), GameOverScreenInput::BackToMenu),
    ];
    commands.insert_resource(ActionState::<GameOverScreenInput>::default());
    commands.insert_resource(InputMap::<GameOverScreenInput>::new(ys));
}
