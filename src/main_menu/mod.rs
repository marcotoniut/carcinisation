pub mod components;
pub mod events;
pub mod input;
pub mod resources;
pub mod systems;

use self::{
    events::{ChangeMainMenuScreenEvent, MainMenuShutdownEvent, MainMenuStartupEvent},
    resources::DifficultySelection,
    systems::{
        interactions::*,
        layout::*,
        setup::{on_shutdown, on_startup},
    },
};
use bevy::prelude::*;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<MainMenuPluginUpdateState>()
            .add_event::<ChangeMainMenuScreenEvent>()
            .add_event::<MainMenuStartupEvent>()
            .add_event::<MainMenuShutdownEvent>()
            .init_resource::<MainMenuScreen>()
            .init_resource::<DifficultySelection>()
            .add_systems(PreUpdate, (on_startup, on_shutdown))
            .add_systems(OnEnter(MainMenuPluginUpdateState::Active), spawn_main_menu)
            // .add_systems(
            //     OnExit(MainMenuPluginUpdateState::Inactive),
            //     despawn_main_menu,
            // )
            .add_systems(
                Update,
                ((
                    on_change_main_menu_screen,
                    (spawn_game_difficulty_screen, spawn_press_start_screen),
                )
                    .chain(),)
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
