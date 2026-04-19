//! Debug drawing utilities and plugin wiring.

#[cfg(debug_assertions)]
pub mod plugin;
#[cfg(debug_assertions)]
pub mod systems;
pub mod types;

#[cfg(debug_assertions)]
pub use self::systems::DebugColliderOverlay;
#[cfg(debug_assertions)]
use self::{
    systems::{
        debug_damage_composed_parts, draw_colliders, draw_floor_lines,
        log_composed_health_pool_changes,
    },
    types::register_types,
};
#[cfg(debug_assertions)]
use activable::{Activable, ActivableAppExt, activate_system};
#[cfg(debug_assertions)]
use bevy::prelude::*;
#[cfg(debug_assertions)]
use serde::{Deserialize, Serialize};

/// Registers debug drawing systems and type introspection helpers.
#[cfg(debug_assertions)]
#[derive(Activable)]
pub struct DebugPlugin;

/// Debug-only player invulnerability toggle.
///
/// This is intentionally a runtime debug resource rather than gameplay state.
/// Combat systems still emit normal damage; the shared damage application
/// boundary decides whether player damage is ignored while debugging.
#[cfg(debug_assertions)]
#[derive(Resource, Clone, Copy, Debug, Reflect, Serialize, Deserialize)]
#[reflect(Resource)]
pub struct DebugGodMode {
    pub enabled: bool,
}

#[cfg(debug_assertions)]
impl DebugGodMode {
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

/// BRP-drivable debug probe used to exercise composed-part damage deterministically.
///
/// This resource is debug-only and does not own gameplay state. It exists so
/// runtime inspection tools can drive the same semantic collision and damage
/// routing path without relying on manual input timing.
#[cfg(debug_assertions)]
#[derive(Resource, Clone, Debug, Default, Reflect, Serialize, Deserialize)]
#[reflect(Resource)]
pub struct DebugComposedDamageProbe {
    pub request: Option<DebugComposedDamageProbeRequest>,
    pub last_result: Option<DebugComposedDamageProbeResult>,
}

/// A single pending composed-part damage request.
#[cfg(debug_assertions)]
#[derive(Clone, Debug, Reflect, Serialize, Deserialize)]
pub struct DebugComposedDamageProbeRequest {
    pub part_id: String,
    pub damage: u32,
}

/// The last composed-part probe the debug system attempted to dispatch.
///
/// `dispatched = true` means the probe resolved through composed collision
/// state and emitted a real `PartDamageMessage`.
#[cfg(debug_assertions)]
#[derive(Clone, Debug, Reflect, Serialize, Deserialize)]
pub struct DebugComposedDamageProbeResult {
    pub requested_part_id: String,
    pub resolved_part_id: Option<String>,
    pub damage: u32,
    pub pool_id: Option<String>,
    pub pool_before: Option<u32>,
    pub probe_point: Option<Vec2>,
    pub dispatched: bool,
    pub error: Option<String>,
}

#[cfg(debug_assertions)]
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        register_types(app);
        app.init_resource::<DebugComposedDamageProbe>()
            .init_resource::<systems::DebugColliderOverlay>()
            .add_systems(Startup, activate_system::<DebugPlugin>)
            .add_active_systems::<DebugPlugin, _>((
                (
                    draw_floor_lines,
                    draw_colliders,
                    systems::draw_pixel_mask_outlines,
                ),
                (
                    systems::toggle_debug_god_mode,
                    systems::toggle_collider_overlay,
                ),
                (
                    debug_damage_composed_parts,
                    log_composed_health_pool_changes,
                ),
            ));
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
