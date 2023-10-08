pub mod components;
pub mod systems;

use self::systems::{layout::*, update::*};
use bevy::prelude::*;
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<HudPluginUpdateState>()
            .add_systems(Startup, spawn_hud)
            .add_systems(
                Update,
                (update_health_text).run_if(in_state(HudPluginUpdateState::Active)),
            );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum HudPluginUpdateState {
    #[default]
    Inactive,
    Active,
}
