//! Letterbox bars shown during cutscenes and transitions.

pub mod components;
pub mod events;
pub mod resources;
mod systems;

use crate::core::time::tick_time;
use activable::{activate_system, Activable, ActivableAppExt};
use cween::linear::{components::TargetingPositionY, LinearMovementPlugin};

use self::{events::LetterboxMoveTrigger, resources::LetterboxTimeDomain, systems::*};
use bevy::{prelude::*, time::Fixed};

/// Manages letterbox entities, movement triggers, and timing.
#[derive(Activable)]
pub struct LetterboxPlugin;

impl Plugin for LetterboxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Time<LetterboxTimeDomain>>()
            .add_message::<LetterboxMoveTrigger>()
            .on_active::<LetterboxPlugin, _>(on_letterbox_startup)
            .on_inactive::<LetterboxPlugin, _>(on_letterbox_shutdown)
            .add_plugins(LinearMovementPlugin::<
                LetterboxTimeDomain,
                TargetingPositionY,
            >::default())
            .add_observer(on_move)
            .add_active_systems_in::<LetterboxPlugin, _>(
                FixedUpdate,
                // Keep letterbox movement timers in sync when active.
                tick_time::<Fixed, LetterboxTimeDomain>,
            )
            .add_systems(Startup, activate_system::<LetterboxPlugin>);
    }
}
