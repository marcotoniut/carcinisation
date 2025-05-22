pub mod components;
pub mod events;
pub mod resources;
mod systems;

use crate::{
    core::time::tick_time,
    plugins::movement::linear::{components::TargetingPositionY, LinearMovementPlugin},
};

use self::{events::LetterboxMoveTrigger, resources::LetterboxTime, systems::*};
use bevy::prelude::*;

pub struct LetterboxPlugin;

impl Plugin for LetterboxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LetterboxTime>()
            .init_state::<LetterboxPluginUpdateState>()
            .add_event::<LetterboxMoveTrigger>()
            .add_systems(
                OnEnter(LetterboxPluginUpdateState::Active),
                on_letterbox_startup,
            )
            .add_systems(
                OnEnter(LetterboxPluginUpdateState::Inactive),
                on_letterbox_shutdown,
            )
            .add_plugins(LinearMovementPlugin::<LetterboxTime, TargetingPositionY>::default())
            .add_observer(on_move)
            .add_systems(
                Update,
                (tick_time::<LetterboxTime>).run_if(in_state(LetterboxPluginUpdateState::Active)),
            );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum LetterboxPluginUpdateState {
    Inactive,
    #[default]
    Active,
}
