pub mod components;
pub mod styles;
pub mod systems;

use bevy::prelude::*;

use self::systems::{interactions::*, layout::*};
use super::super::GameState;

pub struct PauseScreenPlugin;

impl Plugin for PauseScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Paused), spawn_screen)
            .add_systems(OnExit(GameState::Paused), despawn_screen)
            .add_systems(
                Update,
                (
                    interact_with_resume_button,
                    interact_with_quit_to_main_menu_button,
                    interact_with_quit_button,
                )
                    .run_if(in_state(GameState::Paused)),
            );
    }
}
