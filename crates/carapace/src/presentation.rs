//! Visual-only presentation transforms applied at render time.
//!
//! [`CxPresentationTransform`] provides per-entity scaling, rotation, and offset
//! that affect rendering and visual-space collision **without** modifying the
//! gameplay position (`WorldPos`).
//!
//! # Position spaces
//!
//! See [crate-level docs](crate#coordinate-spaces) for the full space
//! definitions.  In brief:
//!
//! - **World space** — `WorldPos` (and derived `CxPosition`). Read by
//!   simulation: physics, AI, spawn placement. Unaffected by presentation.
//!
//! - **Visual space** — world position plus composed presentation offsets.
//!   Read by rendering, collision hit-detection, debug overlays — anything
//!   that must align with what the player sees.
//!
//! Simulation logic reads `WorldPos`; anything that must align with what
//! the player sees reads the appropriate offset field on this component.
//!
//! # Offset categories
//!
//! Presentation offsets come in two categories:
//!
//! - **Collision-affecting.** Spatial displacement the player perceives.
//!   Parallax (depth-weighted camera-pan shift) is this category. Future
//!   knockback recoil is likely this category. Contributes to both
//!   `visual_offset` and `collision_offset`.
//!
//! - **Visual-only.** Decorative feedback that doesn't correspond to spatial
//!   displacement. Hit-flash jiggle is this category. Contributes to
//!   `visual_offset` only.
//!
//! # Composites
//!
//! For composite sprites, parts are first assembled at native size into a scratch
//! buffer. The presentation transform is then applied to the **composed result**.
//!
//! Individual parts may have their own [`PartTransform`](crate::sprite::PartTransform),
//! but that is a separate concept applied **during** composition, not after.
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
//! transforms. Entities with this component that also carry `CxGpuSprite` /
//! `CxGpuComposite` will render at native size on the GPU path.

use crate::prelude::*;

/// Minimum allowed scale per axis. Values below this (including NaN) are
/// clamped to avoid degenerate sampling in the render path.
pub(crate) const MIN_SCALE: f32 = 0.01;

/// Maximum allowed scale per axis in debug builds. Scales above this trigger
/// a warning — they are likely unintentional and allocate large scratch buffers.
#[cfg(debug_assertions)]
const MAX_SCALE_DEBUG_WARN: f32 = 10.0;

/// Presentation transform applied at render time and read by visual-space
/// operations (rendering, collision hit-detection, debug overlays).
///
/// Does **not** affect world-space gameplay position. Simulation systems
/// (physics, AI, spawn placement) must read `WorldPos` directly.
///
/// When absent or at default values, the rendering path is completely unchanged
/// (no scratch buffer allocated, no extra sampling).
///
/// # Four invariants
///
/// 1. **World-space vs visual-space.** `WorldPos` is world space, read by
///    simulation. This component carries the visual-space displacement. Do not
///    use for simulation logic.
///
/// 2. **Two offset categories.** `visual_offset` = sum of ALL presentation
///    offset contributors. `collision_offset` = sum of collision-affecting
///    contributors only. Rendering reads `visual_offset`; collision state
///    computation reads `collision_offset`. Visual-only effects (hit-flash,
///    cosmetic animation) contribute only to `visual_offset`.
///
/// 3. **Fully recomputed each frame** by `compose_presentation_offsets`. Do not
///    accumulate. The composition system writes fresh values every tick (or
///    skips the write when contributors haven't changed).
///
/// 4. **Single writer for composed offset fields.** `compose_presentation_offsets`
///    is the sole system that writes `visual_offset` or `collision_offset`.
///    Future effects register a contributor component and add themselves to
///    the composition sum — they do not write these fields directly.
///
/// # Scale
///
/// `scale` controls per-axis nearest-neighbour scaling around the entity's
/// anchor point. `Vec2::ONE` means native size. Negative values produce
/// render-only mirroring. NaN is treated as `MIN_SCALE`.
///
/// # Rotation
///
/// `rotation` is in radians, counter-clockwise, applied around the anchor
/// point. NaN is treated as 0.0.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct CxPresentationTransform {
    /// Non-uniform scale factor. `Vec2::ONE` = native size. Negative = flip.
    pub scale: Vec2,
    /// Rotation in radians, counter-clockwise around the anchor point.
    pub rotation: f32,
    /// Sum of ALL presentation offset contributors. Read by the rendering
    /// pipeline for final visual position. Always a superset of
    /// `collision_offset` (`visual_offset = collision_offset + visual-only`
    /// contributors).
    pub visual_offset: Vec2,
    /// Sum of collision-affecting presentation offset contributors only.
    /// Read by collision state computation to align hitboxes with the
    /// visible sprite position. Visual-only effects (hit-flash, cosmetic
    /// animation) do NOT contribute here.
    ///
    /// When a visual-only offset contributor is added (e.g., hit-jiggle),
    /// it contributes to `visual_offset` but not to `collision_offset`.
    /// Until then, both fields are equal.
    pub collision_offset: Vec2,
}

impl Default for CxPresentationTransform {
    fn default() -> Self {
        Self {
            scale: Vec2::ONE,
            rotation: 0.0,
            visual_offset: Vec2::ZERO,
            collision_offset: Vec2::ZERO,
        }
    }
}

impl CxPresentationTransform {
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

    /// Returns true if `visual_offset` differs meaningfully from `Vec2::ZERO`.
    #[must_use]
    pub fn has_offset(&self) -> bool {
        self.visual_offset.length_squared() >= f32::EPSILON
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
                    "CxPresentationTransform scale ({:.2}, {:.2}) exceeds {MAX_SCALE_DEBUG_WARN}x — \
                     this allocates a large scratch buffer and is likely unintentional",
                    s.x, s.y,
                );
            }
            if (self.scale.x.abs() < MIN_SCALE && !self.scale.x.is_nan())
                || (self.scale.y.abs() < MIN_SCALE && !self.scale.y.is_nan())
            {
                warn!(
                    "CxPresentationTransform scale ({:.2}, {:.2}) has near-zero axis — \
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
        let r = sanitise_rotation(self.rotation);
        #[cfg(debug_assertions)]
        if self.rotation.is_nan() {
            warn!("CxPresentationTransform rotation is NaN — treated as 0.0");
        }
        r
    }
}

/// Sanitises a rotation value: NaN → 0.0.
///
/// Shared by both entity-level [`CxPresentationTransform`] and part-level
/// [`PartTransform`](crate::sprite::PartTransform).
pub(crate) fn sanitise_rotation(rotation: f32) -> f32 {
    if rotation.is_nan() { 0.0 } else { rotation }
}

/// Clamps a single scale axis, preserving sign.
///
/// - NaN → `+MIN_SCALE`
/// - Magnitude below `MIN_SCALE` → sign × `MIN_SCALE`
/// - Otherwise → unchanged
///
/// Shared by both entity-level [`CxPresentationTransform`] and part-level
/// [`PartTransform`](crate::sprite::PartTransform).
pub(crate) fn clamp_scale_axis(v: f32) -> f32 {
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
        let pt = CxPresentationTransform::default();
        assert!(pt.is_identity());
        assert!(!pt.has_scale());
        assert!(!pt.has_rotation());
        assert!(!pt.has_offset());
        assert!(!pt.needs_transformed_blit());
    }

    #[test]
    fn scaled_constructor() {
        let pt = CxPresentationTransform::scaled(2.0);
        assert!(pt.has_scale());
        assert!(!pt.has_rotation());
        assert!(pt.needs_transformed_blit());
        assert!(!pt.is_identity());
    }

    #[test]
    fn flipped_constructor() {
        let pt = CxPresentationTransform::flipped(true, false);
        assert!(pt.has_scale());
        assert!((pt.scale.x - (-1.0)).abs() < f32::EPSILON);
        assert!((pt.scale.y - 1.0).abs() < f32::EPSILON);
        assert!(pt.needs_transformed_blit());

        let both = CxPresentationTransform::flipped(true, true);
        assert!((both.scale.x - (-1.0)).abs() < f32::EPSILON);
        assert!((both.scale.y - (-1.0)).abs() < f32::EPSILON);

        let none = CxPresentationTransform::flipped(false, false);
        assert!(none.is_identity());
    }

    #[test]
    fn rotated_constructor() {
        let pt = CxPresentationTransform::rotated(0.5);
        assert!(!pt.has_scale());
        assert!(pt.has_rotation());
        assert!(pt.needs_transformed_blit());
    }

    #[test]
    fn offset_only_does_not_need_blit() {
        let pt = CxPresentationTransform {
            visual_offset: Vec2::new(3.0, -2.0),
            collision_offset: Vec2::new(3.0, -2.0),
            ..Default::default()
        };
        assert!(pt.has_offset());
        assert!(!pt.needs_transformed_blit());
        assert!(!pt.is_identity());
    }

    #[test]
    fn clamped_scale_floors_magnitude_preserves_sign() {
        let pt = CxPresentationTransform {
            scale: Vec2::new(0.001, -0.001),
            ..Default::default()
        };
        let s = pt.clamped_scale();
        assert_eq!(s.x, MIN_SCALE);
        assert_eq!(s.y, -MIN_SCALE);
    }

    #[test]
    fn clamped_scale_preserves_negative() {
        let pt = CxPresentationTransform {
            scale: Vec2::new(-2.0, -5.0),
            ..Default::default()
        };
        let s = pt.clamped_scale();
        assert!((s.x - (-2.0)).abs() < f32::EPSILON);
        assert!((s.y - (-5.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn clamped_scale_handles_nan() {
        let pt = CxPresentationTransform {
            scale: Vec2::new(f32::NAN, 1.0),
            ..Default::default()
        };
        let s = pt.clamped_scale();
        assert_eq!(s.x, MIN_SCALE);
        assert_eq!(s.y, 1.0);
    }

    #[test]
    fn clamped_scale_passes_through_valid() {
        let pt = CxPresentationTransform::scaled(3.5);
        let s = pt.clamped_scale();
        assert!((s.x - 3.5).abs() < f32::EPSILON);
        assert!((s.y - 3.5).abs() < f32::EPSILON);
    }

    #[test]
    fn sanitised_rotation_passes_valid() {
        let pt = CxPresentationTransform::rotated(1.23);
        assert!((pt.sanitised_rotation() - 1.23).abs() < f32::EPSILON);
    }

    #[test]
    fn sanitised_rotation_handles_nan() {
        let pt = CxPresentationTransform {
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
