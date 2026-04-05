//! Visual-only presentation transforms applied at render time.
//!
//! [`PxPresentationTransform`] provides per-entity scaling, rotation, and offset
//! that affect rendering **without** modifying gameplay position, collision, or anchoring.
//!
//! # Render-only
//!
//! This component is consumed exclusively by the CPU rendering pipeline in
//! [`draw_spatial_transformed`](crate::frame::draw_spatial_transformed) and the
//! composite transform path in [`draw_layers`](crate::screen::draw::draw_layers).
//! Gameplay systems should never read or depend on it.
//!
//! # Composites
//!
//! For composite sprites, parts are first assembled at native size into a scratch
//! buffer. The presentation transform is then applied to the **composed result** —
//! individual parts are never transformed independently.
//!
//! # Flipping / mirroring
//!
//! Negative scale values produce render-only mirroring:
//! - `scale.x < 0` → horizontal flip
//! - `scale.y < 0` → vertical flip
//! - both negative → flip on both axes
//!
//! The flip falls out naturally from signed scale in the inverse-transform
//! sampling — no separate flip flags or coordinate swaps are needed.
//! For composites, flipping applies to the **composed result** (individual
//! parts are not flipped independently by this component).
//!
//! # Transform ordering
//!
//! The transform is applied in this order:
//! 1. Scale (around anchor) — sign controls mirroring, magnitude controls size
//! 2. Rotation (around anchor)
//! 3. Offset (pixel displacement, post-transform)
//!
//! # GPU palette path
//!
//! The `gpu_palette` render path does **not** currently support presentation
//! transforms. Entities with this component that also carry `PxGpuSprite` /
//! `PxGpuComposite` will render at native size on the GPU path.

use crate::prelude::*;

/// Minimum allowed scale per axis. Values below this (including NaN) are
/// clamped to avoid degenerate sampling in the render path.
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
/// anchor point. `Vec2::ONE` means native size.
///
/// **Negative values** produce render-only mirroring (horizontal and/or
/// vertical flip). The sign is preserved; only the magnitude is clamped
/// to `[MIN_SCALE, ∞)`. NaN is treated as `MIN_SCALE`.
///
/// # Rotation
///
/// `rotation` is in radians, counter-clockwise, applied around the anchor
/// point. The destination bounding box expands automatically to avoid clipping.
/// Pixel art at non-90° angles will show expected nearest-neighbour artefacts.
/// NaN is treated as 0.0.
///
/// # Offset
///
/// `offset` is an additional pixel displacement applied after scale/rotation,
/// useful for shake or recoil effects.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct PxPresentationTransform {
    /// Non-uniform scale factor. `Vec2::ONE` = native size. Negative = flip.
    pub scale: Vec2,
    /// Rotation in radians, counter-clockwise around the anchor point.
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
    /// Uniform scale only, no rotation or offset.
    #[must_use]
    pub fn scaled(factor: f32) -> Self {
        Self {
            scale: Vec2::splat(factor),
            ..Default::default()
        }
    }

    /// Flip on one or both axes at native size, no rotation or offset.
    ///
    /// `flip_x = true` → `scale.x = -1.0` (horizontal mirror).
    /// `flip_y = true` → `scale.y = -1.0` (vertical mirror).
    #[must_use]
    pub fn flipped(flip_x: bool, flip_y: bool) -> Self {
        Self {
            scale: Vec2::new(
                if flip_x { -1.0 } else { 1.0 },
                if flip_y { -1.0 } else { 1.0 },
            ),
            ..Default::default()
        }
    }

    /// Rotation only (radians), no scale or offset.
    #[must_use]
    pub fn rotated(radians: f32) -> Self {
        Self {
            rotation: radians,
            ..Default::default()
        }
    }

    /// Returns true if this transform would have no visual effect
    /// (scale ≈ 1, rotation ≈ 0, offset ≈ 0).
    #[must_use]
    pub fn is_identity(&self) -> bool {
        !self.has_scale() && !self.has_rotation() && !self.has_offset()
    }

    /// Returns true if scale differs meaningfully from `Vec2::ONE`.
    #[must_use]
    pub fn has_scale(&self) -> bool {
        (self.scale - Vec2::ONE).length_squared() >= f32::EPSILON
    }

    /// Returns true if rotation differs meaningfully from zero.
    #[must_use]
    pub fn has_rotation(&self) -> bool {
        self.rotation.abs() >= f32::EPSILON
    }

    /// Returns true if offset differs meaningfully from `Vec2::ZERO`.
    #[must_use]
    pub fn has_offset(&self) -> bool {
        self.offset.length_squared() >= f32::EPSILON
    }

    /// Returns true if this transform requires the scratch-buffer path
    /// (any scale or rotation effect).
    #[must_use]
    pub(crate) fn needs_transformed_blit(&self) -> bool {
        self.has_scale() || self.has_rotation()
    }

    /// Returns the scale with each axis's **magnitude** clamped to
    /// `[MIN_SCALE, ∞)`, preserving sign (negative = flip).
    /// NaN is treated as `+MIN_SCALE`.
    ///
    /// In debug builds, warns on near-zero or excessively large scale.
    #[must_use]
    pub(crate) fn clamped_scale(&self) -> Vec2 {
        let s = Vec2::new(
            clamp_scale_axis(self.scale.x),
            clamp_scale_axis(self.scale.y),
        );

        #[cfg(debug_assertions)]
        {
            if s.x.abs() > MAX_SCALE_DEBUG_WARN || s.y.abs() > MAX_SCALE_DEBUG_WARN {
                warn!(
                    "PxPresentationTransform scale ({:.2}, {:.2}) exceeds {MAX_SCALE_DEBUG_WARN}x — \
                     this allocates a large scratch buffer and is likely unintentional",
                    s.x, s.y,
                );
            }
            if (self.scale.x.abs() < MIN_SCALE && !self.scale.x.is_nan())
                || (self.scale.y.abs() < MIN_SCALE && !self.scale.y.is_nan())
            {
                warn!(
                    "PxPresentationTransform scale ({:.2}, {:.2}) has near-zero axis — \
                     magnitude clamped to minimum {MIN_SCALE}",
                    self.scale.x, self.scale.y,
                );
            }
        }

        s
    }

    /// Returns the rotation, with NaN treated as 0.0.
    #[must_use]
    pub(crate) fn sanitised_rotation(&self) -> f32 {
        if self.rotation.is_nan() {
            #[cfg(debug_assertions)]
            warn!("PxPresentationTransform rotation is NaN — treated as 0.0");
            return 0.0;
        }
        self.rotation
    }
}

/// Clamps a single scale axis, preserving sign.
///
/// - NaN → `+MIN_SCALE`
/// - Magnitude below `MIN_SCALE` → sign × `MIN_SCALE`
/// - Otherwise → unchanged
fn clamp_scale_axis(v: f32) -> f32 {
    if v.is_nan() {
        MIN_SCALE
    } else if v.abs() < MIN_SCALE {
        v.signum() * MIN_SCALE
    } else {
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_identity() {
        let pt = PxPresentationTransform::default();
        assert!(pt.is_identity());
        assert!(!pt.has_scale());
        assert!(!pt.has_rotation());
        assert!(!pt.has_offset());
        assert!(!pt.needs_transformed_blit());
    }

    #[test]
    fn scaled_constructor() {
        let pt = PxPresentationTransform::scaled(2.0);
        assert!(pt.has_scale());
        assert!(!pt.has_rotation());
        assert!(pt.needs_transformed_blit());
        assert!(!pt.is_identity());
    }

    #[test]
    fn flipped_constructor() {
        let pt = PxPresentationTransform::flipped(true, false);
        assert!(pt.has_scale());
        assert!((pt.scale.x - (-1.0)).abs() < f32::EPSILON);
        assert!((pt.scale.y - 1.0).abs() < f32::EPSILON);
        assert!(pt.needs_transformed_blit());

        let both = PxPresentationTransform::flipped(true, true);
        assert!((both.scale.x - (-1.0)).abs() < f32::EPSILON);
        assert!((both.scale.y - (-1.0)).abs() < f32::EPSILON);

        let none = PxPresentationTransform::flipped(false, false);
        assert!(none.is_identity());
    }

    #[test]
    fn rotated_constructor() {
        let pt = PxPresentationTransform::rotated(0.5);
        assert!(!pt.has_scale());
        assert!(pt.has_rotation());
        assert!(pt.needs_transformed_blit());
    }

    #[test]
    fn offset_only_does_not_need_blit() {
        let pt = PxPresentationTransform {
            offset: Vec2::new(3.0, -2.0),
            ..Default::default()
        };
        assert!(pt.has_offset());
        assert!(!pt.needs_transformed_blit());
        assert!(!pt.is_identity());
    }

    #[test]
    fn clamped_scale_floors_magnitude_preserves_sign() {
        let pt = PxPresentationTransform {
            scale: Vec2::new(0.001, -0.001),
            ..Default::default()
        };
        let s = pt.clamped_scale();
        assert_eq!(s.x, MIN_SCALE);
        assert_eq!(s.y, -MIN_SCALE);
    }

    #[test]
    fn clamped_scale_preserves_negative() {
        let pt = PxPresentationTransform {
            scale: Vec2::new(-2.0, -5.0),
            ..Default::default()
        };
        let s = pt.clamped_scale();
        assert!((s.x - (-2.0)).abs() < f32::EPSILON);
        assert!((s.y - (-5.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn clamped_scale_handles_nan() {
        let pt = PxPresentationTransform {
            scale: Vec2::new(f32::NAN, 1.0),
            ..Default::default()
        };
        let s = pt.clamped_scale();
        assert_eq!(s.x, MIN_SCALE);
        assert_eq!(s.y, 1.0);
    }

    #[test]
    fn clamped_scale_passes_through_valid() {
        let pt = PxPresentationTransform::scaled(3.5);
        let s = pt.clamped_scale();
        assert!((s.x - 3.5).abs() < f32::EPSILON);
        assert!((s.y - 3.5).abs() < f32::EPSILON);
    }

    #[test]
    fn sanitised_rotation_passes_valid() {
        let pt = PxPresentationTransform::rotated(1.23);
        assert!((pt.sanitised_rotation() - 1.23).abs() < f32::EPSILON);
    }

    #[test]
    fn sanitised_rotation_handles_nan() {
        let pt = PxPresentationTransform {
            rotation: f32::NAN,
            ..Default::default()
        };
        assert_eq!(pt.sanitised_rotation(), 0.0);
    }

    #[test]
    fn clamp_scale_axis_nan() {
        assert_eq!(clamp_scale_axis(f32::NAN), MIN_SCALE);
    }

    #[test]
    fn clamp_scale_axis_negative_preserved() {
        assert!((clamp_scale_axis(-1.0) - (-1.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn clamp_scale_axis_small_negative_clamped() {
        assert_eq!(clamp_scale_axis(-0.001), -MIN_SCALE);
    }

    #[test]
    fn clamp_scale_axis_zero_positive() {
        // 0.0 has positive sign in IEEE 754, magnitude < MIN_SCALE → +MIN_SCALE
        assert_eq!(clamp_scale_axis(0.0), MIN_SCALE);
    }

    #[test]
    fn clamp_scale_axis_normal() {
        assert!((clamp_scale_axis(2.5) - 2.5).abs() < f32::EPSILON);
    }
}
