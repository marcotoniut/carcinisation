pub mod components;
pub mod styles;
pub mod systems;

use bevy::prelude::*;

use self::systems::{interactions::*, layout::*};
use crate::AppState;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), spawn_screen)
            .add_systems(OnExit(AppState::MainMenu), despawn_screen)
            .add_systems(
                Update,
                (interact_with_play_button, interact_with_quit_button)
                    .run_if(in_state(AppState::MainMenu)),
            );
    }
}
