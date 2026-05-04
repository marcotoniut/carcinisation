//! FPS rendering layer ordering.
//!
//! Self-contained enum — no carapace dependency.

/// Sub-layer ordering for first-person rendering.
///
/// Ordering: `View < Billboards < Hud`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum FpsLayer {
    /// The raycasted 3D view (walls, floor, ceiling).
    #[default]
    View,
    /// Billboards (enemies, pickups, projectiles).
    Billboards,
    /// HUD elements (crosshair, health bar).
    Hud,
}
