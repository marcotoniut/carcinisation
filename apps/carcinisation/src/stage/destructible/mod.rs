//! Stage destructible props and their cleanup scheduling.

pub mod components;
pub mod data;
mod systems;

use self::systems::check_dead_destructible;
use activable::{Activable, ActivableAppExt};
use bevy::prelude::*;

/// Manages destructible entities and despawns them once dead.
#[derive(Activable)]
pub struct DestructiblePlugin;

impl Plugin for DestructiblePlugin {
    fn build(&self, app: &mut App) {
        app.add_active_systems::<DestructiblePlugin, _>(check_dead_destructible);
    }
}
