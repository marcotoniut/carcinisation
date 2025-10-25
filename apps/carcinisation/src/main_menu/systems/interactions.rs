//! Systems handling menu inputs and transitions between screens.

use crate::{
    game::{events::GameStartupTrigger, resources::Difficulty},
    input::GBInput,
    main_menu::{
        events::{ChangeMainMenuScreenTrigger, MainMenuShutdownEvent},
        resources::DifficultySelection,
        MainMenuScreen,
    },
    resources::DifficultySelected,
};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

/// @system Transitions from press-start screen to difficulty selection.
pub fn check_press_start_input(mut commands: Commands, gb_input: Res<ActionState<GBInput>>) {
    if gb_input.just_pressed(&GBInput::Start)
        || gb_input.just_pressed(&GBInput::A)
        || gb_input.just_pressed(&GBInput::B)
    {
        commands.trigger(ChangeMainMenuScreenTrigger(
            MainMenuScreen::DifficultySelect,
        ));
    }
}

/// @system Confirms selection from the main menu screen, transitions to difficulty select.
pub fn check_main_select_select_option_input(
    mut commands: Commands,
    gb_input: Res<ActionState<GBInput>>,
) {
    if gb_input.just_pressed(&GBInput::Start) || gb_input.just_pressed(&GBInput::A) {
        commands.trigger(ChangeMainMenuScreenTrigger(
            MainMenuScreen::DifficultySelect,
        ));
    }
}

/// @system Cycles through difficulty options based on directional input.
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

/// @system Applies the currently highlighted difficulty when confirmed and starts the game.
pub fn game_difficulty_select_option(
    mut commands: Commands,
    selection_state: Res<DifficultySelection>,
    mut selected_state: ResMut<DifficultySelected>,
    gb_input: Res<ActionState<GBInput>>,
) {
    if gb_input.just_pressed(&GBInput::Start) || gb_input.just_pressed(&GBInput::A) {
        selected_state.0 = selection_state.0;
        commands.trigger(GameStartupTrigger);
        commands.trigger(MainMenuShutdownEvent);
    }
}
