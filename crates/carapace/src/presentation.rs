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
//! # Transform ordering
//!
//! The transform is applied in this order:
//! 1. Scale (around anchor)
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
/// anchor point. `Vec2::ONE` means native size. Values below [`MIN_SCALE`]
/// (including NaN and negative values) are clamped automatically — use flip
/// flags on sprites/parts for mirroring, not negative scale.
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
    /// Non-uniform scale factor. `Vec2::ONE` = native size.
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

    /// Returns the scale with each axis clamped to `[MIN_SCALE, ∞)`.
    /// NaN is treated as `MIN_SCALE`.
    ///
    /// In debug builds, warns on non-positive or excessively large scale.
    #[must_use]
    pub(crate) fn clamped_scale(&self) -> Vec2 {
        let s = Vec2::new(
            clamp_scale_axis(self.scale.x),
            clamp_scale_axis(self.scale.y),
        );

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

/// Clamps a single scale axis: NaN or values below `MIN_SCALE` become `MIN_SCALE`.
fn clamp_scale_axis(v: f32) -> f32 {
    if v.is_nan() || v < MIN_SCALE {
        MIN_SCALE
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
    fn clamped_scale_floors_to_min() {
        let pt = PxPresentationTransform {
            scale: Vec2::new(0.0, -5.0),
            ..Default::default()
        };
        let s = pt.clamped_scale();
        assert_eq!(s.x, MIN_SCALE);
        assert_eq!(s.y, MIN_SCALE);
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
    fn clamp_scale_axis_negative() {
        assert_eq!(clamp_scale_axis(-1.0), MIN_SCALE);
    }

    #[test]
    fn clamp_scale_axis_normal() {
        assert!((clamp_scale_axis(2.5) - 2.5).abs() < f32::EPSILON);
    }
}
