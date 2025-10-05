//! Stage destructible props and their cleanup scheduling.

pub mod components;
pub mod data;
mod systems;

use self::systems::check_dead_destructible;
use bevy::prelude::*;

/// Manages destructible entities and despawns them once dead.
pub struct DestructiblePlugin;

impl Plugin for DestructiblePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<DestructiblePluginUpdateState>()
            .add_systems(
                Update,
                // Only check for cleanup while the destructible subsystem is active.
                (check_dead_destructible).run_if(in_state(DestructiblePluginUpdateState::Active)),
            );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
/// Whether destructible cleanup logic should tick this frame.
pub enum DestructiblePluginUpdateState {
    #[default]
    Inactive,
    Active,
}
