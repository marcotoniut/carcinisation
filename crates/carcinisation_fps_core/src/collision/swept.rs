//! Swept-circle vs primitive queries.
//!
//! A swept circle is a circle of fixed radius whose center moves along a
//! segment. Collision is equivalent to inflating the static shape by the
//! sweep radius and testing the center segment against the inflated shape.
//!
//! Returned `point` is the contact point on the **target surface**,
//! `distance` is how far the sweep center traveled before contact,
//! and `normal` is the outward normal of the target at the contact point.

use bevy_math::Vec2;

use super::primitives::{Capsule, Circle, HitDetail, Obb};
use super::segment;
use super::{normalize_or, sign_or};

// ---------------------------------------------------------------------------
// Swept circle vs Circle
// ---------------------------------------------------------------------------

/// Swept circle vs static circle.
///
/// A disk of `sweep_radius` with center moving from `start` to `end` tests
/// against a static circle. Returns the contact point on the **target**
/// circle surface.
#[must_use]
pub fn swept_circle_vs_circle(
    start: Vec2,
    end: Vec2,
    sweep_radius: f32,
    target: &Circle,
) -> Option<HitDetail> {
    if sweep_radius < 0.0 || target.radius <= 0.0 {
        return None;
    }

    let inflated = Circle::new(target.center, target.radius + sweep_radius);
    let hit = segment::segment_vs_circle(start, end, &inflated)?;

    if hit.distance <= 0.0 {
        // Already overlapping at start.
        let dir = normalize_or(start - target.center, Vec2::Y);
        return Some(HitDetail {
            point: target.center + dir * target.radius,
            distance: 0.0,
            normal: dir,
        });
    }

    // For circles, `segment_vs_circle` returns the intersection of the sweep
    // center path with the inflated circle boundary — i.e. `hit.point` IS the
    // sweep center position at contact. We map back to the target surface by
    // projecting from the sweep center toward the target center.
    let center_at_contact = hit.point;
    let to_target = target.center - center_at_contact;
    let dist = to_target.length();

    if dist > f32::EPSILON {
        let dir = to_target / dist;
        Some(HitDetail {
            point: target.center - dir * target.radius,
            distance: hit.distance,
            normal: -dir,
        })
    } else {
        Some(hit)
    }
}

// ---------------------------------------------------------------------------
// Swept circle vs Capsule
// ---------------------------------------------------------------------------

/// Swept circle vs static capsule.
///
/// Inflates the capsule radius by `sweep_radius` and tests the sweep
/// center segment against the inflated capsule.
#[must_use]
pub fn swept_circle_vs_capsule(
    start: Vec2,
    end: Vec2,
    sweep_radius: f32,
    target: &Capsule,
) -> Option<HitDetail> {
    if sweep_radius < 0.0 || target.radius <= 0.0 {
        return None;
    }

    let inflated = Capsule::new(target.a, target.b, target.radius + sweep_radius);
    let hit = segment::segment_vs_capsule(start, end, &inflated)?;

    if hit.distance <= 0.0 {
        // Already overlapping. Find nearest point on capsule segment.
        let ab = target.b - target.a;
        let ab_len_sq = ab.length_squared();
        let nearest = if ab_len_sq < f32::EPSILON * f32::EPSILON {
            target.a
        } else {
            let t = (start - target.a).dot(ab) / ab_len_sq;
            target.a + ab * t.clamp(0.0, 1.0)
        };
        let dir = normalize_or(start - nearest, Vec2::Y);
        return Some(HitDetail {
            point: nearest + dir * target.radius,
            distance: 0.0,
            normal: dir,
        });
    }

    // For capsules and OBBs, the inflated shape preserves the outward normal.
    // Subtracting `normal * sweep_radius` projects from the inflated surface
    // back to the original target surface.
    let contact_on_target = hit.point - hit.normal * sweep_radius;
    Some(HitDetail {
        point: contact_on_target,
        distance: hit.distance,
        normal: hit.normal,
    })
}

// ---------------------------------------------------------------------------
// Swept circle vs OBB
// ---------------------------------------------------------------------------

/// Swept circle vs static OBB.
///
/// The Minkowski sum of the OBB and a disk is a rounded rectangle. This
/// function tests the sweep center segment against that rounded rectangle
/// in OBB-local space, then maps the result back to world coordinates.
#[must_use]
pub fn swept_circle_vs_obb(
    start: Vec2,
    end: Vec2,
    sweep_radius: f32,
    target: &Obb,
) -> Option<HitDetail> {
    if sweep_radius < 0.0 || target.half_extents.x <= 0.0 || target.half_extents.y <= 0.0 {
        return None;
    }

    let (sin, cos) = target.rotation.sin_cos();
    let to_local = |v: Vec2| {
        let d = v - target.center;
        Vec2::new(d.x * cos + d.y * sin, d.y * cos - d.x * sin)
    };
    let to_world_dir = |v: Vec2| Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos);
    let to_world_pt = |v: Vec2| target.center + to_world_dir(v);

    let local_start = to_local(start);
    let local_end = to_local(end);

    let hit =
        segment_vs_rounded_rect_local(local_start, local_end, target.half_extents, sweep_radius)?;

    // Map contact point to OBB surface (subtract outward normal * radius).
    let local_obb_point = hit.point - hit.normal * sweep_radius;

    Some(HitDetail {
        point: to_world_pt(local_obb_point),
        distance: hit.distance,
        normal: to_world_dir(hit.normal),
    })
}

// ---------------------------------------------------------------------------
// Rounded rectangle (local-space helper)
// ---------------------------------------------------------------------------

/// Segment vs rounded rectangle in local space (center at origin).
///
/// A rounded rectangle is the Minkowski sum of AABB `[-hx,hx]×[-hy,hy]`
/// and a disk of `corner_radius`. Its boundary consists of:
/// - Four straight edges (offset outward by `corner_radius` from the AABB faces)
/// - Four quarter-circle arcs at the AABB corners
fn segment_vs_rounded_rect_local(
    start: Vec2,
    end: Vec2,
    half_extents: Vec2,
    corner_radius: f32,
) -> Option<HitDetail> {
    let hx = half_extents.x;
    let hy = half_extents.y;
    let r = corner_radius;

    // Inside check via SDF.
    if inside_rounded_rect(start, half_extents, r) {
        let n = rounded_rect_outward_normal(start, half_extents, r);
        return Some(HitDetail {
            point: start,
            distance: 0.0,
            normal: n,
        });
    }

    let seg = end - start;
    let seg_len_sq = seg.length_squared();
    if seg_len_sq <= f32::EPSILON * f32::EPSILON {
        return None; // Zero-length and outside → miss.
    }
    let seg_len = seg_len_sq.sqrt();
    let dir = seg / seg_len;

    let mut best: Option<HitDetail> = None;
    let mut best_t = seg_len;

    // --- Four straight face edges ---
    // Top: y = hy + r, x ∈ [-hx, hx]
    try_face_y(start, dir, hy + r, hx, Vec2::Y, &mut best, &mut best_t);
    // Bottom: y = -(hy + r)
    try_face_y(
        start,
        dir,
        -(hy + r),
        hx,
        Vec2::NEG_Y,
        &mut best,
        &mut best_t,
    );
    // Right: x = hx + r, y ∈ [-hy, hy]
    try_face_x(start, dir, hx + r, hy, Vec2::X, &mut best, &mut best_t);
    // Left: x = -(hx + r)
    try_face_x(
        start,
        dir,
        -(hx + r),
        hy,
        Vec2::NEG_X,
        &mut best,
        &mut best_t,
    );

    // --- Four corner quarter-circles ---
    for corner in [
        Vec2::new(hx, hy),
        Vec2::new(-hx, hy),
        Vec2::new(hx, -hy),
        Vec2::new(-hx, -hy),
    ] {
        let circle = Circle::new(corner, r);
        if let Some(hit) = segment::segment_vs_circle(start, end, &circle) {
            // Accept only if hit is in the outward quadrant (|x| >= hx AND |y| >= hy).
            if hit.point.x.abs() >= hx - f32::EPSILON
                && hit.point.y.abs() >= hy - f32::EPSILON
                && hit.distance < best_t
            {
                best_t = hit.distance;
                best = Some(hit);
            }
        }
    }

    best
}

fn try_face_y(
    start: Vec2,
    dir: Vec2,
    y: f32,
    half_x: f32,
    normal: Vec2,
    best: &mut Option<HitDetail>,
    best_t: &mut f32,
) {
    if dir.y.abs() < f32::EPSILON {
        return;
    }
    let t = (y - start.y) / dir.y;
    if t >= 0.0 && t < *best_t {
        let hit_x = start.x + dir.x * t;
        if hit_x.abs() <= half_x + f32::EPSILON {
            *best_t = t;
            *best = Some(HitDetail {
                point: Vec2::new(hit_x, y),
                distance: t,
                normal,
            });
        }
    }
}

fn try_face_x(
    start: Vec2,
    dir: Vec2,
    x: f32,
    half_y: f32,
    normal: Vec2,
    best: &mut Option<HitDetail>,
    best_t: &mut f32,
) {
    if dir.x.abs() < f32::EPSILON {
        return;
    }
    let t = (x - start.x) / dir.x;
    if t >= 0.0 && t < *best_t {
        let hit_y = start.y + dir.y * t;
        if hit_y.abs() <= half_y + f32::EPSILON {
            *best_t = t;
            *best = Some(HitDetail {
                point: Vec2::new(x, hit_y),
                distance: t,
                normal,
            });
        }
    }
}

fn inside_rounded_rect(point: Vec2, half_extents: Vec2, corner_radius: f32) -> bool {
    let hx = half_extents.x;
    let hy = half_extents.y;
    let r = corner_radius;

    // Outside bounding AABB.
    if point.x.abs() > hx + r || point.y.abs() > hy + r {
        return false;
    }

    // In the cross region (not in any corner zone).
    if point.x.abs() <= hx || point.y.abs() <= hy {
        return true;
    }

    // Corner zone: check against the corner circle.
    let corner = Vec2::new(hx * point.x.signum(), hy * point.y.signum());
    (point - corner).length_squared() <= r * r
}

fn rounded_rect_outward_normal(point: Vec2, half_extents: Vec2, corner_radius: f32) -> Vec2 {
    let hx = half_extents.x;
    let hy = half_extents.y;

    // Corner zone: normal from corner toward point.
    if point.x.abs() > hx && point.y.abs() > hy {
        let corner = Vec2::new(hx * point.x.signum(), hy * point.y.signum());
        let n = (point - corner).normalize_or_zero();
        if n != Vec2::ZERO {
            return n;
        }
    }

    // Face zone: nearest face normal.
    let dx = (hx + corner_radius) - point.x.abs();
    let dy = (hy + corner_radius) - point.y.abs();
    if dx <= dy {
        Vec2::new(sign_or(point.x, 1.0), 0.0)
    } else {
        Vec2::new(0.0, sign_or(point.y, 1.0))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-3;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    // --- Swept circle vs Circle ---

    #[test]
    fn swept_circle_circle_hit() {
        let target = Circle::new(Vec2::new(5.0, 0.0), 1.0);
        let hit = swept_circle_vs_circle(Vec2::ZERO, Vec2::new(10.0, 0.0), 0.5, &target).unwrap();
        // Contact when sweep center is at 5.0 - 1.0 - 0.5 = 3.5
        assert!(approx(hit.distance, 3.5), "d={}", hit.distance);
        // Contact point on target surface (nearest to sweep center).
        assert!(approx(hit.point.x, 4.0), "px={}", hit.point.x);
        assert!(hit.normal.x < 0.0, "normal faces left");
    }

    #[test]
    fn swept_circle_circle_miss() {
        let target = Circle::new(Vec2::new(5.0, 3.0), 1.0);
        assert!(swept_circle_vs_circle(Vec2::ZERO, Vec2::new(10.0, 0.0), 0.5, &target).is_none());
    }

    #[test]
    fn swept_circle_circle_already_overlapping() {
        let target = Circle::new(Vec2::new(1.0, 0.0), 2.0);
        let hit = swept_circle_vs_circle(Vec2::ZERO, Vec2::new(5.0, 0.0), 1.0, &target).unwrap();
        assert_eq!(hit.distance, 0.0);
    }

    #[test]
    fn swept_circle_circle_segment_too_short() {
        let target = Circle::new(Vec2::new(10.0, 0.0), 1.0);
        assert!(swept_circle_vs_circle(Vec2::ZERO, Vec2::new(5.0, 0.0), 0.5, &target).is_none());
    }

    #[test]
    fn swept_circle_circle_zero_sweep_radius() {
        // Degenerates to segment vs circle.
        let target = Circle::new(Vec2::new(5.0, 0.0), 1.0);
        let hit = swept_circle_vs_circle(Vec2::ZERO, Vec2::new(10.0, 0.0), 0.0, &target).unwrap();
        assert!(approx(hit.distance, 4.0));
    }

    // --- Swept circle vs Capsule ---

    #[test]
    fn swept_circle_capsule_shaft_hit() {
        let cap = Capsule::new(Vec2::new(5.0, -2.0), Vec2::new(5.0, 2.0), 1.0);
        let hit = swept_circle_vs_capsule(Vec2::ZERO, Vec2::new(10.0, 0.0), 0.5, &cap).unwrap();
        // Inflated radius = 1.5. Shaft at x=5, contact at x = 5 - 1.5 = 3.5 for center.
        assert!(approx(hit.distance, 3.5), "d={}", hit.distance);
    }

    #[test]
    fn swept_circle_capsule_miss() {
        let cap = Capsule::new(Vec2::new(5.0, 5.0), Vec2::new(5.0, 8.0), 0.5);
        assert!(swept_circle_vs_capsule(Vec2::ZERO, Vec2::new(10.0, 0.0), 0.3, &cap).is_none());
    }

    #[test]
    fn swept_circle_capsule_overlap_at_start() {
        let cap = Capsule::new(Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), 2.0);
        let hit = swept_circle_vs_capsule(Vec2::ZERO, Vec2::new(5.0, 0.0), 0.5, &cap).unwrap();
        assert_eq!(hit.distance, 0.0);
    }

    // --- Swept circle vs OBB ---

    #[test]
    fn swept_circle_obb_face_hit() {
        let obb = Obb::axis_aligned(Vec2::new(5.0, 0.0), Vec2::new(1.0, 1.0));
        let hit = swept_circle_vs_obb(Vec2::ZERO, Vec2::new(10.0, 0.0), 0.5, &obb).unwrap();
        // OBB left face at x=4. Rounded rect left face at x = 4 - 0.5 = 3.5 for center.
        assert!(approx(hit.distance, 3.5), "d={}", hit.distance);
        // Contact point on OBB surface.
        assert!(approx(hit.point.x, 4.0), "px={}", hit.point.x);
    }

    #[test]
    fn swept_circle_obb_miss() {
        let obb = Obb::axis_aligned(Vec2::new(5.0, 5.0), Vec2::new(1.0, 1.0));
        assert!(swept_circle_vs_obb(Vec2::ZERO, Vec2::new(10.0, 0.0), 0.3, &obb).is_none());
    }

    #[test]
    fn swept_circle_obb_corner_hit() {
        // Sweep toward a corner. OBB at (5, 1.5), half (1,1). Corner at (4, 0.5).
        // Sweep from (0, 0) along X: y=0, corner circle center (4, 0.5), radius 0.5.
        // Should hit the corner circle.
        let obb = Obb::axis_aligned(Vec2::new(5.0, 1.5), Vec2::new(1.0, 1.0));
        let hit = swept_circle_vs_obb(Vec2::ZERO, Vec2::new(10.0, 0.0), 0.5, &obb);
        assert!(hit.is_some(), "should hit corner region");
    }

    #[test]
    fn swept_circle_obb_overlap_at_start() {
        let obb = Obb::axis_aligned(Vec2::ZERO, Vec2::new(2.0, 2.0));
        let hit = swept_circle_vs_obb(Vec2::new(0.5, 0.0), Vec2::new(5.0, 0.0), 0.5, &obb).unwrap();
        assert_eq!(hit.distance, 0.0);
    }

    #[test]
    fn swept_circle_obb_overlap_at_center_has_unit_normal() {
        let obb = Obb::axis_aligned(Vec2::ZERO, Vec2::new(2.0, 2.0));
        let hit = swept_circle_vs_obb(Vec2::ZERO, Vec2::new(5.0, 0.0), 0.5, &obb).unwrap();
        assert_eq!(hit.distance, 0.0);
        assert!(approx(hit.normal.length(), 1.0), "normal={:?}", hit.normal);
    }

    #[test]
    fn swept_circle_obb_rotated() {
        use std::f32::consts::FRAC_PI_4;
        let obb = Obb::new(Vec2::new(5.0, 0.0), Vec2::new(1.0, 1.0), FRAC_PI_4);
        let hit = swept_circle_vs_obb(Vec2::ZERO, Vec2::new(10.0, 0.0), 0.3, &obb);
        assert!(hit.is_some());
        assert!(hit.unwrap().distance > 0.0);
    }

    #[test]
    fn swept_circle_negative_sweep_radius() {
        let target = Circle::new(Vec2::new(5.0, 0.0), 1.0);
        assert!(swept_circle_vs_circle(Vec2::ZERO, Vec2::new(10.0, 0.0), -1.0, &target).is_none());
    }

    // --- Rounded rect internals ---

    #[test]
    fn inside_rounded_rect_center() {
        assert!(inside_rounded_rect(Vec2::ZERO, Vec2::new(1.0, 1.0), 0.5));
    }

    #[test]
    fn inside_rounded_rect_face_region() {
        assert!(inside_rounded_rect(
            Vec2::new(1.3, 0.0),
            Vec2::new(1.0, 1.0),
            0.5
        ));
    }

    #[test]
    fn inside_rounded_rect_corner_inside() {
        // Corner at (1,1), radius 0.5. Point at (1.2, 1.2) → dist = 0.283 < 0.5 ✓
        assert!(inside_rounded_rect(
            Vec2::new(1.2, 1.2),
            Vec2::new(1.0, 1.0),
            0.5
        ));
    }

    #[test]
    fn inside_rounded_rect_corner_outside() {
        // Corner at (1,1), radius 0.5. Point at (1.4, 1.4) → dist = 0.566 > 0.5 ✗
        assert!(!inside_rounded_rect(
            Vec2::new(1.4, 1.4),
            Vec2::new(1.0, 1.0),
            0.5
        ));
    }

    #[test]
    fn inside_rounded_rect_outside_bbox() {
        assert!(!inside_rounded_rect(
            Vec2::new(3.0, 0.0),
            Vec2::new(1.0, 1.0),
            0.5
        ));
    }
}
