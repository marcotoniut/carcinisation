//! Debug drawing utilities and plugin wiring.

pub mod plugin;
mod systems;
pub mod types;

use self::{systems::*, types::register_types};
use bevy::prelude::*;

/// Registers debug drawing systems and type introspection helpers.
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        register_types(app);
        app.init_state::<DebugPluginUpdateState>().add_systems(
            Update,
            (draw_floor_lines, draw_colliders).run_if(in_state(DebugPluginUpdateState::Active)),
        );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
/// Enables optional debug drawing systems.
pub enum DebugPluginUpdateState {
    Inactive,
    #[default]
    Active,
}

pub trait DebugColor {
    const YELLOW_GREEN: Self;
    const ALICE_BLUE: Self;
    const FUCHSIA: Self;
}

impl DebugColor for Color {
    const YELLOW_GREEN: Self = Color::srgb(0.6, 0.8, 0.2);
    const ALICE_BLUE: Self = Color::srgb(0.94, 0.97, 1.0);
    const FUCHSIA: Self = Color::srgb(1.0, 0.0, 1.0);
}
