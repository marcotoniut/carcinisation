//! Main menu state machine, events, and UI spawning.

pub mod components;
pub mod input;
pub mod resources;
mod systems;

use self::{
    resources::DifficultySelection,
    systems::{
        interactions::*,
        layout::*,
        setup::{on_main_menu_shutdown, on_main_menu_startup},
    },
};
use activable::{Activable, ActivableAppExt};
use bevy::prelude::*;

/// Registers menu resources, screens, and interaction systems.
#[derive(Activable)]
pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(MainMenuScreen::default())
            .init_resource::<DifficultySelection>()
            .on_active::<MainMenuPlugin, _>((spawn_main_menu, on_main_menu_startup))
            .on_inactive::<MainMenuPlugin, _>(on_main_menu_shutdown)
            .add_systems(
                OnEnter(MainMenuScreen::PressStart),
                enter_press_start_screen,
            )
            .add_systems(OnExit(MainMenuScreen::PressStart), exit_press_start_screen)
            .add_systems(
                OnEnter(MainMenuScreen::DifficultySelect),
                enter_game_difficulty_screen,
            )
            .add_systems(
                OnExit(MainMenuScreen::DifficultySelect),
                exit_game_difficulty_screen,
            )
            .add_active_systems::<MainMenuPlugin, _>(
                // Handle input according to the active menu screen.
                (
                    (check_press_start_input).run_if(in_state(MainMenuScreen::PressStart)),
                    (check_main_select_select_option_input)
                        .run_if(in_state(MainMenuScreen::MainMenuSelect)),
                    (
                        game_difficulty_select_change,
                        game_difficulty_select_option,
                        update_difficulty_selection_indicator,
                    )
                        .run_if(in_state(MainMenuScreen::DifficultySelect)),
                ),
            );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
/// Tracks which menu screen is currently visible.
pub enum MainMenuScreen {
    #[default]
    Disabled,
    PressStart,
    MainMenuSelect,
    // TODO can this be nested under MainSelect?
    DifficultySelect,
}
