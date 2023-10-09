pub mod systems;

use bevy::prelude::*;

use self::systems::draw_floor_lines;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<DebugPluginUpdateState>().add_systems(
            Update,
            (draw_floor_lines).run_if(in_state(DebugPluginUpdateState::Active)),
        );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum DebugPluginUpdateState {
    Inactive,
    #[default]
    Active,
}
