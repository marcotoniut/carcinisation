//! FP rendering sub-layers.

use bevy::prelude::Reflect;
use carapace::math::Next;
use serde::{Deserialize, Serialize};

/// Sub-layer ordering for first-person rendering.
///
/// Used as `Layer::FirstPerson(FpSubLayer)` in the game's layer enum.
#[derive(
    Clone, Debug, Deserialize, Eq, PartialEq, Hash, PartialOrd, Ord, Reflect, Serialize, Default,
)]
pub enum FpSubLayer {
    /// The raycasted 3D view (walls, floor, ceiling).
    #[default]
    View,
    /// Billboards (enemies, pickups, projectiles) drawn on top of the view.
    Billboards,
    /// HUD elements (crosshair, health bar) overlaying the 3D scene.
    Hud,
}

impl Next for FpSubLayer {
    const MIN: Self = FpSubLayer::View;

    fn next(self) -> Option<Self> {
        match self {
            Self::View => Some(Self::Billboards),
            Self::Billboards => Some(Self::Hud),
            Self::Hud => None,
        }
    }
}
