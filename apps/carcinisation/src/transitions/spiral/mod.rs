pub mod bundles;
pub mod components;
pub mod events;
pub mod resources;
mod systems;

use self::{
    bundles::{check_transition_finished, update_transition},
    events::{TransitionVenetianShutdownEvent, TransitionVenetianStartupEvent},
    resources::{TransitionUpdateTimer, TransitionVenetianTime},
    systems::{
        setup::{on_transition_shutdown, on_transition_startup},
        tick_timer,
    },
};
use crate::core::time::tick_time;
use bevy::prelude::*;

pub struct TransitionVenetianPlugin;

impl Plugin for TransitionVenetianPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<TransitionVenetianPluginUpdateState>()
            .init_resource::<TransitionVenetianTime>()
            .init_resource::<TransitionUpdateTimer>()
            .add_message::<TransitionVenetianStartupEvent>()
            .add_observer(on_transition_startup)
            .add_message::<TransitionVenetianShutdownEvent>()
            .add_observer(on_transition_shutdown)
            .add_systems(
                Update,
                (
                    tick_timer,
                    update_transition,
                    check_transition_finished,
                    tick_time::<TransitionVenetianTime>,
                )
                    .chain()
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
