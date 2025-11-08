//! Letterbox bars shown during cutscenes and transitions.

pub mod components;
pub mod events;
pub mod resources;
mod systems;

use crate::{
    core::time::tick_time,
    plugins::movement::linear::{components::TargetingPositionY, LinearMovementPlugin},
};
use activable::{activate_system, Activable, ActivableAppExt};

use self::{events::LetterboxMoveTrigger, resources::LetterboxTime, systems::*};
use bevy::prelude::*;

/// Manages letterbox entities, movement triggers, and timing.
#[derive(Activable)]
pub struct LetterboxPlugin;

impl Plugin for LetterboxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LetterboxTime>()
            .add_message::<LetterboxMoveTrigger>()
            .on_active::<LetterboxPlugin, _>(on_letterbox_startup)
            .on_inactive::<LetterboxPlugin, _>(on_letterbox_shutdown)
            .add_plugins(LinearMovementPlugin::<LetterboxTime, TargetingPositionY>::default())
            .add_observer(on_move)
            .add_active_systems::<LetterboxPlugin, _>(
                // Keep letterbox movement timers in sync when active.
                tick_time::<LetterboxTime>,
            )
            .add_systems(Startup, activate_system::<LetterboxPlugin>);
    }
}
