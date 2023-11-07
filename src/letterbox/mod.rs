pub mod components;
pub mod events;
pub mod resources;
pub mod systems;

use crate::{
    core::time::tick_time,
    plugins::movement::linear::{components::TargetingPositionY, LinearMovementPlugin},
};

use self::{events::LetterboxMoveEvent, resources::LetterboxTime, systems::*};
use bevy::prelude::*;

pub struct LetterboxPlugin;

impl Plugin for LetterboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<LetterboxPluginUpdateState>()
            .add_event::<LetterboxMoveEvent>()
            .init_resource::<LetterboxTime>()
            .add_systems(OnEnter(LetterboxPluginUpdateState::Active), on_startup)
            .add_systems(OnEnter(LetterboxPluginUpdateState::Inactive), on_shutdown)
            .add_plugins(LinearMovementPlugin::<LetterboxTime, TargetingPositionY>::default())
            .add_systems(
                PreUpdate,
                (on_move).run_if(in_state(LetterboxPluginUpdateState::Active)),
            )
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
