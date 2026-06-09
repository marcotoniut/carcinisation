//! Segment vs primitive intersection queries.
//!
//! A segment is a finite line from `start` to `end`. Zero-length segments
//! degenerate to point containment tests.

use bevy_math::Vec2;

use super::primitives::{Capsule, Circle, HitDetail, Obb};
use super::ray;
use super::sign_or;

// ---------------------------------------------------------------------------
// Segment vs Circle
// ---------------------------------------------------------------------------

/// Segment vs circle intersection.
///
/// Returns the nearest hit where the segment enters the circle surface.
/// If `start` is inside the circle, returns distance 0.
/// Zero-length segment tests point containment.
#[must_use]
pub fn segment_vs_circle(start: Vec2, end: Vec2, circle: &Circle) -> Option<HitDetail> {
    if circle.radius <= 0.0 {
        return None;
    }

    let seg = end - start;
    let len_sq = seg.length_squared();

    // Zero-length: point containment.
    if len_sq <= f32::EPSILON * f32::EPSILON {
        return point_in_circle(start, circle);
    }

    let len = len_sq.sqrt();
    let dir = seg / len;

    // Reuse ray math but cap to segment length.
    let f = start - circle.center;
    let b = f.dot(dir);
    let c = f.length_squared() - circle.radius * circle.radius;

    // Start inside.
    if c <= 0.0 {
        let n = f.normalize_or_zero();
        return Some(HitDetail {
            point: start,
            distance: 0.0,
            normal: if n == Vec2::ZERO { -dir } else { n },
        });
    }

    let discriminant = b.mul_add(b, -c);
    if discriminant < 0.0 {
        return None;
    }

    let t = -b - discriminant.sqrt();
    if t < 0.0 || t > len {
        return None;
    }

    let point = start + dir * t;
    let n = (point - circle.center).normalize_or_zero();
    Some(HitDetail {
        point,
        distance: t,
        normal: if n == Vec2::ZERO { -dir } else { n },
    })
}

fn point_in_circle(point: Vec2, circle: &Circle) -> Option<HitDetail> {
    let d = point.distance(circle.center);
    if d <= circle.radius {
        let n = (point - circle.center).normalize_or_zero();
        Some(HitDetail {
            point,
            distance: 0.0,
            normal: if n == Vec2::ZERO { Vec2::Y } else { n },
        })
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Segment vs Capsule
// ---------------------------------------------------------------------------

/// Segment vs capsule intersection.
///
/// Returns the nearest entry hit. If `start` is inside the capsule, distance 0.
#[must_use]
pub fn segment_vs_capsule(start: Vec2, end: Vec2, capsule: &Capsule) -> Option<HitDetail> {
    if capsule.radius <= 0.0 {
        return None;
    }

    let seg = end - start;
    let seg_len_sq = seg.length_squared();

    // Zero-length segment.
    if seg_len_sq <= f32::EPSILON * f32::EPSILON {
        return point_in_capsule(start, capsule);
    }

    let seg_len = seg_len_sq.sqrt();

    // Delegate to ray query, then filter by segment length.
    let hit = ray::ray_vs_capsule(start, seg, capsule)?;
    if hit.distance <= seg_len {
        Some(hit)
    } else {
        None
    }
}

fn point_in_capsule(point: Vec2, capsule: &Capsule) -> Option<HitDetail> {
    let ab = capsule.b - capsule.a;
    let ab_len_sq = ab.length_squared();

    let nearest = if ab_len_sq < f32::EPSILON * f32::EPSILON {
        capsule.a
    } else {
        let t = (point - capsule.a).dot(ab) / ab_len_sq;
        capsule.a + ab * t.clamp(0.0, 1.0)
    };

    let dist_sq = (point - nearest).length_squared();
    if dist_sq <= capsule.radius * capsule.radius {
        let n = (point - nearest).normalize_or_zero();
        Some(HitDetail {
            point,
            distance: 0.0,
            normal: if n == Vec2::ZERO { Vec2::Y } else { n },
        })
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Segment vs OBB
// ---------------------------------------------------------------------------

/// Segment vs OBB intersection.
///
/// Returns the nearest entry hit. If `start` is inside the OBB, distance 0.
#[must_use]
pub fn segment_vs_obb(start: Vec2, end: Vec2, obb: &Obb) -> Option<HitDetail> {
    if obb.half_extents.x <= 0.0 || obb.half_extents.y <= 0.0 {
        return None;
    }

    let seg = end - start;
    let seg_len_sq = seg.length_squared();

    if seg_len_sq <= f32::EPSILON * f32::EPSILON {
        return point_in_obb(start, obb);
    }

    let seg_len = seg_len_sq.sqrt();

    let hit = ray::ray_vs_obb(start, seg, obb)?;
    if hit.distance <= seg_len {
        Some(hit)
    } else {
        None
    }
}

fn point_in_obb(point: Vec2, obb: &Obb) -> Option<HitDetail> {
    let (sin, cos) = obb.rotation.sin_cos();
    let d = point - obb.center;
    let local = Vec2::new(d.x * cos + d.y * sin, d.y * cos - d.x * sin);

    if local.x.abs() <= obb.half_extents.x && local.y.abs() <= obb.half_extents.y {
        let dx = obb.half_extents.x - local.x.abs();
        let dy = obb.half_extents.y - local.y.abs();
        let local_n = if dx <= dy {
            Vec2::new(sign_or(local.x, 1.0), 0.0)
        } else {
            Vec2::new(0.0, sign_or(local.y, 1.0))
        };
        let normal = Vec2::new(
            local_n.x * cos - local_n.y * sin,
            local_n.x * sin + local_n.y * cos,
        );
        Some(HitDetail {
            point,
            distance: 0.0,
            normal,
        })
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-4;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    // --- Circle ---

    #[test]
    fn seg_circle_direct_hit() {
        let c = Circle::new(Vec2::new(5.0, 0.0), 1.0);
        let hit = segment_vs_circle(Vec2::ZERO, Vec2::new(10.0, 0.0), &c).unwrap();
        assert!(approx(hit.distance, 4.0));
    }

    #[test]
    fn seg_circle_miss_too_short() {
        let c = Circle::new(Vec2::new(5.0, 0.0), 1.0);
        assert!(segment_vs_circle(Vec2::ZERO, Vec2::new(3.0, 0.0), &c).is_none());
    }

    #[test]
    fn seg_circle_miss_wide() {
        let c = Circle::new(Vec2::new(5.0, 3.0), 1.0);
        assert!(segment_vs_circle(Vec2::ZERO, Vec2::new(10.0, 0.0), &c).is_none());
    }

    #[test]
    fn seg_circle_start_inside() {
        let c = Circle::new(Vec2::ZERO, 2.0);
        let hit = segment_vs_circle(Vec2::new(0.5, 0.0), Vec2::new(5.0, 0.0), &c).unwrap();
        assert_eq!(hit.distance, 0.0);
    }

    #[test]
    fn seg_circle_zero_length() {
        let c = Circle::new(Vec2::ZERO, 2.0);
        let hit = segment_vs_circle(Vec2::new(0.5, 0.0), Vec2::new(0.5, 0.0), &c).unwrap();
        assert_eq!(hit.distance, 0.0);
    }

    #[test]
    fn seg_circle_zero_length_outside() {
        let c = Circle::new(Vec2::ZERO, 1.0);
        assert!(segment_vs_circle(Vec2::new(5.0, 0.0), Vec2::new(5.0, 0.0), &c).is_none());
    }

    #[test]
    fn seg_circle_zero_radius() {
        let c = Circle::new(Vec2::ZERO, 0.0);
        assert!(segment_vs_circle(Vec2::new(-5.0, 0.0), Vec2::new(5.0, 0.0), &c).is_none());
    }

    // --- Capsule ---

    #[test]
    fn seg_capsule_shaft_hit() {
        let cap = Capsule::new(Vec2::new(3.0, -2.0), Vec2::new(3.0, 2.0), 1.0);
        let hit = segment_vs_capsule(Vec2::ZERO, Vec2::new(10.0, 0.0), &cap).unwrap();
        assert!(approx(hit.distance, 2.0));
    }

    #[test]
    fn seg_capsule_miss_too_short() {
        let cap = Capsule::new(Vec2::new(5.0, -2.0), Vec2::new(5.0, 2.0), 1.0);
        assert!(segment_vs_capsule(Vec2::ZERO, Vec2::new(3.0, 0.0), &cap).is_none());
    }

    #[test]
    fn seg_capsule_start_inside() {
        let cap = Capsule::new(Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), 1.0);
        let hit = segment_vs_capsule(Vec2::ZERO, Vec2::new(5.0, 0.0), &cap).unwrap();
        assert_eq!(hit.distance, 0.0);
    }

    // --- OBB ---

    #[test]
    fn seg_obb_direct_hit() {
        let obb = Obb::axis_aligned(Vec2::new(5.0, 0.0), Vec2::new(1.0, 1.0));
        let hit = segment_vs_obb(Vec2::ZERO, Vec2::new(10.0, 0.0), &obb).unwrap();
        assert!(approx(hit.distance, 4.0));
    }

    #[test]
    fn seg_obb_miss_too_short() {
        let obb = Obb::axis_aligned(Vec2::new(5.0, 0.0), Vec2::new(1.0, 1.0));
        assert!(segment_vs_obb(Vec2::ZERO, Vec2::new(3.0, 0.0), &obb).is_none());
    }

    #[test]
    fn seg_obb_start_inside() {
        let obb = Obb::axis_aligned(Vec2::ZERO, Vec2::new(2.0, 2.0));
        let hit = segment_vs_obb(Vec2::new(0.5, 0.0), Vec2::new(5.0, 0.0), &obb).unwrap();
        assert_eq!(hit.distance, 0.0);
    }

    #[test]
    fn seg_obb_zero_length_inside() {
        let obb = Obb::axis_aligned(Vec2::ZERO, Vec2::new(2.0, 2.0));
        let hit = segment_vs_obb(Vec2::new(0.5, 0.0), Vec2::new(0.5, 0.0), &obb).unwrap();
        assert_eq!(hit.distance, 0.0);
    }

    #[test]
    fn seg_obb_zero_length_center_has_unit_normal() {
        let obb = Obb::axis_aligned(Vec2::ZERO, Vec2::new(2.0, 2.0));
        let hit = segment_vs_obb(Vec2::ZERO, Vec2::ZERO, &obb).unwrap();
        assert_eq!(hit.distance, 0.0);
        assert!(approx(hit.normal.length(), 1.0), "normal={:?}", hit.normal);
    }

    #[test]
    fn seg_obb_zero_length_outside() {
        let obb = Obb::axis_aligned(Vec2::ZERO, Vec2::new(1.0, 1.0));
        assert!(segment_vs_obb(Vec2::new(5.0, 0.0), Vec2::new(5.0, 0.0), &obb).is_none());
    }

    #[test]
    fn seg_capsule_zero_length_inside() {
        let cap = Capsule::new(Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), 1.0);
        let hit = segment_vs_capsule(Vec2::ZERO, Vec2::ZERO, &cap).unwrap();
        assert_eq!(hit.distance, 0.0);
    }

    #[test]
    fn seg_capsule_zero_length_outside() {
        let cap = Capsule::new(Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), 0.5);
        assert!(segment_vs_capsule(Vec2::new(0.0, 3.0), Vec2::new(0.0, 3.0), &cap).is_none());
    }

    #[test]
    fn seg_obb_rotated_zero_length_inside() {
        use std::f32::consts::FRAC_PI_4;
        // Rotated 45°, half_extents (1,1) → diamond with corners at ±√2 on axes.
        let obb = Obb::new(Vec2::ZERO, Vec2::new(1.0, 1.0), FRAC_PI_4);
        // Point at (0.5, 0.5): in local frame this is ~(0.707, 0.0), inside the box.
        let hit = segment_vs_obb(Vec2::new(0.5, 0.5), Vec2::new(0.5, 0.5), &obb).unwrap();
        assert_eq!(hit.distance, 0.0);
    }

    #[test]
    fn seg_obb_rotated_zero_length_outside() {
        use std::f32::consts::FRAC_PI_4;
        let obb = Obb::new(Vec2::ZERO, Vec2::new(1.0, 1.0), FRAC_PI_4);
        // Point at (1.0, 1.0): in local frame this is ~(1.414, 0.0), outside.
        assert!(segment_vs_obb(Vec2::new(1.0, 1.0), Vec2::new(1.0, 1.0), &obb).is_none());
    }
}
