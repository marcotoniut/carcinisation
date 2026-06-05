//! Headless occupancy, separation, and impulse math.
//!
//! Pure data types and functions for 3D gameplay-space occupancy. No Bevy ECS,
//! no rendering. Shared by singleplayer and server.
//!
//! Occupancy is modelled as an XZ-plane circle (radius) combined with a vertical
//! Y range (height band). Separation only applies when both XZ circles and Y
//! ranges overlap.

use bevy_math::Vec2;

// ---------------------------------------------------------------------------
// Volume
// ---------------------------------------------------------------------------

/// Occupancy volume: XZ circle + vertical Y range.
///
/// `y_min` and `y_max` are relative to the entity's effective ground position.
/// For a grounded entity, `y_min = 0.0` and `y_max` equals its gameplay height.
/// For an airborne entity, the caller applies `height_offset` to shift the range
/// before overlap testing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OccupancyVolume {
    /// XZ-plane radius (map units).
    pub radius_xz: f32,
    /// Bottom of vertical range (relative to entity origin, usually 0.0).
    pub y_min: f32,
    /// Top of vertical range (relative to entity origin).
    pub y_max: f32,
}

// ---------------------------------------------------------------------------
// Mode / Profile
// ---------------------------------------------------------------------------

/// How an entity participates in occupancy resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OccupancyMode {
    /// Normal grounded entity. Separates with others on the same Y band.
    Grounded,
    /// Airborne (hop, lunge arc). May skip separation when Y ranges diverge.
    Airborne,
    /// Ghost / dying / dead. No occupancy interaction.
    Disabled,
}

/// Coarse state-dependent occupancy profile.
///
/// Mapped to [`OccupancyMode`] via [`to_mode`](Self::to_mode). The ECS layer
/// selects a profile based on the entity's gameplay state (e.g. Spidey in
/// `LungeAttack` → `Lunging`).
///
/// Future modes (burrowed, submerged, phased) can be added when a real enemy
/// state needs them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OccupancyProfile {
    Standing,
    Lunging,
    Airborne,
    Dying,
    Disabled,
}

impl OccupancyProfile {
    #[must_use]
    pub const fn to_mode(self) -> OccupancyMode {
        match self {
            Self::Standing => OccupancyMode::Grounded,
            Self::Lunging | Self::Airborne => OccupancyMode::Airborne,
            Self::Dying | Self::Disabled => OccupancyMode::Disabled,
        }
    }
}

// ---------------------------------------------------------------------------
// Entry
// ---------------------------------------------------------------------------

/// One entity's occupancy snapshot for a single resolution pass.
#[derive(Clone, Copy, Debug)]
pub struct OccupancyEntry {
    /// XZ map-plane position.
    pub position: Vec2,
    /// Vertical offset above ground (map units). Shifts the Y range for overlap
    /// testing. For a hopping Spidey this is the hop arc height; for a grounded
    /// entity it is `0.0`.
    pub height_offset: f32,
    /// Volume shape.
    pub volume: OccupancyVolume,
    /// Current mode (derived from profile).
    pub mode: OccupancyMode,
    /// Mass-like weight for asymmetric separation. Heavier = less displaced.
    pub weight: f32,
    /// Whether this entity can be displaced by separation.
    pub pushable: bool,
    /// Strength of the separation force this entity exerts on others.
    pub separation_strength: f32,
    /// Stable identity for deterministic coincident-entity fallback direction.
    /// The caller must assign a unique, deterministic value per entity (e.g.
    /// `Entity` index or `NetworkObjectId`). Used only when two entities are
    /// at the exact same position to break directional symmetry.
    pub stable_index: u32,
}

// ---------------------------------------------------------------------------
// Impulse
// ---------------------------------------------------------------------------

/// A one-shot displacement impulse (e.g. lunge push, recoil).
///
/// Decays linearly over `duration` seconds. The caller is responsible for
/// applying the displacement returned by [`tick`](Self::tick) through
/// wall-aware movement (e.g. `try_move`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OccupancyImpulse {
    /// Push direction (normalised or zero).
    pub direction: Vec2,
    /// Initial strength in map units per second.
    pub strength: f32,
    /// Remaining lifetime (seconds). Decremented by `tick`.
    pub remaining: f32,
    /// Total lifetime at creation (seconds). Used for decay curve.
    pub duration: f32,
}

impl OccupancyImpulse {
    /// Advance the impulse by `dt` seconds, returning the displacement to apply
    /// this frame. Strength decays linearly from full at `remaining == duration`
    /// to zero at `remaining == 0`.
    ///
    /// Decay is computed from the **pre-tick** `remaining` value, then
    /// `remaining` is decremented. When `dt > remaining`, the displacement uses
    /// the full `dt` at the current decay level rather than clamping to the
    /// remaining fraction. This is consistent with how other timed systems in
    /// `carcinisation_fps_core` (snap turns, speed modifiers) handle their final
    /// tick — at 30 Hz the overshoot is at most one frame (~0.033s).
    #[must_use]
    pub fn tick(&mut self, dt: f32) -> Vec2 {
        if self.remaining <= 0.0 || self.duration <= 0.0 {
            return Vec2::ZERO;
        }
        let t = (self.remaining / self.duration).clamp(0.0, 1.0);
        let frame_strength = self.strength * t;
        let displacement = self.direction * frame_strength * dt;
        self.remaining = (self.remaining - dt).max(0.0);
        displacement
    }

    /// Whether the impulse has fully expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.remaining <= 0.0
    }
}

// ---------------------------------------------------------------------------
// Separation
// ---------------------------------------------------------------------------

/// Compute the separation displacement for one entity against all others.
///
/// Returns a `Vec2` displacement to apply (before wall collision check).
/// The caller must apply this through `try_move` or equivalent.
///
/// # Determinism contract
///
/// This function is deterministic for identical inputs in identical order.
/// If called from an ECS system, the caller **must** supply `others` in a
/// stable, deterministic order (e.g. sorted by `NetworkObjectId` or `Entity`
/// index). `HashMap` iteration order and Bevy query iteration order are **not**
/// guaranteed stable across runs and must not be relied upon.
///
/// Additional invariants maintained internally:
/// - Single pass, no iterative relaxation.
/// - Coincident entities (distance < epsilon) receive a deterministic fallback
///   push direction derived from `stable_index` ordering. The entity with the
///   lower index is pushed in `+X`, the higher in `-X`, breaking directional
///   symmetry so coincident pairs separate rather than drifting in lockstep.
///   No random jitter.
/// - No platform-specific approximation beyond IEEE 754 `f32` arithmetic
///   (same assumptions as the rest of `carcinisation_fps_core`).
#[must_use]
pub fn compute_separation(
    self_index: usize,
    entries: &[OccupancyEntry],
    max_separation_step: f32,
) -> Vec2 {
    let entity = &entries[self_index];
    if !entity.pushable || entity.mode == OccupancyMode::Disabled {
        return Vec2::ZERO;
    }

    let mut total = Vec2::ZERO;

    for (i, other) in entries.iter().enumerate() {
        if i == self_index {
            continue;
        }
        if other.mode == OccupancyMode::Disabled {
            continue;
        }

        // --- Y-range overlap check ---
        let e_y_min = entity.height_offset + entity.volume.y_min;
        let e_y_max = entity.height_offset + entity.volume.y_max;
        let o_y_min = other.height_offset + other.volume.y_min;
        let o_y_max = other.height_offset + other.volume.y_max;
        if e_y_max <= o_y_min || o_y_max <= e_y_min {
            continue;
        }

        // --- XZ overlap check ---
        let diff = entity.position - other.position;
        let dist = diff.length();
        let combined_radius = entity.volume.radius_xz + other.volume.radius_xz;

        if dist >= combined_radius {
            continue;
        }

        // Penetration depth and push direction. Coincident entities use a
        // deterministic fallback direction derived from stable_index ordering:
        // lower index → +X, higher index → -X. This breaks symmetry so
        // coincident pairs separate rather than drifting in lockstep.
        let (penetration, push_dir) = if dist < f32::EPSILON {
            let dir = if entity.stable_index <= other.stable_index {
                Vec2::X
            } else {
                Vec2::NEG_X
            };
            (combined_radius, dir)
        } else {
            (combined_radius - dist, diff / dist)
        };

        // Weight-based asymmetry: lighter entities are displaced more.
        let weight_sum = (entity.weight + other.weight).max(f32::EPSILON);
        let weight_ratio = other.weight / weight_sum;

        total += push_dir * penetration * weight_ratio * other.separation_strength;
    }

    // Clamp to prevent teleportation.
    let len = total.length();
    if len > max_separation_step && len > f32::EPSILON {
        total * (max_separation_step / len)
    } else {
        total
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn grounded_entry_idx(x: f32, y: f32, radius: f32, idx: u32) -> OccupancyEntry {
        OccupancyEntry {
            position: Vec2::new(x, y),
            height_offset: 0.0,
            volume: OccupancyVolume {
                radius_xz: radius,
                y_min: 0.0,
                y_max: 0.8,
            },
            mode: OccupancyMode::Grounded,
            weight: 1.0,
            pushable: true,
            separation_strength: 1.0,
            stable_index: idx,
        }
    }

    fn grounded_entry(x: f32, y: f32, radius: f32) -> OccupancyEntry {
        grounded_entry_idx(x, y, radius, 0)
    }

    /// Helper: compute separation for entity (index 0) against one other.
    fn sep(entity: &OccupancyEntry, other: &OccupancyEntry, max_step: f32) -> Vec2 {
        let entries = [*entity, *other];
        compute_separation(0, &entries, max_step)
    }

    const MAX_STEP: f32 = 0.15;

    // -----------------------------------------------------------------------
    // Separation — basic overlap
    // -----------------------------------------------------------------------

    #[test]
    fn no_overlap_returns_zero() {
        let entity = grounded_entry(0.0, 0.0, 0.3);
        let other = grounded_entry(2.0, 0.0, 0.3);
        let result = sep(&entity, &other, MAX_STEP);
        assert_eq!(result, Vec2::ZERO);
    }

    #[test]
    fn xz_overlap_returns_separation() {
        let entity = grounded_entry(0.0, 0.0, 0.3);
        let other = grounded_entry(0.4, 0.0, 0.3);
        let result = sep(&entity, &other, MAX_STEP);
        assert!(result.x < 0.0, "expected negative X push, got {result:?}");
        assert!(
            result.y.abs() < f32::EPSILON,
            "expected zero Y push, got {result:?}"
        );
    }

    #[test]
    fn exact_xz_boundary_returns_zero() {
        let entity = grounded_entry(0.0, 0.0, 0.3);
        // Distance = 0.6 = combined_radius → no overlap (>=).
        let other = grounded_entry(0.6, 0.0, 0.3);
        let result = sep(&entity, &other, MAX_STEP);
        assert_eq!(result, Vec2::ZERO);
    }

    // -----------------------------------------------------------------------
    // Separation — vertical overlap
    // -----------------------------------------------------------------------

    #[test]
    fn vertical_non_overlap_returns_zero() {
        let entity = grounded_entry(0.0, 0.0, 0.3);
        let mut other = grounded_entry(0.4, 0.0, 0.3);
        other.height_offset = 1.0; // Y range: 1.0..1.8, entity: 0.0..0.8
        let result = sep(&entity, &other, MAX_STEP);
        assert_eq!(result, Vec2::ZERO);
    }

    #[test]
    fn vertical_overlap_applies_separation() {
        let entity = grounded_entry(0.0, 0.0, 0.3);
        let mut other = grounded_entry(0.4, 0.0, 0.3);
        other.height_offset = 0.5; // Y range: 0.5..1.3, overlaps 0.5..0.8.
        let result = sep(&entity, &other, MAX_STEP);
        assert!(result.x < 0.0, "expected separation, got {result:?}");
    }

    #[test]
    fn exact_vertical_boundary_returns_zero() {
        let entity = grounded_entry(0.0, 0.0, 0.3);
        let mut other = grounded_entry(0.4, 0.0, 0.3);
        // Entity Y: 0.0..0.8, other Y: 0.8..1.6 — touching but not overlapping (<=).
        other.height_offset = 0.8;
        let result = sep(&entity, &other, MAX_STEP);
        assert_eq!(result, Vec2::ZERO);
    }

    // -----------------------------------------------------------------------
    // Separation — disabled entities
    // -----------------------------------------------------------------------

    #[test]
    fn disabled_subject_returns_zero() {
        let mut entity = grounded_entry(0.0, 0.0, 0.3);
        entity.mode = OccupancyMode::Disabled;
        let other = grounded_entry(0.2, 0.0, 0.3);
        let result = sep(&entity, &other, MAX_STEP);
        assert_eq!(result, Vec2::ZERO);
    }

    #[test]
    fn disabled_other_ignored() {
        let entity = grounded_entry(0.0, 0.0, 0.3);
        let mut other = grounded_entry(0.2, 0.0, 0.3);
        other.mode = OccupancyMode::Disabled;
        let result = sep(&entity, &other, MAX_STEP);
        assert_eq!(result, Vec2::ZERO);
    }

    // -----------------------------------------------------------------------
    // Separation — pushable / non-pushable
    // -----------------------------------------------------------------------

    #[test]
    fn non_pushable_subject_returns_zero() {
        let mut entity = grounded_entry(0.0, 0.0, 0.3);
        entity.pushable = false;
        let other = grounded_entry(0.2, 0.0, 0.3);
        let result = sep(&entity, &other, MAX_STEP);
        assert_eq!(result, Vec2::ZERO);
    }

    #[test]
    fn non_pushable_other_still_exerts_separation() {
        let entity = grounded_entry(0.0, 0.0, 0.3);
        let mut other = grounded_entry(0.4, 0.0, 0.3);
        other.pushable = false;
        let result = sep(&entity, &other, MAX_STEP);
        assert!(
            result.x < 0.0,
            "non-pushable other should still push entity, got {result:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Separation — coincident entities
    // -----------------------------------------------------------------------

    #[test]
    fn coincident_entities_lower_index_pushed_positive_x() {
        let entity = grounded_entry_idx(1.0, 1.0, 0.3, 0);
        let other = grounded_entry_idx(1.0, 1.0, 0.3, 1);
        let result = sep(&entity, &other, MAX_STEP);
        assert!(result.x > 0.0, "lower index should get +X, got {result:?}");
    }

    #[test]
    fn coincident_entities_higher_index_pushed_negative_x() {
        let entity = grounded_entry_idx(1.0, 1.0, 0.3, 1);
        let other = grounded_entry_idx(1.0, 1.0, 0.3, 0);
        let result = sep(&entity, &other, MAX_STEP);
        assert!(result.x < 0.0, "higher index should get -X, got {result:?}");
    }

    #[test]
    fn coincident_pair_separates_in_opposite_directions() {
        let a = grounded_entry_idx(1.0, 1.0, 0.3, 0);
        let b = grounded_entry_idx(1.0, 1.0, 0.3, 1);
        let entries = [a, b];
        let push_a = compute_separation(0, &entries, 1.0);
        let push_b = compute_separation(1, &entries, 1.0);
        assert!(
            push_a.x > 0.0 && push_b.x < 0.0,
            "pair should separate: a={push_a:?}, b={push_b:?}"
        );
    }

    #[test]
    fn coincident_fallback_uses_proportional_formula() {
        let entity = grounded_entry_idx(0.0, 0.0, 0.3, 0);
        let other = grounded_entry_idx(0.0, 0.0, 0.3, 1);
        let coincident = sep(&entity, &other, 1.0);

        let near_entity = grounded_entry(0.0, 0.0, 0.3);
        let near_other = grounded_entry(0.001, 0.0, 0.3);
        let near = sep(&near_entity, &near_other, 1.0);

        let ratio = coincident.length() / near.length();
        assert!(
            (ratio - 1.0).abs() < 0.01,
            "coincident and near-coincident should be proportional, ratio={ratio}"
        );
    }

    #[test]
    fn coincident_fallback_clamped_by_max_step() {
        let entity = grounded_entry_idx(0.0, 0.0, 0.5, 0);
        let other = grounded_entry_idx(0.0, 0.0, 0.5, 1);
        let tiny_max = 0.01;
        let result = sep(&entity, &other, tiny_max);
        let len = result.length();
        assert!(
            len <= tiny_max + f32::EPSILON,
            "coincident fallback should be clamped to {tiny_max}, got {len}"
        );
    }

    // -----------------------------------------------------------------------
    // Separation — weight asymmetry
    // -----------------------------------------------------------------------

    #[test]
    fn weight_asymmetry() {
        let entity_light = grounded_entry(0.0, 0.0, 0.3);
        let mut other_heavy = grounded_entry(0.4, 0.0, 0.3);
        other_heavy.weight = 5.0;
        let light_result = sep(&entity_light, &other_heavy, 1.0);

        let mut entity_heavy = grounded_entry(0.0, 0.0, 0.3);
        entity_heavy.weight = 5.0;
        let other_light = grounded_entry(0.4, 0.0, 0.3);
        let heavy_result = sep(&entity_heavy, &other_light, 1.0);

        assert!(
            light_result.length() > heavy_result.length(),
            "lighter entity should be displaced more: light={}, heavy={}",
            light_result.length(),
            heavy_result.length(),
        );
    }

    // -----------------------------------------------------------------------
    // Separation — max clamp
    // -----------------------------------------------------------------------

    #[test]
    fn max_separation_step_clamps() {
        let entity = grounded_entry(0.0, 0.0, 0.5);
        let other = grounded_entry(0.1, 0.0, 0.5);
        let tiny_max = 0.01;
        let result = sep(&entity, &other, tiny_max);
        let len = result.length();
        assert!(
            len <= tiny_max + f32::EPSILON,
            "expected clamped to {tiny_max}, got {len}"
        );
    }

    // -----------------------------------------------------------------------
    // Separation — airborne Y-elevation
    // -----------------------------------------------------------------------

    #[test]
    fn airborne_entity_above_grounded_skips_separation() {
        let grounded = grounded_entry(3.5, 3.5, 0.3);
        let mut airborne = grounded_entry(3.6, 3.5, 0.3);
        // Elevated above body_height (0.8): Y range becomes 0.9..1.7,
        // no overlap with grounded 0.0..0.8.
        airborne.height_offset = 0.9;
        airborne.mode = OccupancyMode::Airborne;
        let result = sep(&grounded, &airborne, MAX_STEP);
        assert_eq!(
            result,
            Vec2::ZERO,
            "elevated airborne should skip separation"
        );
    }

    #[test]
    fn airborne_entity_partially_overlapping_still_separates() {
        let grounded = grounded_entry(3.5, 3.5, 0.3);
        let mut airborne = grounded_entry(3.6, 3.5, 0.3);
        // Partially elevated: Y range 0.4..1.2, overlaps grounded 0.0..0.8 at 0.4..0.8.
        airborne.height_offset = 0.4;
        airborne.mode = OccupancyMode::Airborne;
        let result = sep(&grounded, &airborne, MAX_STEP);
        assert!(
            result != Vec2::ZERO,
            "partially overlapping airborne should still separate"
        );
    }

    // -----------------------------------------------------------------------
    // Impulse
    // -----------------------------------------------------------------------

    #[test]
    fn impulse_tick_produces_displacement() {
        let mut impulse = OccupancyImpulse {
            direction: Vec2::X,
            strength: 4.0,
            remaining: 0.3,
            duration: 0.3,
        };
        let dt = 1.0 / 30.0;
        let disp = impulse.tick(dt);
        let expected = 4.0 * dt;
        assert!(
            (disp.x - expected).abs() < 0.001,
            "expected ~{expected}, got {}",
            disp.x
        );
        assert!(disp.y.abs() < f32::EPSILON);
    }

    #[test]
    fn impulse_decays_over_time() {
        let mut impulse = OccupancyImpulse {
            direction: Vec2::X,
            strength: 4.0,
            remaining: 0.3,
            duration: 0.3,
        };
        let dt = 0.1;
        let first = impulse.tick(dt);
        let second = impulse.tick(dt);
        let third = impulse.tick(dt);
        assert!(
            first.x > second.x,
            "first ({}) should exceed second ({})",
            first.x,
            second.x
        );
        assert!(
            second.x > third.x,
            "second ({}) should exceed third ({})",
            second.x,
            third.x
        );
    }

    #[test]
    fn impulse_expires() {
        let mut impulse = OccupancyImpulse {
            direction: Vec2::X,
            strength: 4.0,
            remaining: 0.1,
            duration: 0.3,
        };
        assert!(!impulse.is_expired());
        let _ = impulse.tick(0.2);
        assert!(impulse.is_expired());
        let disp = impulse.tick(0.1);
        assert_eq!(disp, Vec2::ZERO);
    }

    #[test]
    fn zero_duration_impulse_produces_zero() {
        let mut impulse = OccupancyImpulse {
            direction: Vec2::X,
            strength: 4.0,
            remaining: 0.0,
            duration: 0.0,
        };
        assert!(impulse.is_expired());
        let disp = impulse.tick(0.033);
        assert_eq!(disp, Vec2::ZERO);
    }

    /// When `dt` exceeds `remaining`, the impulse computes displacement using
    /// the pre-tick decay level and the full `dt`. This is consistent with how
    /// snap turns and speed modifiers handle their final tick.
    #[test]
    fn impulse_dt_exceeds_remaining() {
        let mut impulse = OccupancyImpulse {
            direction: Vec2::X,
            strength: 4.0,
            remaining: 0.01,
            duration: 0.3,
        };
        let disp = impulse.tick(0.1);
        assert!(disp.x > 0.0, "should still produce displacement");
        assert!(impulse.is_expired());
    }

    // -----------------------------------------------------------------------
    // Profile mapping
    // -----------------------------------------------------------------------

    #[test]
    fn profile_mapping() {
        assert_eq!(
            OccupancyProfile::Standing.to_mode(),
            OccupancyMode::Grounded
        );
        assert_eq!(OccupancyProfile::Lunging.to_mode(), OccupancyMode::Airborne);
        assert_eq!(
            OccupancyProfile::Airborne.to_mode(),
            OccupancyMode::Airborne
        );
        assert_eq!(OccupancyProfile::Dying.to_mode(), OccupancyMode::Disabled);
        assert_eq!(
            OccupancyProfile::Disabled.to_mode(),
            OccupancyMode::Disabled
        );
    }
}
