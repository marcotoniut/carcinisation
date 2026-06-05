//! Shared 2D combat hit-detection.

use bevy_math::Vec2;

use crate::camera::Camera;
use crate::config::PlayerFlamethrowerConfig;
use crate::map::Map;
use crate::raycast::cast_ray;

/// Shared 2D weapon fire pose.
///
/// `yaw` is gameplay direction. `visual_pitch_px` is carried for presentation
/// only and must not affect damage or wall obstruction.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FirePose2d {
    pub origin_xy: Vec2,
    pub yaw: f32,
    pub visual_pitch_px: f32,
}

impl FirePose2d {
    #[must_use]
    pub const fn new(origin_xy: Vec2, yaw: f32, visual_pitch_px: f32) -> Self {
        Self {
            origin_xy,
            yaw,
            visual_pitch_px,
        }
    }

    #[must_use]
    pub fn direction(self) -> Vec2 {
        Vec2::new(self.yaw.cos(), self.yaw.sin())
    }
}

impl From<&Camera> for FirePose2d {
    fn from(camera: &Camera) -> Self {
        Self::new(camera.position, camera.angle, camera.aim_pitch)
    }
}

/// Distance fire may travel before hitting a wall, capped by `max_distance`.
///
/// Low-level helper for callers that already have an explicit 2D origin and
/// direction. Prefer [`wall_obstruction_distance_for_pose`] when a full weapon
/// fire pose is available.
#[must_use]
pub fn wall_obstruction_distance(
    map: &Map,
    origin: Vec2,
    direction: Vec2,
    max_distance: f32,
) -> f32 {
    if max_distance <= 0.0 {
        return 0.0;
    }
    if map.get(origin.x.floor() as i32, origin.y.floor() as i32) > 0 {
        return 0.0;
    }
    let direction = direction.normalize_or_zero();
    if direction == Vec2::ZERO {
        return 0.0;
    }
    let hit = cast_ray(map, origin, direction);
    if hit.wall_id > 0 {
        hit.distance.min(max_distance)
    } else {
        max_distance
    }
}

/// Distance fire may travel before hitting a wall from a shared fire pose.
#[must_use]
pub fn wall_obstruction_distance_for_pose(map: &Map, pose: FirePose2d, max_distance: f32) -> f32 {
    wall_obstruction_distance(map, pose.origin_xy, pose.direction(), max_distance)
}

/// Presentation helper for flame streams.
///
/// Uses the same solid-start and wall-distance semantics as combat
/// obstruction, so local and remote flame visuals stop at the same geometry.
#[must_use]
pub fn flame_visual_max_distance(
    map: &Map,
    origin: Vec2,
    direction: Vec2,
    max_distance: f32,
) -> f32 {
    wall_obstruction_distance(map, origin, direction, max_distance)
}

/// Check whether a flamethrower at `origin` facing `direction` hits a target
/// at `target_pos`.
///
/// Uses perpendicular distance from the flame line:
/// - `along = dot(to_target, direction)` must be in `(0.01 ..= range)`
/// - `perp = |to_target - direction * along|` must be `<= half_width`
/// - Line-of-sight from `origin` to `target_pos` must be clear
///
/// Both SP and server call this function for flame damage decisions.
#[must_use]
pub fn flame_hits_position(
    origin: Vec2,
    direction: Vec2,
    target_pos: Vec2,
    range: f32,
    half_width: f32,
    map: &Map,
) -> bool {
    let direction = direction.normalize_or_zero();
    if direction == Vec2::ZERO {
        return false;
    }
    let to_target = target_pos - origin;
    let along = to_target.dot(direction);

    // Must be in front and within range.
    if !(0.01..=range).contains(&along) {
        return false;
    }

    // Perpendicular distance from the flame line.
    let perp = to_target - direction * along;
    if perp.length_squared() > half_width * half_width {
        return false;
    }

    // Line-of-sight check.
    let dist = to_target.length();
    if dist <= 0.01 {
        return true;
    }
    wall_obstruction_distance(map, origin, to_target / dist, dist) >= dist
}

/// Check whether a flamethrower from a shared 2D fire pose hits a target.
#[must_use]
pub fn flame_hits_position_from_pose(
    pose: FirePose2d,
    target_pos: Vec2,
    range: f32,
    half_width: f32,
    map: &Map,
) -> bool {
    flame_hits_position(
        pose.origin_xy,
        pose.direction(),
        target_pos,
        range,
        half_width,
        map,
    )
}

/// Compatibility wrapper using raw origin/direction values from a
/// `PlayerFlamethrowerConfig`.
///
/// New weapon-fire call sites should prefer
/// [`flame_hits_position_configured_from_pose`] so origin/yaw/pitch metadata
/// stays centralized in [`FirePose2d`].
#[must_use]
pub fn flame_hits_position_configured(
    origin: Vec2,
    direction: Vec2,
    target_pos: Vec2,
    map: &Map,
    cfg: &PlayerFlamethrowerConfig,
) -> bool {
    flame_hits_position(
        origin,
        direction,
        target_pos,
        cfg.range,
        cfg.hit_half_width,
        map,
    )
}

/// Convenience wrapper using values from a `PlayerFlamethrowerConfig`.
#[must_use]
pub fn flame_hits_position_configured_from_pose(
    pose: FirePose2d,
    target_pos: Vec2,
    map: &Map,
    cfg: &PlayerFlamethrowerConfig,
) -> bool {
    flame_hits_position_from_pose(pose, target_pos, cfg.range, cfg.hit_half_width, map)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::map::test_map;

    #[test]
    fn direct_hit_in_front() {
        let map = test_map();
        let origin = Vec2::new(1.5, 1.5);
        let dir = Vec2::new(1.0, 0.0); // facing east
        let target = Vec2::new(3.5, 1.5); // directly ahead

        assert!(flame_hits_position(origin, dir, target, 5.0, 0.5, &map));
    }

    #[test]
    fn miss_too_far() {
        let map = test_map();
        let origin = Vec2::new(1.5, 1.5);
        let dir = Vec2::new(1.0, 0.0);
        let target = Vec2::new(1.5 + 6.0, 1.5); // beyond range

        assert!(!flame_hits_position(origin, dir, target, 5.0, 0.5, &map));
    }

    #[test]
    fn miss_too_wide() {
        let map = test_map();
        let origin = Vec2::new(1.5, 1.5);
        let dir = Vec2::new(1.0, 0.0);
        // 1.0 units off to the side, half_width is 0.5
        let target = Vec2::new(3.5, 2.5);

        assert!(!flame_hits_position(origin, dir, target, 5.0, 0.5, &map));
    }

    #[test]
    fn hit_within_half_width() {
        let map = test_map();
        let origin = Vec2::new(1.5, 1.5);
        let dir = Vec2::new(1.0, 0.0);
        // 0.3 units off to the side, half_width is 0.5
        let target = Vec2::new(3.5, 1.8);

        assert!(flame_hits_position(origin, dir, target, 5.0, 0.5, &map));
    }

    #[test]
    fn miss_behind_player() {
        let map = test_map();
        let origin = Vec2::new(3.5, 1.5);
        let dir = Vec2::new(1.0, 0.0); // facing east
        let target = Vec2::new(1.5, 1.5); // behind

        assert!(!flame_hits_position(origin, dir, target, 5.0, 0.5, &map));
    }

    #[test]
    fn blocked_by_wall() {
        // Wall at column 4 in test_map (8x8 map, walls on borders).
        let map = crate::map::Map {
            width: 8,
            height: 4,
            cells: vec![
                1, 1, 1, 1, 1, 1, 1, 1, //
                1, 0, 0, 0, 1, 0, 0, 1, // wall at (4,1)
                1, 0, 0, 0, 0, 0, 0, 1, //
                1, 1, 1, 1, 1, 1, 1, 1,
            ],
        };
        let origin = Vec2::new(1.5, 1.5);
        let dir = Vec2::new(1.0, 0.0);
        let target = Vec2::new(5.5, 1.5); // behind wall

        assert!(!flame_hits_position(origin, dir, target, 6.0, 0.5, &map));
    }

    #[test]
    fn obstruction_distance_clamps_to_wall() {
        let map = crate::map::Map {
            width: 5,
            height: 3,
            cells: vec![
                1, 1, 1, 1, 1, //
                1, 0, 1, 0, 1, //
                1, 1, 1, 1, 1,
            ],
        };
        let distance = wall_obstruction_distance(&map, Vec2::new(1.5, 1.5), Vec2::X, 10.0);
        assert!((distance - 0.5).abs() < 0.001, "{distance}");
    }

    #[test]
    fn obstruction_origin_inside_wall_returns_zero() {
        let map = crate::map::Map {
            width: 5,
            height: 3,
            cells: vec![
                1, 1, 1, 1, 1, //
                1, 0, 1, 0, 1, //
                1, 1, 1, 1, 1,
            ],
        };

        let distance = wall_obstruction_distance(&map, Vec2::new(2.5, 1.5), Vec2::X, 10.0);

        assert_eq!(distance, 0.0);
    }

    #[test]
    fn visual_pitch_does_not_affect_flame_obstruction() {
        let map = crate::map::Map {
            width: 6,
            height: 3,
            cells: vec![
                1, 1, 1, 1, 1, 1, //
                1, 0, 0, 1, 0, 1, //
                1, 1, 1, 1, 1, 1,
            ],
        };
        let flat = FirePose2d::new(Vec2::new(1.5, 1.5), 0.0, 0.0);
        let pitched = FirePose2d::new(Vec2::new(1.5, 1.5), 0.0, 48.0);

        let flat_distance = wall_obstruction_distance_for_pose(&map, flat, 10.0);
        let pitched_distance = wall_obstruction_distance_for_pose(&map, pitched, 10.0);
        assert!((flat_distance - pitched_distance).abs() < f32::EPSILON);
        assert_eq!(
            flame_hits_position_configured_from_pose(
                flat,
                Vec2::new(4.5, 1.5),
                &map,
                &PlayerFlamethrowerConfig::load(),
            ),
            flame_hits_position_configured_from_pose(
                pitched,
                Vec2::new(4.5, 1.5),
                &map,
                &PlayerFlamethrowerConfig::load(),
            )
        );
    }

    #[test]
    fn hit_at_edge_of_range() {
        let map = test_map();
        let origin = Vec2::new(1.5, 1.5);
        let dir = Vec2::new(1.0, 0.0);
        // Exactly at range boundary.
        let target = Vec2::new(1.5 + 5.0, 1.5);

        assert!(flame_hits_position(origin, dir, target, 5.0, 0.5, &map));
    }

    #[test]
    fn hit_at_edge_of_half_width() {
        let map = test_map();
        let origin = Vec2::new(1.5, 1.5);
        let dir = Vec2::new(1.0, 0.0);
        // 0.49 off-center, half_width is 0.5 — inside.
        let target = Vec2::new(2.5, 1.5 + 0.49);

        assert!(flame_hits_position(origin, dir, target, 5.0, 0.5, &map));
    }

    #[test]
    fn very_close_target_hits() {
        let map = test_map();
        let origin = Vec2::new(1.5, 1.5);
        let dir = Vec2::new(1.0, 0.0);
        let target = Vec2::new(1.52, 1.5); // very close but in front

        assert!(flame_hits_position(origin, dir, target, 5.0, 0.5, &map));
    }

    #[test]
    fn default_wrapper_uses_config_constants() {
        let map = test_map();
        let origin = Vec2::new(1.5, 1.5);
        let dir = Vec2::new(1.0, 0.0);
        let target = Vec2::new(3.5, 1.5);

        let cfg = PlayerFlamethrowerConfig::load();
        let explicit =
            flame_hits_position(origin, dir, target, cfg.range, cfg.hit_half_width, &map);
        let default = flame_hits_position_configured(origin, dir, target, &map, &cfg);
        assert_eq!(explicit, default);
    }
}
