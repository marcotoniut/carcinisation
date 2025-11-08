//! Main menu state machine, events, and UI spawning.

pub mod components;
pub mod events;
pub mod input;
pub mod resources;
mod systems;

use self::{
    events::{MainMenuShutdownEvent, MainMenuStartupEvent},
    resources::DifficultySelection,
    systems::{
        interactions::*,
        layout::*,
        setup::{on_main_menu_shutdown, on_main_menu_startup},
    },
};
use bevy::prelude::*;

/// Registers menu resources, screens, and interaction systems.
pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<MainMenuPluginUpdateState>()
            .insert_state(MainMenuScreen::default())
            .init_resource::<DifficultySelection>()
            .add_message::<MainMenuStartupEvent>()
            .add_observer(on_main_menu_startup)
            .add_message::<MainMenuShutdownEvent>()
            .add_observer(on_main_menu_shutdown)
            .add_systems(OnEnter(MainMenuPluginUpdateState::Active), spawn_main_menu)
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
            .add_systems(
                Update,
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
                )
                    .run_if(in_state(MainMenuPluginUpdateState::Active)),
            );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
/// Controls when menu systems should run.
pub enum MainMenuPluginUpdateState {
    #[default]
    Inactive,
    Active,
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
