//! Shared combat hit-detection — flamethrower line-distance check.

use bevy_math::Vec2;

use crate::config::PlayerFlamethrowerConfig;
use crate::map::Map;
use crate::raycast::cast_ray;

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
    let ray = cast_ray(map, origin, to_target / dist);
    ray.distance >= dist
}

/// Convenience wrapper using values from a `PlayerFlamethrowerConfig`.
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

#[cfg(test)]
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
