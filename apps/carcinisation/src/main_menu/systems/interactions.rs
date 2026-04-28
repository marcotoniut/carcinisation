//! Systems handling menu inputs and transitions between screens.

#[cfg(feature = "gallery")]
use crate::gallery::{GalleryPlugin, messages::GalleryStartupEvent};
use crate::{
    game::{messages::GameStartupEvent, resources::Difficulty},
    main_menu::{MainMenuPlugin, MainMenuScreen, resources::DifficultySelection},
    resources::DifficultySelected,
};
#[cfg(feature = "gallery")]
use activable::activate;
use activable::deactivate;
use bevy::prelude::*;
use carcinisation_input::GBInput;
use leafwing_input_manager::prelude::ActionState;

/// @system Transitions from press-start screen to difficulty selection, or gallery via Select.
#[allow(unused_mut, unused_variables)]
pub fn check_press_start_input(
    mut commands: Commands,
    mut next_screen: ResMut<NextState<MainMenuScreen>>,
    gb_input: Res<ActionState<GBInput>>,
) {
    #[cfg(feature = "gallery")]
    if gb_input.just_pressed(&GBInput::Select) {
        commands.trigger(GalleryStartupEvent);
        deactivate::<MainMenuPlugin>(&mut commands);
        activate::<GalleryPlugin>(&mut commands);
        return;
    }

    if gb_input.just_pressed(&GBInput::Start)
        || gb_input.just_pressed(&GBInput::A)
        || gb_input.just_pressed(&GBInput::B)
    {
        next_screen.set(MainMenuScreen::DifficultySelect);
    }
}

/// @system Confirms selection from the main menu screen, transitions to difficulty select.
pub fn check_main_select_select_option_input(
    mut next_screen: ResMut<NextState<MainMenuScreen>>,
    gb_input: Res<ActionState<GBInput>>,
) {
    if gb_input.just_pressed(&GBInput::Start) || gb_input.just_pressed(&GBInput::A) {
        next_screen.set(MainMenuScreen::DifficultySelect);
    }
}

/// @system Cycles through difficulty options based on directional input.
pub fn game_difficulty_select_change(
    mut selection: ResMut<DifficultySelection>,
    gb_input: Res<ActionState<GBInput>>,
) {
    let input = i8::from(gb_input.just_pressed(&GBInput::Down))
        - i8::from(gb_input.just_pressed(&GBInput::Up));
    match input.cmp(&0) {
        std::cmp::Ordering::Less => {
            if let Ok(y) = Difficulty::try_from((selection.0 as i8) - 1) {
                selection.0 = y;
            } else {
                // Little sound indicating lower bound
            }
        }
        std::cmp::Ordering::Greater => {
            if let Ok(y) = Difficulty::try_from((selection.0 as i8) + 1) {
                selection.0 = y;
            } else {
                // Little sound indicating upper bound
            }
        }
        std::cmp::Ordering::Equal => {}
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
        commands.trigger(GameStartupEvent);
        deactivate::<MainMenuPlugin>(&mut commands);
    }
}
