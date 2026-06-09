//! Ray vs primitive intersection queries.
//!
//! All functions accept a non-normalized direction and normalize internally.
//! Zero direction returns `None`.

use bevy_math::Vec2;

use super::primitives::{Capsule, Circle, HitDetail, Obb};
use super::{normalize_or, sign_or};

// ---------------------------------------------------------------------------
// Ray vs Circle
// ---------------------------------------------------------------------------

/// Ray vs circle intersection.
///
/// Returns the nearest hit where the ray enters the circle surface.
/// If the ray origin is inside the circle, returns distance 0 with the
/// outward normal from center toward origin.
#[must_use]
pub fn ray_vs_circle(origin: Vec2, direction: Vec2, circle: &Circle) -> Option<HitDetail> {
    let direction = direction.normalize_or_zero();
    if direction == Vec2::ZERO || circle.radius <= 0.0 {
        return None;
    }
    ray_vs_circle_impl(origin, direction, circle.center, circle.radius)
}

/// Internal: direction must already be unit-length. Radius must be positive.
pub(super) fn ray_vs_circle_impl(
    origin: Vec2,
    direction: Vec2,
    center: Vec2,
    radius: f32,
) -> Option<HitDetail> {
    if radius <= 0.0 {
        return None;
    }

    let f = origin - center;
    let b = f.dot(direction);
    let c = f.length_squared() - radius * radius;

    // Origin inside circle.
    if c <= 0.0 {
        return Some(HitDetail {
            point: origin,
            distance: 0.0,
            normal: normalize_or(f, -direction),
        });
    }

    let discriminant = b.mul_add(b, -c);
    if discriminant < 0.0 {
        return None;
    }

    let t = -b - discriminant.sqrt();
    if t < 0.0 {
        return None;
    }

    let point = origin + direction * t;
    Some(HitDetail {
        point,
        distance: t,
        normal: normalize_or(point - center, -direction),
    })
}

// ---------------------------------------------------------------------------
// Ray vs Capsule
// ---------------------------------------------------------------------------

/// Ray vs capsule intersection.
///
/// The capsule is the Minkowski sum of segment `a`–`b` and a disk of `radius`.
/// If the ray origin is inside, returns distance 0.
/// Degenerate capsule (`a == b`) falls back to circle intersection.
#[must_use]
pub fn ray_vs_capsule(origin: Vec2, direction: Vec2, capsule: &Capsule) -> Option<HitDetail> {
    let direction = direction.normalize_or_zero();
    if direction == Vec2::ZERO || capsule.radius <= 0.0 {
        return None;
    }
    ray_vs_capsule_impl(origin, direction, capsule)
}

/// Internal: direction must already be unit-length. Radius must be positive.
pub(super) fn ray_vs_capsule_impl(
    origin: Vec2,
    direction: Vec2,
    capsule: &Capsule,
) -> Option<HitDetail> {
    if capsule.radius <= 0.0 {
        return None;
    }

    let ab = capsule.b - capsule.a;
    let ab_len_sq = ab.length_squared();

    // Degenerate: treat as circle.
    if ab_len_sq < f32::EPSILON * f32::EPSILON {
        return ray_vs_circle_impl(origin, direction, capsule.a, capsule.radius);
    }

    let ab_len = ab_len_sq.sqrt();
    let axis = ab / ab_len;
    let perp = Vec2::new(-axis.y, axis.x);
    let f = origin - capsule.a;

    // --- Inside test ---
    let proj = f.dot(axis).clamp(0.0, ab_len);
    let nearest = capsule.a + axis * proj;
    if (origin - nearest).length_squared() <= capsule.radius * capsule.radius {
        return Some(HitDetail {
            point: origin,
            distance: 0.0,
            normal: normalize_or(origin - nearest, -direction),
        });
    }

    let mut best: Option<HitDetail> = None;

    // --- Shaft walls ---
    let f_perp = f.dot(perp);
    let d_perp = direction.dot(perp);

    if d_perp.abs() > f32::EPSILON {
        for sign in [1.0_f32, -1.0] {
            let t = (sign * capsule.radius - f_perp) / d_perp;
            if t >= 0.0 {
                let hit_point = origin + direction * t;
                let along = (hit_point - capsule.a).dot(axis);
                if (0.0..=ab_len).contains(&along) && best.as_ref().is_none_or(|b| t < b.distance) {
                    best = Some(HitDetail {
                        point: hit_point,
                        distance: t,
                        normal: perp * sign,
                    });
                }
            }
        }
    }

    // --- End-cap at A (hemisphere facing away from B) ---
    if let Some(hit) = ray_vs_circle_impl(origin, direction, capsule.a, capsule.radius)
        && hit.distance > 0.0
        && (hit.point - capsule.a).dot(axis) <= 0.0
        && best.as_ref().is_none_or(|b| hit.distance < b.distance)
    {
        best = Some(hit);
    }

    // --- End-cap at B (hemisphere facing away from A) ---
    if let Some(hit) = ray_vs_circle_impl(origin, direction, capsule.b, capsule.radius)
        && hit.distance > 0.0
        && (hit.point - capsule.b).dot(axis) >= 0.0
        && best.as_ref().is_none_or(|b| hit.distance < b.distance)
    {
        best = Some(hit);
    }

    best
}

// ---------------------------------------------------------------------------
// Ray vs OBB
// ---------------------------------------------------------------------------

/// Ray vs oriented bounding box intersection.
///
/// Uses slab decomposition in OBB-local space. If the ray origin is inside,
/// returns distance 0 with the nearest-face outward normal.
#[must_use]
pub fn ray_vs_obb(origin: Vec2, direction: Vec2, obb: &Obb) -> Option<HitDetail> {
    let direction = direction.normalize_or_zero();
    if direction == Vec2::ZERO || obb.half_extents.x <= 0.0 || obb.half_extents.y <= 0.0 {
        return None;
    }
    ray_vs_obb_impl(origin, direction, obb)
}

/// Internal: direction must already be unit-length. Half-extents must be positive.
pub(super) fn ray_vs_obb_impl(origin: Vec2, direction: Vec2, obb: &Obb) -> Option<HitDetail> {
    if obb.half_extents.x <= 0.0 || obb.half_extents.y <= 0.0 {
        return None;
    }

    let (sin, cos) = obb.rotation.sin_cos();

    let to_local = |v: Vec2| Vec2::new(v.x * cos + v.y * sin, v.y * cos - v.x * sin);
    let to_world = |v: Vec2| Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos);

    let local_o = to_local(origin - obb.center);
    let local_d = to_local(direction);

    ray_vs_aabb_local(local_o, local_d, obb.half_extents).map(|hit| HitDetail {
        point: obb.center + to_world(hit.point),
        distance: hit.distance,
        normal: to_world(hit.normal),
    })
}

/// Ray vs axis-aligned box in local space (center at origin).
fn ray_vs_aabb_local(origin: Vec2, direction: Vec2, half: Vec2) -> Option<HitDetail> {
    let hx = half.x;
    let hy = half.y;

    // Inside check.
    if origin.x.abs() <= hx && origin.y.abs() <= hy {
        let dx = hx - origin.x.abs();
        let dy = hy - origin.y.abs();
        let local_normal = if dx <= dy {
            Vec2::new(sign_or(origin.x, -direction.x), 0.0)
        } else {
            Vec2::new(0.0, sign_or(origin.y, -direction.y))
        };
        return Some(HitDetail {
            point: origin,
            distance: 0.0,
            normal: local_normal,
        });
    }

    let mut t_near = f32::NEG_INFINITY;
    let mut t_far = f32::INFINITY;
    let mut near_normal = Vec2::ZERO;

    // X slab.
    if direction.x.abs() < f32::EPSILON {
        if origin.x.abs() > hx {
            return None;
        }
    } else {
        let t1 = (-hx - origin.x) / direction.x;
        let t2 = (hx - origin.x) / direction.x;
        let (t_min, t_max) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
        if t_min > t_near {
            t_near = t_min;
            near_normal = Vec2::new(-direction.x.signum(), 0.0);
        }
        t_far = t_far.min(t_max);
        if t_near > t_far || t_far < 0.0 {
            return None;
        }
    }

    // Y slab.
    if direction.y.abs() < f32::EPSILON {
        if origin.y.abs() > hy {
            return None;
        }
    } else {
        let t1 = (-hy - origin.y) / direction.y;
        let t2 = (hy - origin.y) / direction.y;
        let (t_min, t_max) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
        if t_min > t_near {
            t_near = t_min;
            near_normal = Vec2::new(0.0, -direction.y.signum());
        }
        t_far = t_far.min(t_max);
        if t_near > t_far || t_far < 0.0 {
            return None;
        }
    }

    if t_near < 0.0 {
        return None;
    }

    let point = origin + direction * t_near;
    Some(HitDetail {
        point,
        distance: t_near,
        normal: near_normal,
    })
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
    fn circle_direct_hit() {
        let c = Circle::new(Vec2::new(5.0, 0.0), 1.0);
        let hit = ray_vs_circle(Vec2::ZERO, Vec2::X, &c).unwrap();
        assert!(approx(hit.distance, 4.0), "d={}", hit.distance);
        assert!(approx(hit.point.x, 4.0));
        assert!(approx(hit.normal.x, -1.0));
    }

    #[test]
    fn circle_miss() {
        let c = Circle::new(Vec2::new(5.0, 3.0), 1.0);
        assert!(ray_vs_circle(Vec2::ZERO, Vec2::X, &c).is_none());
    }

    #[test]
    fn circle_behind_ray() {
        let c = Circle::new(Vec2::new(-3.0, 0.0), 1.0);
        assert!(ray_vs_circle(Vec2::ZERO, Vec2::X, &c).is_none());
    }

    #[test]
    fn circle_origin_inside() {
        let c = Circle::new(Vec2::ZERO, 2.0);
        let hit = ray_vs_circle(Vec2::new(0.5, 0.0), Vec2::X, &c).unwrap();
        assert_eq!(hit.distance, 0.0);
        assert!(hit.normal.x > 0.0, "outward from center");
    }

    #[test]
    fn circle_tangent_hit() {
        let c = Circle::new(Vec2::new(5.0, 1.0), 1.0);
        let hit = ray_vs_circle(Vec2::ZERO, Vec2::X, &c);
        // Tangent: perpendicular distance == radius. Discriminant ≈ 0.
        assert!(hit.is_some());
        let h = hit.unwrap();
        assert!(approx(h.point.y, 0.0));
    }

    #[test]
    fn circle_zero_direction() {
        let c = Circle::new(Vec2::new(5.0, 0.0), 1.0);
        assert!(ray_vs_circle(Vec2::ZERO, Vec2::ZERO, &c).is_none());
    }

    #[test]
    fn circle_zero_radius() {
        let c = Circle::new(Vec2::new(5.0, 0.0), 0.0);
        assert!(ray_vs_circle(Vec2::ZERO, Vec2::X, &c).is_none());
    }

    #[test]
    fn circle_negative_radius() {
        let c = Circle::new(Vec2::new(5.0, 0.0), -1.0);
        assert!(ray_vs_circle(Vec2::ZERO, Vec2::X, &c).is_none());
    }

    #[test]
    fn circle_unnormalized_direction() {
        let c = Circle::new(Vec2::new(5.0, 0.0), 1.0);
        let hit = ray_vs_circle(Vec2::ZERO, Vec2::new(10.0, 0.0), &c).unwrap();
        assert!(approx(hit.distance, 4.0));
    }

    // --- Capsule ---

    #[test]
    fn capsule_shaft_hit() {
        let cap = Capsule::new(Vec2::new(3.0, -2.0), Vec2::new(3.0, 2.0), 1.0);
        let hit = ray_vs_capsule(Vec2::ZERO, Vec2::X, &cap).unwrap();
        assert!(approx(hit.distance, 2.0), "d={}", hit.distance);
        assert!(approx(hit.normal.x, -1.0));
    }

    #[test]
    fn capsule_cap_a_hit() {
        // Capsule along Y. Ray aimed at cap A (bottom).
        let cap = Capsule::new(Vec2::new(5.0, 0.0), Vec2::new(5.0, 4.0), 1.0);
        let hit = ray_vs_capsule(Vec2::ZERO, Vec2::new(5.0, -0.5).normalize(), &cap).unwrap();
        assert!(hit.distance > 0.0);
    }

    #[test]
    fn capsule_cap_b_hit() {
        let cap = Capsule::new(Vec2::new(5.0, 0.0), Vec2::new(5.0, 4.0), 1.0);
        let hit = ray_vs_capsule(Vec2::ZERO, Vec2::new(5.0, 4.5).normalize(), &cap).unwrap();
        assert!(hit.distance > 0.0);
    }

    #[test]
    fn capsule_miss() {
        let cap = Capsule::new(Vec2::new(3.0, 3.0), Vec2::new(3.0, 6.0), 0.5);
        assert!(ray_vs_capsule(Vec2::ZERO, Vec2::X, &cap).is_none());
    }

    #[test]
    fn capsule_origin_inside() {
        let cap = Capsule::new(Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), 1.0);
        let hit = ray_vs_capsule(Vec2::ZERO, Vec2::X, &cap).unwrap();
        assert_eq!(hit.distance, 0.0);
    }

    #[test]
    fn capsule_degenerate_to_circle() {
        let cap = Capsule::new(Vec2::new(5.0, 0.0), Vec2::new(5.0, 0.0), 1.0);
        let hit = ray_vs_capsule(Vec2::ZERO, Vec2::X, &cap).unwrap();
        assert!(approx(hit.distance, 4.0));
    }

    #[test]
    fn capsule_zero_radius() {
        let cap = Capsule::new(Vec2::ZERO, Vec2::new(5.0, 0.0), 0.0);
        assert!(ray_vs_capsule(Vec2::ZERO, Vec2::X, &cap).is_none());
    }

    #[test]
    fn capsule_shaft_cap_boundary() {
        // Ray aimed exactly at the junction of shaft and A-cap.
        // Capsule along Y from (5,0) to (5,4), radius 1.
        // A-cap hemisphere center at (5,0). Shaft valid for along ∈ [0, 4].
        // A ray aimed at (5, 0) — the exact boundary — should produce a hit.
        let cap = Capsule::new(Vec2::new(5.0, 0.0), Vec2::new(5.0, 4.0), 1.0);
        let hit = ray_vs_capsule(Vec2::ZERO, Vec2::new(5.0, 0.0).normalize(), &cap);
        assert!(hit.is_some(), "shaft/cap boundary should hit");
    }

    #[test]
    fn capsule_diagonal_shaft() {
        // 45-degree capsule.
        let cap = Capsule::new(Vec2::new(2.0, 2.0), Vec2::new(6.0, 6.0), 0.5);
        let hit = ray_vs_capsule(Vec2::new(0.0, 4.0), Vec2::X, &cap);
        assert!(hit.is_some());
    }

    // --- OBB ---

    #[test]
    fn obb_axis_aligned_direct_hit() {
        let obb = Obb::axis_aligned(Vec2::new(5.0, 0.0), Vec2::new(1.0, 1.0));
        let hit = ray_vs_obb(Vec2::ZERO, Vec2::X, &obb).unwrap();
        assert!(approx(hit.distance, 4.0), "d={}", hit.distance);
        assert!(approx(hit.normal.x, -1.0));
    }

    #[test]
    fn obb_axis_aligned_miss() {
        let obb = Obb::axis_aligned(Vec2::new(5.0, 5.0), Vec2::new(1.0, 1.0));
        assert!(ray_vs_obb(Vec2::ZERO, Vec2::X, &obb).is_none());
    }

    #[test]
    fn obb_origin_inside() {
        let obb = Obb::axis_aligned(Vec2::ZERO, Vec2::new(2.0, 2.0));
        let hit = ray_vs_obb(Vec2::new(0.5, 0.0), Vec2::X, &obb).unwrap();
        assert_eq!(hit.distance, 0.0);
    }

    #[test]
    fn obb_origin_at_center_has_unit_normal() {
        let obb = Obb::axis_aligned(Vec2::ZERO, Vec2::new(2.0, 2.0));
        let hit = ray_vs_obb(Vec2::ZERO, Vec2::X, &obb).unwrap();
        assert_eq!(hit.distance, 0.0);
        assert!(approx(hit.normal.length(), 1.0), "normal={:?}", hit.normal);
    }

    #[test]
    fn obb_rotated_45_hit() {
        use std::f32::consts::FRAC_PI_4;
        // Diamond shape: rotated 45°, half_extents (1,1).
        // Corner at ~(1.414, 0) in world.
        let obb = Obb::new(Vec2::new(5.0, 0.0), Vec2::new(1.0, 1.0), FRAC_PI_4);
        let hit = ray_vs_obb(Vec2::ZERO, Vec2::X, &obb).unwrap();
        let expected_dist = 5.0 - std::f32::consts::SQRT_2;
        assert!(approx(hit.distance, expected_dist), "d={}", hit.distance);
    }

    #[test]
    fn obb_rotated_miss() {
        use std::f32::consts::FRAC_PI_4;
        let obb = Obb::new(Vec2::new(5.0, 3.0), Vec2::new(0.5, 0.5), FRAC_PI_4);
        assert!(ray_vs_obb(Vec2::ZERO, Vec2::X, &obb).is_none());
    }

    #[test]
    fn obb_zero_direction() {
        let obb = Obb::axis_aligned(Vec2::new(5.0, 0.0), Vec2::new(1.0, 1.0));
        assert!(ray_vs_obb(Vec2::ZERO, Vec2::ZERO, &obb).is_none());
    }

    #[test]
    fn obb_zero_half_extents() {
        let obb = Obb::axis_aligned(Vec2::new(5.0, 0.0), Vec2::new(0.0, 1.0));
        assert!(ray_vs_obb(Vec2::ZERO, Vec2::X, &obb).is_none());
    }

    #[test]
    fn obb_hit_from_y_axis() {
        let obb = Obb::axis_aligned(Vec2::new(0.0, 5.0), Vec2::new(1.0, 1.0));
        let hit = ray_vs_obb(Vec2::ZERO, Vec2::Y, &obb).unwrap();
        assert!(approx(hit.distance, 4.0));
        assert!(approx(hit.normal.y, -1.0));
    }

    #[test]
    fn obb_behind_ray() {
        let obb = Obb::axis_aligned(Vec2::new(-5.0, 0.0), Vec2::new(1.0, 1.0));
        assert!(ray_vs_obb(Vec2::ZERO, Vec2::X, &obb).is_none());
    }

    #[test]
    fn obb_edge_graze() {
        // Ray just barely clips the corner of the box.
        let obb = Obb::axis_aligned(Vec2::new(5.0, 0.99), Vec2::new(1.0, 1.0));
        let hit = ray_vs_obb(Vec2::ZERO, Vec2::X, &obb);
        assert!(hit.is_some());
    }
}
