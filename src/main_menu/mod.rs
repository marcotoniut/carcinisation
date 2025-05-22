pub mod components;
pub mod events;
pub mod input;
pub mod resources;
mod systems;

use self::{
    events::{ChangeMainMenuScreenTrigger, MainMenuShutdownEvent, MainMenuStartupEvent},
    resources::DifficultySelection,
    systems::{
        interactions::*,
        layout::*,
        setup::{on_main_menu_shutdown, on_main_menu_startup},
    },
};
use bevy::prelude::*;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<MainMenuPluginUpdateState>()
            .init_resource::<MainMenuScreen>()
            .init_resource::<DifficultySelection>()
            .add_event::<ChangeMainMenuScreenTrigger>()
            .add_observer(on_change_main_menu_screen)
            .add_event::<MainMenuStartupEvent>()
            .add_observer(on_main_menu_startup)
            .add_event::<MainMenuShutdownEvent>()
            .add_observer(on_main_menu_shutdown)
            .add_systems(OnEnter(MainMenuPluginUpdateState::Active), spawn_main_menu)
            // .add_systems(
            //     OnExit(MainMenuPluginUpdateState::Inactive),
            //     despawn_main_menu,
            // )
            .add_systems(
                Update,
                (spawn_game_difficulty_screen, spawn_press_start_screen)
                    .run_if(in_state(MainMenuPluginUpdateState::Active)),
            )
            .add_systems(
                PostUpdate,
                (
                    (check_press_start_input)
                        .run_if(resource_exists_and_equals(MainMenuScreen::PressStart)),
                    (check_main_select_select_option_input)
                        .run_if(resource_exists_and_equals(MainMenuScreen::MainMenuSelect)),
                    (game_difficulty_select_change, game_difficulty_select_option)
                        .run_if(resource_exists_and_equals(MainMenuScreen::DifficultySelect)),
                )
                    .run_if(in_state(MainMenuPluginUpdateState::Active)),
            );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum MainMenuPluginUpdateState {
    #[default]
    Inactive,
    Active,
}

#[derive(Resource, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum MainMenuScreen {
    #[default]
    PressStart,
    MainMenuSelect,
    // TODO can this be nested under MainSelect?
    DifficultySelect,
}
