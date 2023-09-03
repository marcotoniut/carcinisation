pub mod components;
pub mod systems;

use bevy::prelude::*;

use self::systems::{interactions::*, layout::*};
use crate::AppState;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), spawn_main_menu)
            .add_systems(OnExit(AppState::MainMenu), despawn_main_menu)
            .add_systems(
                Update,
                (press_next, press_esc).run_if(in_state(AppState::MainMenu)),
            );
    }
}
