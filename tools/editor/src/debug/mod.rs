pub mod types;

use bevy::prelude::*;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        carcinisation::debug::types::register_types(app);
        self::types::register_types(app);
        app.init_state::<DebugPluginUpdateState>();
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum DebugPluginUpdateState {
    Inactive,
    #[default]
    Active,
}
