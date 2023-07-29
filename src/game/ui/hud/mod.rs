mod components;
mod styles;
mod systems;

use bevy::prelude::*;

use self::systems::{layout::*, update::*};
use crate::AppState;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), spawn_hud)
            .add_systems(OnExit(AppState::Game), despawn_hud)
            .add_systems(
                Update,
                (update_score_text, update_enemy_text).run_if(in_state(AppState::Game)),
            );
    }
}
