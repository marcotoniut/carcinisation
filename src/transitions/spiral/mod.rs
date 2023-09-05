pub mod bundles;
pub mod components;
pub mod resources;
pub mod systems;

use bevy::prelude::*;

use self::{
    bundles::update_transition,
    resources::TransitionUpdateTimer,
    systems::{layout::*, tick_timer},
};
use crate::AppState;

pub struct TransitionVenetianPlugin;

impl Plugin for TransitionVenetianPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransitionUpdateTimer>()
            .add_systems(OnEnter(AppState::Transition), insert_transition_counter)
            .add_systems(OnExit(AppState::Transition), remove_transition_counter)
            .add_systems(
                Update,
                (
                    tick_timer,
                    (update_transition).run_if(in_state(AppState::Transition)),
                ),
            );
    }
}
