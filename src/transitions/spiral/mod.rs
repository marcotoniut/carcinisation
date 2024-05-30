pub mod bundles;
pub mod components;
pub mod events;
pub mod resources;
pub mod systems;

use self::{
    bundles::{progress, update_transition},
    events::{TransitionVenetianShutdownEvent, TransitionVenetianStartupEvent},
    resources::{TransitionUpdateTimer, TransitionVenetianTime},
    systems::{
        setup::{on_shutdown, on_startup},
        tick_timer,
    },
};
use crate::core::time::tick_time;
use bevy::prelude::*;

pub struct TransitionVenetianPlugin;

impl Plugin for TransitionVenetianPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<TransitionVenetianPluginUpdateState>()
            .add_event::<TransitionVenetianStartupEvent>()
            .add_event::<TransitionVenetianShutdownEvent>()
            .init_resource::<TransitionUpdateTimer>()
            .add_systems(PreUpdate, (on_startup, on_shutdown))
            .add_systems(
                Update,
                (
                    tick_timer,
                    update_transition,
                    progress,
                    tick_time::<TransitionVenetianTime>,
                )
                    .run_if(in_state(TransitionVenetianPluginUpdateState::Active)),
            );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum TransitionVenetianPluginUpdateState {
    #[default]
    Inactive,
    Active,
}
