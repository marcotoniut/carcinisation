//! Debug drawing utilities and plugin wiring.

pub mod plugin;
mod systems;
pub mod types;

use self::{systems::*, types::register_types};
use activable::{Activable, ActivableAppExt};
use bevy::prelude::*;

/// Registers debug drawing systems and type introspection helpers.
#[derive(Activable)]
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        register_types(app);
        app.add_active_systems::<DebugPlugin, _>((draw_floor_lines, draw_colliders));
    }
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
