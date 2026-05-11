//! Shared debug utilities and marker resources.

use bevy::prelude::*;

/// Debug-only player invulnerability toggle.
#[derive(Resource, Clone, Copy, Debug, Default, Reflect, serde::Serialize, serde::Deserialize)]
#[reflect(Resource)]
pub struct DebugGodMode {
    pub enabled: bool,
}

impl DebugGodMode {
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

/// Log a debug startup message (only in debug builds).
pub fn debug_print_startup(module: &str) {
    #[cfg(debug_assertions)]
    bevy::log::info!("[DEBUG] {module} startup");
}

/// Log a debug shutdown message (only in debug builds).
pub fn debug_print_shutdown(module: &str) {
    #[cfg(debug_assertions)]
    bevy::log::info!("[DEBUG] {module} shutdown");
}
