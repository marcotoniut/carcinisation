pub mod components;
pub mod events;
pub mod systems;

use self::{
    events::{MainMenuShutdownEvent, MainMenuStartupEvent},
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
        app.add_state::<MainMenuPluginUpdateState>()
            .add_state::<MainMenuScreenState>()
            .add_event::<MainMenuStartupEvent>()
            .add_event::<MainMenuShutdownEvent>()
            .add_systems(OnEnter(MainMenuPluginUpdateState::Active), spawn_main_menu)
            .add_systems(
                OnExit(MainMenuPluginUpdateState::Inactive),
                despawn_main_menu,
            )
            .add_systems(PreUpdate, (on_startup, on_shutdown))
            .add_systems(
                Update,
                (
                    (press_start).run_if(in_state(MainMenuScreenState::PressStart)),
                    press_next,
                    press_esc,
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

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum MainMenuScreenState {
    #[default]
    PressStart,
    MainSelect,
    // TODO can this be nested under MainSelect?
    GameDifficulty,
}
