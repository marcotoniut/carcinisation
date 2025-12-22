use bevy::prelude::*;

/// Requests unloading the active scene and editor entities.
#[derive(Event)]
pub struct UnloadSceneTrigger;
