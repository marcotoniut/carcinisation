pub mod systems;
pub mod types;

use self::{systems::*, types::register_types};
use bevy::prelude::*;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        register_types(app);
        app.add_state::<DebugPluginUpdateState>().add_systems(
            Update,
            (draw_floor_lines, draw_colliders).run_if(in_state(DebugPluginUpdateState::Active)),
        );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum DebugPluginUpdateState {
    Inactive,
    #[default]
    Active,
}
