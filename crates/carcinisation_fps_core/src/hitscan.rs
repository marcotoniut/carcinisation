//! Per-part hitscan resolution against animated billboard targets.
//!
//! This is the gameplay-facing layer that ties the [`FirePose2d`] weapon pose,
//! wall obstruction, and per-part collision frames together. It replaces the
//! legacy centre/radius approximation (`hitscan_generic_from_pose`) for
//! pistol/hitscan target selection while preserving its gating semantics:
//!
//! - ray origin/direction derive from `yaw` only; `visual_pitch_px` is ignored
//! - hits beyond the first wall are rejected
//! - the nearest *surface* hit across all targets wins (not nearest centre)
//!
//! Targets carry a reference to a [`TargetCollisionSet`] plus the animation key
//! and frame index to look up. The [`BillboardFacing8`] is computed per target
//! from the attacker position, so two attackers may resolve different facings
//! for the same target in the same tick.
//!
//! # Fallback policy
//!
//! If a target has no collision frame for the requested animation/frame/facing
//! — or the frame exists but is **empty** (zero parts) — the query falls back
//! to a single whole-body circle of `fallback_radius` centred on the target,
//! tagged [`PartId::FALLBACK`]. This is a deliberate *fail-open* choice:
//! missing or empty authoring data must not make a target invulnerable. The
//! circle baseline is the contract the legacy hitscan already guaranteed, so
//! falling back to it preserves gameplay rather than silently dropping hits. A
//! non-empty frame whose parts the ray misses is a genuine miss, not a
//! fallback.

use bevy_math::Vec2;

use crate::collision::primitives::HitResult;
use crate::collision::target::{AnimationKey, PartId, PartReactionProfile};
use crate::collision::{Circle, TargetCollisionSet, TargetQueryPose2d, swept_circle_vs_circle};
use crate::combat::{FirePose2d, wall_obstruction_distance, wall_obstruction_distance_for_pose};
use crate::config::PlayerFlamethrowerConfig;
use crate::map::Map;

/// A single hitscan candidate target.
///
/// Geometry is resolved from `set` via (`animation`, `frame`, computed facing).
/// `fallback_radius` is used only when no matching frame exists.
#[derive(Clone, Copy)]
pub struct PartHitscanTarget<'a> {
    pub position: Vec2,
    /// Target facing yaw (radians). Local part colliders are authored relative
    /// to this. Pass `0.0` for targets whose facing is not yet authoritative.
    pub yaw: f32,
    pub alive: bool,
    pub set: &'a TargetCollisionSet,
    pub animation: AnimationKey,
    pub frame: u16,
    /// Whole-body circle radius used when no collision frame is found.
    pub fallback_radius: f32,
}

/// Result of a per-part hitscan query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartHitscanResult {
    /// Index of the hit target within the queried iterator.
    pub target_idx: usize,
    /// Which part was hit. [`PartId::FALLBACK`] when the whole-body circle
    /// fallback was used.
    pub part_id: PartId,
    /// Damage multiplier of the hit part (`PartMetadata::damage_scale`).
    /// [`NEUTRAL_DAMAGE_SCALE`] for the fallback circle and for parts with no
    /// registered metadata.
    pub damage_scale: f32,
    /// Flat armour of the hit part (`PartMetadata::armour`), subtracted after
    /// scaling. `0.0` for the fallback circle and parts with no metadata.
    pub armour: f32,
    /// Per-part reaction modifiers of the hit part (`PartMetadata::reaction`).
    /// [`PartReactionProfile::NEUTRAL`] for the fallback circle and parts with
    /// no metadata, so reactions are unchanged unless explicitly authored.
    pub reaction: PartReactionProfile,
    /// Distance from ray origin to the hit surface.
    pub distance: f32,
    /// World-space hit point on the part surface.
    pub point: Vec2,
}

impl PartHitscanResult {
    /// Final damage for this hit: `(base × damage_scale) − armour`, clamped.
    /// Convenience over [`routed_damage`] using this result's routing fields.
    #[must_use]
    pub fn routed_damage(&self, base: f32) -> f32 {
        routed_damage(base, self.damage_scale, self.armour)
    }
}

/// Damage multiplier meaning "no authored damage modifier" — the identity
/// scale used for fallback hits, parts without metadata, and any neutral
/// routing path.
///
/// This is an **engine invariant, not gameplay tuning**: it must stay `1.0`
/// (multiplying by it is a no-op) and must not be made configurable. Authored
/// per-part multipliers live in [`PartMetadata::damage_scale`].
pub const NEUTRAL_DAMAGE_SCALE: f32 = 1.0;

/// `tracing` target for opt-in per-hit combat debug logs (single-player and
/// server hitscan + flame). Logs are emitted at `trace` level, so they are
/// **disabled by default** with negligible cost when off (the macro short-
/// circuits before formatting). Enable at runtime with a log filter, e.g.
/// `RUST_LOG=carcinisation::fps::hit=trace`, to surface per-shot routing
/// (kind, part, damage_scale, armour, base, dealt, fallback usage).
///
/// Logging only — the value must never influence simulation or be branched on
/// for gameplay. It exists so per-part routing can be *observed* in the live
/// FPS, which is otherwise invisible.
pub const HIT_DEBUG_TARGET: &str = "carcinisation::fps::hit";

/// Whether a hit is a *critical* (weak-point) hit, for **presentation only**.
///
/// A hit is critical when the part's `damage_scale` exceeds the neutral
/// identity ([`NEUTRAL_DAMAGE_SCALE`]) — i.e. the part takes amplified damage
/// (e.g. a 2× headshot). The single source of truth so the server and
/// single-player classify identically; both feed the result into the hit-impact
/// feedback (a scaled blood splat), never back into damage/AI. A non-finite
/// scale is not critical.
///
/// This drives *feedback emphasis*, not gameplay: the damage itself is already
/// computed by [`routed_damage`]. Reading this flag must never alter the
/// simulation.
#[must_use]
pub fn is_critical_hit(damage_scale: f32) -> bool {
    damage_scale.is_finite() && damage_scale > NEUTRAL_DAMAGE_SCALE
}

/// Part damage routing rule: `base × scale`, clamped non-negative.
///
/// The single source of truth for per-part damage scaling, so single-player
/// and server route identically. A non-finite scale falls back to `base`
/// (neutral); a negative scale is clamped to 0. Damage stays `f32` here (the
/// server's representation); the integer-health single-player path rounds at
/// its own boundary.
///
/// # Damage pipeline contract
///
/// Canonical stage order, fixed now so future stages slot in without
/// re-deciding it per call site:
///
/// 1. **base** weapon damage (config, e.g. `hitscan_damage`)
/// 2. **× weapon/context multiplier** (e.g. the single-player melee ×3)
/// 3. **× part `damage_scale`** — [`scaled_damage`]
/// 4. **− armour flat subtraction** (ORS semantics) — [`routed_damage`]
/// 5. **clamp to ≥ 0**, then round at the integer-health boundary
///    (single-player) or keep `f32` (server)
///
/// Stages 2 and 3 are multiplicative and commute, so today's call sites
/// (which apply the weapon multiplier before scaling) already conform. Stage 4
/// does **not** commute with the multipliers — that is why armour applies
/// *after* all multipliers: flat armour is then worth the same absolute amount
/// against scaled and unscaled hits, and a weak-point hit cannot be
/// armour-immune while the body is not. Stage 4 lives in [`routed_damage`], the
/// single routing function shared by server and single-player.
#[must_use]
pub fn scaled_damage(base: f32, scale: f32) -> f32 {
    if !scale.is_finite() {
        return base;
    }
    base * scale.max(0.0)
}

/// Full per-part damage routing (stages 3–5): `(base × scale) − armour`,
/// clamped to `≥ 0`.
///
/// The single source of truth so single-player and server route identically.
/// `armour = 0.0` reproduces [`scaled_damage`] exactly (current behaviour for
/// all shipped parts). Fail-safe on bad inputs: a non-finite `scale` falls back
/// to `base` (via [`scaled_damage`]); a non-finite or negative `armour` is
/// treated as no armour. Damage stays `f32` (the server's representation); the
/// integer-health single-player path rounds at its own boundary.
#[must_use]
pub fn routed_damage(base: f32, scale: f32, armour: f32) -> f32 {
    let scaled = scaled_damage(base, scale);
    let armour = if armour.is_finite() {
        armour.max(0.0)
    } else {
        0.0
    };
    (scaled - armour).max(0.0)
}

/// Damage scale + armour + reaction profile for a part, with neutral defaults
/// when no metadata is registered ([`NEUTRAL_DAMAGE_SCALE`], `0.0` armour,
/// [`PartReactionProfile::NEUTRAL`]).
///
/// `MaterialId` is intentionally not surfaced: it has no consumer (see
/// [`MaterialId`](crate::collision::MaterialId)), so it is not threaded through
/// the hot-path result.
fn part_routing(set: &TargetCollisionSet, id: PartId) -> (f32, f32, PartReactionProfile) {
    set.part_metadata(id).map_or(
        (NEUTRAL_DAMAGE_SCALE, 0.0, PartReactionProfile::NEUTRAL),
        |m| (m.damage_scale, m.armour, m.reaction),
    )
}

/// Resolve the nearest per-part hit for a fire pose against a set of targets.
///
/// Returns the closest surface hit nearer than the first wall, or `None` if no
/// target is hit. Equal-distance ties break by lower `target_idx`, then by
/// lower [`PartId`] (the latter handled inside the frame query).
pub fn hitscan_parts_from_pose<'a>(
    pose: FirePose2d,
    map: &Map,
    targets: impl Iterator<Item = PartHitscanTarget<'a>>,
) -> Option<PartHitscanResult> {
    let origin = pose.origin_xy;
    let dir = pose.direction();
    let wall_dist = wall_obstruction_distance_for_pose(map, pose, f32::MAX);

    let mut best: Option<PartHitscanResult> = None;

    for (idx, target) in targets.enumerate() {
        if !target.alive {
            continue;
        }

        let target_pose = TargetQueryPose2d::new(target.position, target.yaw);
        let facing = target_pose.facing_for_attacker(origin);

        let resolved = match target.set.lookup(target.animation, target.frame, facing) {
            Some(frame) if !frame.is_empty() => frame
                // Damage routes only to targetable parts; a non-targetable part
                // is transparent here (the ray continues to the nearest
                // targetable part behind it). An all-non-targetable frame yields
                // no hit (and no fallback — the frame is authored, just inert).
                .nearest_world_ray_hit_filtered(target_pose, origin, dir, |id| {
                    target.set.is_targetable(id)
                })
                .map(|hit| {
                    let (scale, armour, reaction) = part_routing(target.set, hit.id);
                    (hit, scale, armour, reaction)
                }),
            // Missing OR empty frame falls back to the whole-body circle. An
            // empty frame (zero parts) is treated as unauthored, not as "no
            // hittable surface": bad/sparse generated metadata must not be able
            // to make a target invulnerable. A non-empty frame whose parts the
            // ray simply misses is a genuine miss (no fallback).
            _ => fallback_circle_hit(origin, dir, target.position, target.fallback_radius)
                .map(|hit| (hit, NEUTRAL_DAMAGE_SCALE, 0.0, PartReactionProfile::NEUTRAL)),
        };

        let Some((hit, damage_scale, armour, reaction)) = resolved else {
            continue;
        };

        if hit.distance > wall_dist {
            continue;
        }

        let candidate = PartHitscanResult {
            target_idx: idx,
            part_id: hit.id,
            damage_scale,
            armour,
            reaction,
            distance: hit.distance,
            point: hit.point,
        };

        if best.is_none_or(|b| candidate.distance < b.distance) {
            best = Some(candidate);
        }
    }

    best
}

/// Whole-body circle fallback hit, tagged [`PartId::FALLBACK`].
fn fallback_circle_hit(
    origin: Vec2,
    dir: Vec2,
    center: Vec2,
    radius: f32,
) -> Option<HitResult<PartId>> {
    if radius <= 0.0 {
        return None;
    }
    let circle = Circle::new(center, radius);
    crate::collision::ray_vs_circle(origin, dir, &circle).map(|hit| HitResult {
        point: hit.point,
        normal: hit.normal,
        distance: hit.distance,
        id: PartId::FALLBACK,
    })
}

// ---------------------------------------------------------------------------
// Flamethrower (swept-strip per-part overlap)
// ---------------------------------------------------------------------------

/// Result of a per-part flamethrower overlap query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlamePartHit {
    /// Nearest overlapping part. [`PartId::FALLBACK`] for the whole-body circle.
    pub part_id: PartId,
    /// Distance along the flame axis to the contact.
    pub distance: f32,
    /// World-space contact point on the part surface.
    pub point: Vec2,
}

/// Wall-capped flamethrower strip, precomputed once per flame tick.
///
/// The flame is modelled as a circle of radius `half_width` swept along the
/// fire axis from the origin out to the wall-capped range — i.e. a capsule of
/// half-width `half_width`. The wall capping (a grid raycast) lives in
/// [`Self::from_pose`] so callers iterating many targets pay for it **once**,
/// not once per target; [`Self::hits_target`] then tests each candidate.
/// The stop distance is shared with the flame visuals (both use
/// [`wall_obstruction_distance_for_pose`]), so damage and visuals end at the
/// same geometry and neither passes through walls.
///
/// # Range at the far cap
///
/// The strip is a capsule, so its rounded far cap can overlap a part up to
/// `half_width` beyond the nominal `range` (when no wall caps it sooner). This
/// is an accepted ~`half_width` over-reach at the very tip — the flame has
/// width, so this reads as correct, and it avoids special-casing the cap. Wall
/// stopping is unaffected (the wall caps the length and per-contact LOS still
/// applies). Clamp the length by `half_width` here only if exact parity with
/// the old flat-ended strip is ever required.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlameStrip {
    origin: Vec2,
    end: Vec2,
    half_width: f32,
}

impl FlameStrip {
    /// Build the strip for a fire pose, capping its length at the first wall
    /// (or `range`, whichever is nearer).
    ///
    /// Returns `None` when no flame strip exists at all: non-positive range,
    /// negative half-width, degenerate direction, or origin inside a wall
    /// (zero wall-capped length).
    #[must_use]
    pub fn from_pose(pose: FirePose2d, range: f32, half_width: f32, map: &Map) -> Option<Self> {
        if range <= 0.0 || half_width < 0.0 {
            return None;
        }
        let origin = pose.origin_xy;
        let dir = pose.direction();
        if dir == Vec2::ZERO {
            return None;
        }
        let len = wall_obstruction_distance_for_pose(map, pose, range);
        if len <= 0.0 {
            return None;
        }
        Some(Self {
            origin,
            end: origin + dir * len,
            half_width,
        })
    }

    /// [`Self::from_pose`] using range/half-width from a
    /// [`PlayerFlamethrowerConfig`].
    #[must_use]
    pub fn from_config(
        pose: FirePose2d,
        map: &Map,
        cfg: &PlayerFlamethrowerConfig,
    ) -> Option<Self> {
        Self::from_pose(pose, cfg.range, cfg.hit_half_width, map)
    }

    /// Test whether the strip overlaps any of a target's part colliders.
    ///
    /// Returns the nearest overlapping part (for `PartId` tracking), or
    /// `None` if the strip misses or a wall lies between the origin and the
    /// contact point (per-contact line-of-sight — `map` is needed for that
    /// check only). Damage amount/material are not applied here.
    ///
    /// Uses the same [`PartHitscanTarget`] setup and fallback policy as
    /// [`hitscan_parts_from_pose`] — no separate target-selection path.
    #[must_use]
    pub fn hits_target(&self, map: &Map, target: PartHitscanTarget) -> Option<FlamePartHit> {
        if !target.alive {
            return None;
        }
        let origin = self.origin;

        let target_pose = TargetQueryPose2d::new(target.position, target.yaw);
        let facing = target_pose.facing_for_attacker(origin);

        let resolved = match target.set.lookup(target.animation, target.frame, facing) {
            Some(frame) if !frame.is_empty() => frame
                // Exposure routes only to targetable parts (non-targetable parts
                // are transparent to the strip).
                .nearest_world_swept_hit_filtered(
                    target_pose,
                    origin,
                    self.end,
                    self.half_width,
                    |id| target.set.is_targetable(id),
                ),
            // Missing/empty frame → fail-open whole-body swept circle (same
            // policy as the ray fallback).
            _ => fallback_swept_circle(
                origin,
                self.end,
                self.half_width,
                target.position,
                target.fallback_radius,
            ),
        };
        let hit = resolved?;

        // Per-contact line-of-sight: a wall between the origin and the contact
        // point blocks the flame. Preserves the legacy `flame_hits_position`
        // LOS and stops damage leaking past a wall corner beside the flame
        // axis.
        let to_point = hit.point - origin;
        let dist = to_point.length();
        if dist > 0.01 && wall_obstruction_distance(map, origin, to_point / dist, dist) < dist {
            return None;
        }

        Some(FlamePartHit {
            part_id: hit.id,
            distance: hit.distance,
            point: hit.point,
        })
    }
}

/// Single-target convenience over [`FlameStrip`]: build the strip and test one
/// target. Callers with more than one candidate target should build the
/// [`FlameStrip`] once and call [`FlameStrip::hits_target`] per target instead
/// — this wrapper re-runs the wall capping on every call.
#[must_use]
pub fn flame_hits_target_parts(
    pose: FirePose2d,
    range: f32,
    half_width: f32,
    map: &Map,
    target: PartHitscanTarget,
) -> Option<FlamePartHit> {
    FlameStrip::from_pose(pose, range, half_width, map)?.hits_target(map, target)
}

/// [`flame_hits_target_parts`] using range/half-width from a
/// [`PlayerFlamethrowerConfig`].
#[must_use]
pub fn flame_hits_target_parts_configured(
    pose: FirePose2d,
    map: &Map,
    cfg: &PlayerFlamethrowerConfig,
    target: PartHitscanTarget,
) -> Option<FlamePartHit> {
    flame_hits_target_parts(pose, cfg.range, cfg.hit_half_width, map, target)
}

/// Whole-body swept-circle fallback, tagged [`PartId::FALLBACK`].
fn fallback_swept_circle(
    start: Vec2,
    end: Vec2,
    sweep_radius: f32,
    center: Vec2,
    radius: f32,
) -> Option<HitResult<PartId>> {
    if radius <= 0.0 {
        return None;
    }
    let circle = Circle::new(center, radius);
    swept_circle_vs_circle(start, end, sweep_radius, &circle).map(|hit| HitResult {
        point: hit.point,
        normal: hit.normal,
        distance: hit.distance,
        id: PartId::FALLBACK,
    })
}

#[cfg(test)]
mod tests;
