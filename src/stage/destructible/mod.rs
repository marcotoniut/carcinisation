pub mod components;
pub mod data;
pub mod systems;

use bevy::prelude::*;

use self::systems::check_dead_destructible;

use super::{GameState, StageState};
use crate::AppState;

pub struct DestructiblePlugin;

impl Plugin for DestructiblePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (check_dead_destructible)
                .run_if(in_state(StageState::Running))
                .run_if(in_state(GameState::Running))
                .run_if(in_state(AppState::Game)),
        );
    }
}
