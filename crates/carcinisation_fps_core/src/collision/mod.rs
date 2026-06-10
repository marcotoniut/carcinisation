//! 2D collision primitives and queries for FPS target hit-detection.
//!
//! Provides shared, server-authoritative collision math used by hitscan,
//! flamethrower area checks, projectile intersection, and future per-part
//! damage routing. All queries are deterministic and 2D (XZ map plane).
//!
//! # Modules
//!
//! - [`primitives`] — `Circle`, `Capsule`, `Obb`, `Collider` enum, result types
//! - [`ray`] — ray vs single primitive
//! - [`segment`] — finite segment vs single primitive
//! - [`swept`] — swept circle (moving disk) vs static primitive
//! - [`nearest`] — nearest hit across a collection of colliders
//! - [`target`] — per-part, per-facing collision metadata
//! - [`wall`] — grid-based wall sliding (existing movement collision)

pub mod nearest;
pub mod primitives;
pub mod ray;
pub mod segment;
pub mod swept;
pub mod target;
mod wall;

// --- Wall collision (backwards-compatible re-export) ---
pub use wall::try_move;

// --- Primitive types ---
pub use primitives::{Capsule, Circle, Collider, HitDetail, HitResult, Obb};

// --- Ray queries ---
pub use ray::{ray_vs_capsule, ray_vs_circle, ray_vs_obb};

// --- Segment queries ---
pub use segment::{segment_vs_capsule, segment_vs_circle, segment_vs_obb};

// --- Swept circle queries ---
pub use swept::{swept_circle_vs_capsule, swept_circle_vs_circle, swept_circle_vs_obb};

// --- Nearest-hit queries ---
pub use nearest::{
    nearest_ray_hit, nearest_ray_hit_tagged, nearest_segment_hit, nearest_segment_hit_tagged,
    nearest_swept_circle_hit_tagged,
};

// --- Target collision metadata ---
pub use target::{
    AnimationKey, BillboardFacing8, CollisionFrameKey, MaterialId, PartCollider2d, PartId,
    PartMetadata, TargetCollisionFrame, TargetCollisionSet, TargetQueryPose2d,
};

// --- Convenience dispatch ---

use bevy_math::Vec2;

/// Normalize with fallback: returns the normalized vector, or `fallback` when
/// the input is too short.
fn normalize_or(v: Vec2, fallback: Vec2) -> Vec2 {
    let n = v.normalize_or_zero();
    if n == Vec2::ZERO { fallback } else { n }
}

/// Return a deterministic non-zero sign for axis-aligned normals.
fn sign_or(value: f32, fallback: f32) -> f32 {
    let source = if value == 0.0 { fallback } else { value };
    if source < 0.0 { -1.0 } else { 1.0 }
}

/// Test a ray against any [`Collider`] variant.
#[must_use]
pub fn ray_vs_collider(origin: Vec2, direction: Vec2, collider: &Collider) -> Option<HitDetail> {
    match collider {
        Collider::Circle(c) => ray::ray_vs_circle(origin, direction, c),
        Collider::Capsule(c) => ray::ray_vs_capsule(origin, direction, c),
        Collider::Obb(o) => ray::ray_vs_obb(origin, direction, o),
    }
}

/// Test a segment against any [`Collider`] variant.
#[must_use]
pub fn segment_vs_collider(start: Vec2, end: Vec2, collider: &Collider) -> Option<HitDetail> {
    match collider {
        Collider::Circle(c) => segment::segment_vs_circle(start, end, c),
        Collider::Capsule(c) => segment::segment_vs_capsule(start, end, c),
        Collider::Obb(o) => segment::segment_vs_obb(start, end, o),
    }
}

/// Test a swept circle against any [`Collider`] variant.
#[must_use]
pub fn swept_circle_vs_collider(
    start: Vec2,
    end: Vec2,
    sweep_radius: f32,
    collider: &Collider,
) -> Option<HitDetail> {
    match collider {
        Collider::Circle(c) => swept::swept_circle_vs_circle(start, end, sweep_radius, c),
        Collider::Capsule(c) => swept::swept_circle_vs_capsule(start, end, sweep_radius, c),
        Collider::Obb(o) => swept::swept_circle_vs_obb(start, end, sweep_radius, o),
    }
}
