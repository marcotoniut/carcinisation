pub mod components;
pub mod styles;
pub mod systems;

use bevy::prelude::*;

use self::systems::{layout::*, update::*};
use super::super::GameState;
use crate::AppState;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Running), spawn_hud)
            .add_systems(OnExit(GameState::Running), despawn_hud)
            .add_systems(
                Update,
                (update_score_text, update_enemy_text).run_if(in_state(AppState::Game)),
            );
    }
}
