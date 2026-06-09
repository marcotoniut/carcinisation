//! Collision primitive types and result structures.

use bevy_math::Vec2;

/// 2D circle collider.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
}

impl Circle {
    #[must_use]
    pub const fn new(center: Vec2, radius: f32) -> Self {
        Self { center, radius }
    }
}

/// 2D capsule collider: a line segment with uniform radius.
///
/// The capsule surface is the set of points within `radius` of segment `a`–`b`.
/// When `a == b`, degenerates to a circle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capsule {
    pub a: Vec2,
    pub b: Vec2,
    pub radius: f32,
}

impl Capsule {
    #[must_use]
    pub const fn new(a: Vec2, b: Vec2, radius: f32) -> Self {
        Self { a, b, radius }
    }
}

/// 2D oriented bounding box.
///
/// `half_extents.x` is the half-width along the local X axis,
/// `half_extents.y` is the half-height along the local Y axis.
/// `rotation` is the angle in radians from world X to local X.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obb {
    pub center: Vec2,
    pub half_extents: Vec2,
    pub rotation: f32,
}

impl Obb {
    #[must_use]
    pub const fn new(center: Vec2, half_extents: Vec2, rotation: f32) -> Self {
        Self {
            center,
            half_extents,
            rotation,
        }
    }

    /// Axis-aligned OBB (rotation = 0).
    #[must_use]
    pub const fn axis_aligned(center: Vec2, half_extents: Vec2) -> Self {
        Self::new(center, half_extents, 0.0)
    }
}

/// Union of 2D collision primitives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Collider {
    Circle(Circle),
    Capsule(Capsule),
    Obb(Obb),
}

/// Detail of a collision hit on a single primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitDetail {
    /// Hit point on the collider surface.
    pub point: Vec2,
    /// Distance from query origin to hit point.
    pub distance: f32,
    /// Outward surface normal at the hit point (unit length).
    pub normal: Vec2,
}

impl HitDetail {
    /// Attach a caller-supplied identifier to produce a [`HitResult`].
    #[must_use]
    pub fn with_id<Id: Copy>(self, id: Id) -> HitResult<Id> {
        HitResult {
            point: self.point,
            distance: self.distance,
            normal: self.normal,
            id,
        }
    }
}

/// Result of a nearest-hit query with caller-supplied identifier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitResult<Id: Copy> {
    /// Hit point on the collider surface.
    pub point: Vec2,
    /// Distance from query origin to hit point.
    pub distance: f32,
    /// Outward surface normal at the hit point (unit length).
    pub normal: Vec2,
    /// Caller-supplied identifier for the hit primitive.
    pub id: Id,
}
