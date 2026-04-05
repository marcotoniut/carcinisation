//! Visual-only presentation transforms applied at render time.
//!
//! [`PxPresentationTransform`] provides per-entity scaling (and future rotation/offset)
//! that affects rendering **without** modifying gameplay position, collision, or anchoring.
//!
//! # Render-only
//!
//! This component is consumed exclusively by the CPU rendering pipeline in
//! [`draw_spatial_scaled`](crate::frame::draw_spatial_scaled) and the composite
//! scaling path in [`draw_layers`](crate::screen::draw::draw_layers). Gameplay
//! systems should never read or depend on it.
//!
//! # GPU palette path
//!
//! The `gpu_palette` render path does **not** currently support presentation
//! transforms. Entities with this component that also carry `PxGpuSprite` /
//! `PxGpuComposite` will render at native size on the GPU path.

use crate::prelude::*;

/// Minimum allowed scale per axis. Values below this are clamped to avoid
/// degenerate sampling (division-by-near-zero) in the render path.
const MIN_SCALE: f32 = 0.01;

/// Maximum allowed scale per axis in debug builds. Scales above this trigger
/// a warning — they are likely unintentional and allocate large scratch buffers.
#[cfg(debug_assertions)]
const MAX_SCALE_DEBUG_WARN: f32 = 10.0;

/// Visual-only transform applied at render time.
///
/// Does **not** affect gameplay position, collision, or anchoring.
/// When absent or at default values, the rendering path is completely unchanged
/// (no scratch buffer allocated, no extra sampling).
///
/// # Scale
///
/// `scale` controls per-axis nearest-neighbour scaling around the entity's
/// anchor point. `Vec2::ONE` means native size. Values below [`MIN_SCALE`]
/// are clamped automatically.
///
/// # Rotation
///
/// `rotation` is **reserved for future use** and currently ignored by the
/// render path.
///
/// # Offset
///
/// `offset` is an additional pixel displacement applied after scaling, useful
/// for shake or recoil effects.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct PxPresentationTransform {
    /// Non-uniform scale factor. `Vec2::ONE` = native size.
    pub scale: Vec2,
    /// Rotation in radians, counter-clockwise around the anchor point.
    /// Currently unused — reserved for future implementation.
    pub rotation: f32,
    /// Additional pixel offset applied after scale/rotation.
    pub offset: Vec2,
}

impl Default for PxPresentationTransform {
    fn default() -> Self {
        Self {
            scale: Vec2::ONE,
            rotation: 0.0,
            offset: Vec2::ZERO,
        }
    }
}

impl PxPresentationTransform {
    /// Returns true if this transform would have no visual effect
    /// (scale ≈ 1, rotation ≈ 0, offset ≈ 0).
    #[must_use]
    pub fn is_identity(&self) -> bool {
        !self.has_scale() && !self.has_offset() && self.rotation.abs() < f32::EPSILON
    }

    /// Returns true if scale differs meaningfully from `Vec2::ONE`.
    #[must_use]
    pub fn has_scale(&self) -> bool {
        (self.scale - Vec2::ONE).length_squared() >= f32::EPSILON
    }

    /// Returns true if offset differs meaningfully from `Vec2::ZERO`.
    #[must_use]
    pub fn has_offset(&self) -> bool {
        self.offset.length_squared() >= f32::EPSILON
    }

    /// Returns the scale with each axis clamped to `[MIN_SCALE, ∞)`.
    ///
    /// In debug builds, warns if any axis exceeds [`MAX_SCALE_DEBUG_WARN`].
    #[must_use]
    pub(crate) fn clamped_scale(&self) -> Vec2 {
        let s = Vec2::new(self.scale.x.max(MIN_SCALE), self.scale.y.max(MIN_SCALE));

        #[cfg(debug_assertions)]
        {
            if s.x > MAX_SCALE_DEBUG_WARN || s.y > MAX_SCALE_DEBUG_WARN {
                warn!(
                    "PxPresentationTransform scale ({:.2}, {:.2}) exceeds {MAX_SCALE_DEBUG_WARN}x — \
                     this allocates a large scratch buffer and is likely unintentional",
                    s.x, s.y,
                );
            }
            if self.scale.x <= 0.0 || self.scale.y <= 0.0 {
                warn!(
                    "PxPresentationTransform scale ({:.2}, {:.2}) has non-positive axis — \
                     clamped to minimum {MIN_SCALE}",
                    self.scale.x, self.scale.y,
                );
            }
        }

        s
    }
}
