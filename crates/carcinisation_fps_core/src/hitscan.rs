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
use crate::collision::target::{AnimationKey, MaterialId, PartId};
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
    /// Material of the hit part, if registered. `None` for fallback hits.
    pub material: Option<MaterialId>,
    /// Distance from ray origin to the hit surface.
    pub distance: f32,
    /// World-space hit point on the part surface.
    pub point: Vec2,
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
                .nearest_world_ray_hit(target_pose, origin, dir)
                .map(|hit| (hit, target.set.part_metadata(hit.id).map(|m| m.material))),
            // Missing OR empty frame falls back to the whole-body circle. An
            // empty frame (zero parts) is treated as unauthored, not as "no
            // hittable surface": bad/sparse generated metadata must not be able
            // to make a target invulnerable. A non-empty frame whose parts the
            // ray simply misses is a genuine miss (no fallback).
            _ => fallback_circle_hit(origin, dir, target.position, target.fallback_radius)
                .map(|hit| (hit, None)),
        };

        let Some((hit, material)) = resolved else {
            continue;
        };

        if hit.distance > wall_dist {
            continue;
        }

        let candidate = PartHitscanResult {
            target_idx: idx,
            part_id: hit.id,
            material,
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
    /// Material of the hit part, if registered. `None` for fallback hits.
    pub material: Option<MaterialId>,
    /// Distance along the flame axis to the contact.
    pub distance: f32,
    /// World-space contact point on the part surface.
    pub point: Vec2,
}

/// Test whether a flamethrower strip overlaps any of a target's part colliders.
///
/// The flame is modelled as a circle of radius `half_width` swept along the
/// fire axis from the origin out to the wall-capped range — i.e. a capsule of
/// half-width `half_width`. This shares its stop distance with the flame
/// visuals (both use [`wall_obstruction_distance_for_pose`]), so damage and
/// visuals end at the same geometry and neither passes through walls.
///
/// Returns the nearest overlapping part (for `PartId` tracking), or `None` if
/// the strip misses, is fully wall-blocked, or a wall lies between the origin
/// and the contact point. Damage amount/material are not applied here.
///
/// Uses the same [`PartHitscanTarget`] setup and fallback policy as
/// [`hitscan_parts_from_pose`] — no separate target-selection path.
#[must_use]
pub fn flame_hits_target_parts(
    pose: FirePose2d,
    range: f32,
    half_width: f32,
    map: &Map,
    target: PartHitscanTarget,
) -> Option<FlamePartHit> {
    if !target.alive || range <= 0.0 || half_width < 0.0 {
        return None;
    }
    let origin = pose.origin_xy;
    let dir = pose.direction();
    if dir == Vec2::ZERO {
        return None;
    }

    // Shared stop distance with the flame visuals: the strip stops at the first
    // wall (or `range`, whichever is nearer). Zero when the origin is in a wall.
    let len = wall_obstruction_distance_for_pose(map, pose, range);
    if len <= 0.0 {
        return None;
    }
    let end = origin + dir * len;

    let target_pose = TargetQueryPose2d::new(target.position, target.yaw);
    let facing = target_pose.facing_for_attacker(origin);

    let resolved = match target.set.lookup(target.animation, target.frame, facing) {
        Some(frame) if !frame.is_empty() => frame
            .nearest_world_swept_hit(target_pose, origin, end, half_width)
            .map(|hit| (hit, target.set.part_metadata(hit.id).map(|m| m.material))),
        // Missing/empty frame → fail-open whole-body swept circle (same policy
        // as the ray fallback).
        _ => fallback_swept_circle(
            origin,
            end,
            half_width,
            target.position,
            target.fallback_radius,
        )
        .map(|hit| (hit, None)),
    };
    let (hit, material) = resolved?;

    // Per-contact line-of-sight: a wall between the origin and the contact
    // point blocks the flame. Preserves the legacy `flame_hits_position` LOS
    // and stops damage leaking past a wall corner beside the flame axis.
    let to_point = hit.point - origin;
    let dist = to_point.length();
    if dist > 0.01 && wall_obstruction_distance(map, origin, to_point / dist, dist) < dist {
        return None;
    }

    Some(FlamePartHit {
        part_id: hit.id,
        material,
        distance: hit.distance,
        point: hit.point,
    })
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
