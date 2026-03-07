//! Debug drawing utilities and plugin wiring.

#[cfg(debug_assertions)]
pub mod plugin;
#[cfg(debug_assertions)]
mod systems;
pub mod types;

#[cfg(debug_assertions)]
use self::{
    systems::{draw_colliders, draw_floor_lines},
    types::register_types,
};
#[cfg(debug_assertions)]
use activable::{Activable, ActivableAppExt};
#[cfg(debug_assertions)]
use bevy::prelude::*;

/// Registers debug drawing systems and type introspection helpers.
#[cfg(debug_assertions)]
#[derive(Activable)]
pub struct DebugPlugin;

#[cfg(debug_assertions)]
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        register_types(app);
        app.add_active_systems::<DebugPlugin, _>((draw_floor_lines, draw_colliders));
    }
}

#[cfg(debug_assertions)]
pub trait DebugColor {
    const YELLOW_GREEN: Self;
    const ALICE_BLUE: Self;
    const FUCHSIA: Self;
}

#[cfg(debug_assertions)]
impl DebugColor for Color {
    const YELLOW_GREEN: Self = Color::srgb(0.6, 0.8, 0.2);
    const ALICE_BLUE: Self = Color::srgb(0.94, 0.97, 1.0);
    const FUCHSIA: Self = Color::srgb(1.0, 0.0, 1.0);
}
