pub mod components;
pub mod data;
pub mod systems;

use self::systems::check_dead_destructible;
use bevy::prelude::*;

pub struct DestructiblePlugin;

impl Plugin for DestructiblePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<DestructiblePluginUpdateState>()
            .add_systems(
                Update,
                (check_dead_destructible).run_if(in_state(DestructiblePluginUpdateState::Active)),
            );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum DestructiblePluginUpdateState {
    #[default]
    Inactive,
    Active,
}
