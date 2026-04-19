//! Generic debug drawing primitives for resolved collider geometry.
//!
//! This module intentionally only draws already-resolved shapes. It does not
//! decide which entities own collision, how collision is resolved, or what
//! gameplay policy should be applied.

use bevy_color::Color;
use bevy_gizmos::prelude::Gizmos;
use bevy_math::Vec2;

/// Draw a resolved world-mask outline.
///
/// `segments` should already be expressed as world-space line segments. The
/// `map_point` callback converts world-space points into the destination space
/// used by the caller's gizmo layer.
pub fn draw_world_mask_outline_2d<I, F>(
    gizmos: &mut Gizmos<'_, '_>,
    segments: I,
    color: Color,
    map_point: F,
) where
    I: IntoIterator<Item = (Vec2, Vec2)>,
    F: Fn(Vec2) -> Vec2,
{
    for (a, b) in segments {
        gizmos.line_2d(map_point(a), map_point(b), color);
    }
}

/// Draw a resolved circle collider.
///
/// `center` and `radius` should already represent the collider in world space.
/// The mapping callbacks adapt those values into the caller's gizmo space.
pub fn draw_circle_collider_2d<FPoint, FRadius>(
    gizmos: &mut Gizmos<'_, '_>,
    center: Vec2,
    radius: f32,
    color: Color,
    map_point: FPoint,
    map_radius: FRadius,
) where
    FPoint: Fn(Vec2) -> Vec2,
    FRadius: Fn(f32) -> f32,
{
    gizmos.circle_2d(map_point(center), map_radius(radius), color);
}

/// Draw a resolved axis-aligned rectangle collider.
///
/// `center` and `size` should already represent the collider in world space.
/// The mapping callbacks adapt those values into the caller's gizmo space.
pub fn draw_rect_collider_2d<FPoint, FSize>(
    gizmos: &mut Gizmos<'_, '_>,
    center: Vec2,
    size: Vec2,
    color: Color,
    map_point: FPoint,
    map_size: FSize,
) where
    FPoint: Fn(Vec2) -> Vec2,
    FSize: Fn(Vec2) -> Vec2,
{
    gizmos.rect_2d(map_point(center), map_size(size), color);
}
