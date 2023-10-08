pub mod components;
pub mod systems;

use bevy::prelude::*;

use self::systems::{interactions::*, layout::*};

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<MainMenuState>()
            .add_systems(OnEnter(MainMenuState::Active), spawn_main_menu)
            .add_systems(OnExit(MainMenuState::Inactive), despawn_main_menu)
            .add_systems(
                Update,
                (press_next, press_esc).run_if(in_state(MainMenuState::Active)),
            );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum MainMenuState {
    #[default]
    Inactive,
    Active,
}
