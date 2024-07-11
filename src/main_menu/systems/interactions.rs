use crate::{
    game::{events::GameStartupTrigger, resources::Difficulty},
    input::GBInput,
    main_menu::{events::MainMenuShutdownEvent, resources::DifficultySelection},
    resources::DifficultySelected,
};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

pub fn check_press_start_input(mut commands: Commands, gb_input: Res<ActionState<GBInput>>) {
    if gb_input.just_pressed(&GBInput::Start)
        || gb_input.just_pressed(&GBInput::A)
        || gb_input.just_pressed(&GBInput::B)
    {
        commands.trigger(GameStartupTrigger);
        commands.trigger(MainMenuShutdownEvent);
    }
}

pub fn check_main_select_select_option_input(
    mut commands: Commands,
    gb_input: Res<ActionState<GBInput>>,
) {
    if gb_input.just_pressed(&GBInput::Start) || gb_input.just_pressed(&GBInput::A) {
        commands.trigger(GameStartupTrigger);
        commands.trigger(MainMenuShutdownEvent);
    }
}

pub fn game_difficulty_select_change(
    mut selection: ResMut<DifficultySelection>,
    gb_input: Res<ActionState<GBInput>>,
) {
    let input =
        gb_input.just_pressed(&GBInput::Up) as i8 - gb_input.just_pressed(&GBInput::Down) as i8;
    if input < 0 {
        if let Ok(y) = Difficulty::try_from((selection.0 as i8) - 1) {
            selection.0 = y;
        } else {
            // Little sound indicating lower bound
        }
    } else if input > 0 {
        if let Ok(y) = Difficulty::try_from((selection.0 as i8) + 1) {
            selection.0 = y;
        } else {
            // Little sound indicating upper bound
        }
    }
}

pub fn game_difficulty_select_option(
    selection_state: Res<DifficultySelection>,
    mut selected_state: ResMut<DifficultySelected>,
    gb_input: Res<ActionState<GBInput>>,
) {
    if gb_input.just_pressed(&GBInput::Start) || gb_input.just_pressed(&GBInput::A) {
        selected_state.0 = selection_state.0;
    }
}
