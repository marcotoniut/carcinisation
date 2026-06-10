//! Nearest-hit queries across collections of colliders.
//!
//! Deterministic tie-breaking: when two hits have exactly equal distance,
//! the one with the lower index wins. This avoids platform-dependent or
//! iteration-order-dependent results.

use bevy_math::Vec2;

use super::primitives::{Collider, HitDetail, HitResult};
use super::ray;

// ---------------------------------------------------------------------------
// Ray
// ---------------------------------------------------------------------------

/// Find the nearest ray hit across a slice of colliders.
///
/// Returns the index of the hit collider and the hit detail.
/// Equal-distance ties are broken by lower index.
#[must_use]
pub fn nearest_ray_hit(
    origin: Vec2,
    direction: Vec2,
    colliders: &[Collider],
) -> Option<(usize, HitDetail)> {
    let direction_n = direction.normalize_or_zero();
    if direction_n == Vec2::ZERO {
        return None;
    }

    let mut best: Option<(usize, HitDetail)> = None;

    for (i, collider) in colliders.iter().enumerate() {
        if let Some(h) = ray_dispatch(origin, direction_n, collider)
            && is_closer_indexed(h.distance, i, &best)
        {
            best = Some((i, h));
        }
    }

    best
}

/// Find the nearest ray hit with caller-supplied identifiers.
///
/// Equal-distance ties are broken by `Id` ordering (smaller wins).
#[must_use]
pub fn nearest_ray_hit_tagged<Id: Copy + Ord>(
    origin: Vec2,
    direction: Vec2,
    targets: &[(Collider, Id)],
) -> Option<HitResult<Id>> {
    nearest_ray_hit_tagged_filter(origin, direction, targets, |_| true)
}

/// [`nearest_ray_hit_tagged`] restricted to targets whose id satisfies
/// `accept`. Rejected ids are skipped entirely (never tested, never blocking),
/// so the nearest *accepted* hit wins — the ray passes through filtered-out
/// parts. Used to route damage only to targetable parts.
pub fn nearest_ray_hit_tagged_filter<Id, F>(
    origin: Vec2,
    direction: Vec2,
    targets: &[(Collider, Id)],
    accept: F,
) -> Option<HitResult<Id>>
where
    Id: Copy + Ord,
    F: Fn(Id) -> bool,
{
    let direction_n = direction.normalize_or_zero();
    if direction_n == Vec2::ZERO {
        return None;
    }

    let mut best: Option<HitResult<Id>> = None;

    for (collider, id) in targets {
        if accept(*id)
            && let Some(h) = ray_dispatch(origin, direction_n, collider)
            && is_closer_tagged(h.distance, *id, &best)
        {
            best = Some(h.with_id(*id));
        }
    }

    best
}

// ---------------------------------------------------------------------------
// Segment
// ---------------------------------------------------------------------------

/// Find the nearest segment hit across a slice of colliders.
///
/// Returns the index of the hit collider and the hit detail.
/// Equal-distance ties are broken by lower index.
#[must_use]
pub fn nearest_segment_hit(
    start: Vec2,
    end: Vec2,
    colliders: &[Collider],
) -> Option<(usize, HitDetail)> {
    let mut best: Option<(usize, HitDetail)> = None;

    for (i, collider) in colliders.iter().enumerate() {
        if let Some(h) = super::segment_vs_collider(start, end, collider)
            && is_closer_indexed(h.distance, i, &best)
        {
            best = Some((i, h));
        }
    }

    best
}

/// Find the nearest segment hit with caller-supplied identifiers.
///
/// Equal-distance ties are broken by `Id` ordering (smaller wins).
#[must_use]
pub fn nearest_segment_hit_tagged<Id: Copy + Ord>(
    start: Vec2,
    end: Vec2,
    targets: &[(Collider, Id)],
) -> Option<HitResult<Id>> {
    let mut best: Option<HitResult<Id>> = None;

    for (collider, id) in targets {
        if let Some(h) = super::segment_vs_collider(start, end, collider)
            && is_closer_tagged(h.distance, *id, &best)
        {
            best = Some(h.with_id(*id));
        }
    }

    best
}

// ---------------------------------------------------------------------------
// Swept circle
// ---------------------------------------------------------------------------

/// Find the nearest swept-circle hit with caller-supplied identifiers.
///
/// A circle of `sweep_radius` is swept from `start` to `end`; the nearest
/// contact across `targets` is returned. Equal-distance ties break by smaller
/// `Id`.
#[must_use]
pub fn nearest_swept_circle_hit_tagged<Id: Copy + Ord>(
    start: Vec2,
    end: Vec2,
    sweep_radius: f32,
    targets: &[(Collider, Id)],
) -> Option<HitResult<Id>> {
    nearest_swept_circle_hit_tagged_filter(start, end, sweep_radius, targets, |_| true)
}

/// [`nearest_swept_circle_hit_tagged`] restricted to targets whose id satisfies
/// `accept`. Rejected ids are skipped entirely, so the strip passes through
/// filtered-out parts. Used to route flame exposure only to targetable parts.
pub fn nearest_swept_circle_hit_tagged_filter<Id, F>(
    start: Vec2,
    end: Vec2,
    sweep_radius: f32,
    targets: &[(Collider, Id)],
    accept: F,
) -> Option<HitResult<Id>>
where
    Id: Copy + Ord,
    F: Fn(Id) -> bool,
{
    let mut best: Option<HitResult<Id>> = None;

    for (collider, id) in targets {
        if accept(*id)
            && let Some(h) = super::swept_circle_vs_collider(start, end, sweep_radius, collider)
            && is_closer_tagged(h.distance, *id, &best)
        {
            best = Some(h.with_id(*id));
        }
    }

    best
}

// ---------------------------------------------------------------------------
// Dispatch helpers
// ---------------------------------------------------------------------------

/// Dispatch to pre-normalized ray `_impl` variants. Direction must already be
/// unit-length; per-primitive dimension guards are still applied internally.
fn ray_dispatch(origin: Vec2, direction: Vec2, collider: &Collider) -> Option<HitDetail> {
    match collider {
        Collider::Circle(c) => ray::ray_vs_circle_impl(origin, direction, c.center, c.radius),
        Collider::Capsule(c) => ray::ray_vs_capsule_impl(origin, direction, c),
        Collider::Obb(o) => ray::ray_vs_obb_impl(origin, direction, o),
    }
}

// ---------------------------------------------------------------------------
// Tie-breaking
// ---------------------------------------------------------------------------

/// True if `(distance, index)` is strictly closer than the current best.
fn is_closer_indexed(distance: f32, index: usize, best: &Option<(usize, HitDetail)>) -> bool {
    match best {
        None => true,
        Some((bi, bh)) => match distance.partial_cmp(&bh.distance) {
            Some(std::cmp::Ordering::Less) => true,
            Some(std::cmp::Ordering::Equal) => index < *bi,
            _ => false,
        },
    }
}

/// True if `(distance, id)` is strictly closer than the current best.
fn is_closer_tagged<Id: Copy + Ord>(distance: f32, id: Id, best: &Option<HitResult<Id>>) -> bool {
    match best {
        None => true,
        Some(b) => match distance.partial_cmp(&b.distance) {
            Some(std::cmp::Ordering::Less) => true,
            Some(std::cmp::Ordering::Equal) => id < b.id,
            _ => false,
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::collision::primitives::{Capsule, Circle, Obb};

    const EPS: f32 = 1e-4;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn nearest_ray_single_circle() {
        let colliders = [Collider::Circle(Circle::new(Vec2::new(5.0, 0.0), 1.0))];
        let (idx, hit) = nearest_ray_hit(Vec2::ZERO, Vec2::X, &colliders).unwrap();
        assert_eq!(idx, 0);
        assert!(approx(hit.distance, 4.0));
    }

    #[test]
    fn nearest_ray_picks_closest() {
        let colliders = [
            Collider::Circle(Circle::new(Vec2::new(8.0, 0.0), 1.0)), // far
            Collider::Circle(Circle::new(Vec2::new(3.0, 0.0), 1.0)), // near
        ];
        let (idx, _) = nearest_ray_hit(Vec2::ZERO, Vec2::X, &colliders).unwrap();
        assert_eq!(idx, 1, "should pick nearer circle");
    }

    #[test]
    fn nearest_ray_deterministic_tie() {
        // Two circles at equal distance on opposite sides of the ray.
        // Only the one on the ray line is hit; but if both are at exact same
        // distance, the lower index wins.
        let colliders = [
            Collider::Circle(Circle::new(Vec2::new(5.0, 0.0), 1.0)),
            Collider::Circle(Circle::new(Vec2::new(5.0, 0.0), 1.0)), // same position
        ];
        let (idx, _) = nearest_ray_hit(Vec2::ZERO, Vec2::X, &colliders).unwrap();
        assert_eq!(idx, 0, "lower index wins tie");
    }

    #[test]
    fn nearest_ray_all_miss() {
        let colliders = [
            Collider::Circle(Circle::new(Vec2::new(5.0, 5.0), 1.0)),
            Collider::Obb(Obb::axis_aligned(Vec2::new(5.0, -5.0), Vec2::new(1.0, 1.0))),
        ];
        assert!(nearest_ray_hit(Vec2::ZERO, Vec2::X, &colliders).is_none());
    }

    #[test]
    fn nearest_ray_mixed_types() {
        let colliders = [
            Collider::Obb(Obb::axis_aligned(Vec2::new(3.0, 0.0), Vec2::new(0.5, 0.5))), // near
            Collider::Circle(Circle::new(Vec2::new(8.0, 0.0), 1.0)),                    // far
            Collider::Capsule(Capsule::new(Vec2::new(5.0, -1.0), Vec2::new(5.0, 1.0), 0.5)), // mid
        ];
        let (idx, hit) = nearest_ray_hit(Vec2::ZERO, Vec2::X, &colliders).unwrap();
        assert_eq!(idx, 0, "OBB is closest");
        assert!(approx(hit.distance, 2.5));
    }

    #[test]
    fn nearest_ray_empty_slice() {
        assert!(nearest_ray_hit(Vec2::ZERO, Vec2::X, &[]).is_none());
    }

    #[test]
    fn nearest_ray_zero_direction() {
        let colliders = [Collider::Circle(Circle::new(Vec2::new(5.0, 0.0), 1.0))];
        assert!(nearest_ray_hit(Vec2::ZERO, Vec2::ZERO, &colliders).is_none());
    }

    #[test]
    fn nearest_ray_ignores_non_positive_circle_radius() {
        let colliders = [Collider::Circle(Circle::new(Vec2::new(5.0, 0.0), -1.0))];
        assert!(nearest_ray_hit(Vec2::ZERO, Vec2::X, &colliders).is_none());
    }

    // --- Tagged ---

    #[test]
    fn nearest_ray_tagged_picks_closest() {
        let targets = [
            (
                Collider::Circle(Circle::new(Vec2::new(8.0, 0.0), 1.0)),
                10u32,
            ),
            (Collider::Circle(Circle::new(Vec2::new(3.0, 0.0), 1.0)), 20),
        ];
        let result = nearest_ray_hit_tagged(Vec2::ZERO, Vec2::X, &targets).unwrap();
        assert_eq!(result.id, 20);
    }

    #[test]
    fn nearest_ray_tagged_tie_by_id() {
        let targets = [
            (
                Collider::Circle(Circle::new(Vec2::new(5.0, 0.0), 1.0)),
                30u32,
            ),
            (Collider::Circle(Circle::new(Vec2::new(5.0, 0.0), 1.0)), 10),
        ];
        let result = nearest_ray_hit_tagged(Vec2::ZERO, Vec2::X, &targets).unwrap();
        assert_eq!(result.id, 10, "smaller id wins tie");
    }

    // --- Segment ---

    #[test]
    fn nearest_segment_picks_closest() {
        let colliders = [
            Collider::Circle(Circle::new(Vec2::new(8.0, 0.0), 1.0)),
            Collider::Circle(Circle::new(Vec2::new(3.0, 0.0), 1.0)),
        ];
        let (idx, _) = nearest_segment_hit(Vec2::ZERO, Vec2::new(20.0, 0.0), &colliders).unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn nearest_segment_respects_length() {
        let colliders = [Collider::Circle(Circle::new(Vec2::new(8.0, 0.0), 1.0))];
        // Segment ends before reaching the circle.
        assert!(nearest_segment_hit(Vec2::ZERO, Vec2::new(5.0, 0.0), &colliders).is_none());
    }

    #[test]
    fn results_never_nan() {
        // Exercise a variety of edge-case inputs and verify no NaN in results.
        let origins = [Vec2::ZERO, Vec2::new(0.5, 0.0), Vec2::new(0.0, 0.5)];
        let directions = [Vec2::X, Vec2::Y, Vec2::new(1.0, 1.0)];
        let colliders = [
            Collider::Circle(Circle::new(Vec2::new(3.0, 0.0), 1.0)),
            Collider::Capsule(Capsule::new(Vec2::new(3.0, -1.0), Vec2::new(3.0, 1.0), 0.5)),
            Collider::Obb(Obb::axis_aligned(Vec2::new(3.0, 0.0), Vec2::new(0.5, 0.5))),
            // Degenerate capsule.
            Collider::Capsule(Capsule::new(Vec2::new(3.0, 0.0), Vec2::new(3.0, 0.0), 0.5)),
        ];
        for origin in &origins {
            for dir in &directions {
                if let Some((_, hit)) = nearest_ray_hit(*origin, *dir, &colliders) {
                    assert!(
                        !hit.distance.is_nan(),
                        "distance NaN for {origin:?} {dir:?}"
                    );
                    assert!(
                        !hit.normal.x.is_nan() && !hit.normal.y.is_nan(),
                        "normal NaN for {origin:?} {dir:?}"
                    );
                    assert!(
                        hit.normal.length() > 0.5,
                        "degenerate normal for {origin:?} {dir:?}: {:?}",
                        hit.normal
                    );
                }
            }
        }
    }

    #[test]
    fn nearest_surface_beats_nearest_center() {
        // A large circle far away has its center farther, but its near surface
        // is closer than a small circle's surface.
        let colliders = [
            Collider::Circle(Circle::new(Vec2::new(5.0, 0.0), 0.5)), // surface at 4.5
            Collider::Circle(Circle::new(Vec2::new(8.0, 0.0), 5.0)), // surface at 3.0
        ];
        let (idx, hit) = nearest_ray_hit(Vec2::ZERO, Vec2::X, &colliders).unwrap();
        assert_eq!(idx, 1, "large circle's surface is nearer");
        assert!(approx(hit.distance, 3.0));
    }
}
