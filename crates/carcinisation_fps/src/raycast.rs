//! DDA raycasting against an [`Map`] grid.

use bevy_math::Vec2;

use crate::map::Map;

/// Which side of a grid cell the ray hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HitSide {
    /// Hit a wall face perpendicular to X (vertical wall edge).
    Vertical,
    /// Hit a wall face perpendicular to Y (horizontal wall edge).
    Horizontal,
}

/// Stable identity for one visible wall face.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WallSurfaceId {
    pub cell_x: i32,
    pub cell_y: i32,
    pub side: HitSide,
    pub normal_sign: i8,
}

/// Result of casting a single ray into the map.
#[derive(Clone, Debug)]
pub struct RayHit {
    /// Perpendicular distance from the camera plane to the wall.
    pub distance: f32,
    /// Wall cell value (>0). 0 means the ray escaped the map.
    pub wall_id: u8,
    /// Fractional hit position along the wall face (0.0–1.0).
    pub wall_x: f32,
    /// Which side of the cell was hit.
    pub side: HitSide,
    /// Wall face hit by the ray. `None` means the ray escaped the map.
    pub surface_id: Option<WallSurfaceId>,
}

/// Cast a ray from `origin` in direction `dir` through `map` using DDA.
///
/// `dir` does NOT need to be normalized — the perpendicular distance
/// calculation accounts for the ray direction magnitude, which avoids
/// fisheye distortion when used with a camera-plane projection.
#[must_use]
pub fn cast_ray(map: &Map, origin: Vec2, dir: Vec2) -> RayHit {
    let mut map_x = origin.x.floor() as i32;
    let mut map_y = origin.y.floor() as i32;

    let delta_dist_x = if dir.x == 0.0 {
        f32::MAX
    } else {
        (1.0 / dir.x).abs()
    };
    let delta_dist_y = if dir.y == 0.0 {
        f32::MAX
    } else {
        (1.0 / dir.y).abs()
    };

    let (step_x, mut side_dist_x) = if dir.x < 0.0 {
        (-1_i32, (origin.x - map_x as f32) * delta_dist_x)
    } else {
        (1_i32, (map_x as f32 + 1.0 - origin.x) * delta_dist_x)
    };

    let (step_y, mut side_dist_y) = if dir.y < 0.0 {
        (-1_i32, (origin.y - map_y as f32) * delta_dist_y)
    } else {
        (1_i32, (map_y as f32 + 1.0 - origin.y) * delta_dist_y)
    };

    let max_steps = (map.width + map.height) * 2;

    for _ in 0..max_steps {
        let side = if side_dist_x < side_dist_y {
            side_dist_x += delta_dist_x;
            map_x += step_x;
            HitSide::Vertical
        } else {
            side_dist_y += delta_dist_y;
            map_y += step_y;
            HitSide::Horizontal
        };

        let cell = map.get(map_x, map_y);
        if cell > 0 {
            let perp_dist = match side {
                HitSide::Vertical => side_dist_x - delta_dist_x,
                HitSide::Horizontal => side_dist_y - delta_dist_y,
            };

            let wall_x = match side {
                HitSide::Vertical => origin.y + perp_dist * dir.y,
                HitSide::Horizontal => origin.x + perp_dist * dir.x,
            };
            let wall_x = wall_x - wall_x.floor();

            return RayHit {
                distance: perp_dist.max(0.001),
                wall_id: cell,
                wall_x,
                side,
                surface_id: Some(WallSurfaceId {
                    cell_x: map_x,
                    cell_y: map_y,
                    side,
                    normal_sign: match side {
                        HitSide::Vertical => -step_x as i8,
                        HitSide::Horizontal => -step_y as i8,
                    },
                }),
            };
        }
    }

    // Ray escaped the map.
    RayHit {
        distance: f32::MAX,
        wall_id: 0,
        wall_x: 0.0,
        side: HitSide::Vertical,
        surface_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::test_map;

    #[test]
    fn ray_hits_east_wall() {
        let map = test_map();
        // From center, facing east.
        let hit = cast_ray(&map, Vec2::new(4.0, 4.0), Vec2::new(1.0, 0.0));
        assert_eq!(hit.wall_id, 2); // hits interior wall at x=5
        assert_eq!(hit.side, HitSide::Vertical);
        assert!(hit.distance > 0.0 && hit.distance < 4.0);
    }

    #[test]
    fn ray_hits_north_wall() {
        let map = test_map();
        // From center, facing north.
        let hit = cast_ray(&map, Vec2::new(1.5, 1.5), Vec2::new(0.0, 1.0));
        assert_eq!(hit.wall_id, 1); // hits border wall
        assert_eq!(hit.side, HitSide::Horizontal);
    }

    #[test]
    fn ray_near_wall_no_crash() {
        let map = test_map();
        // Very close to a wall.
        let hit = cast_ray(&map, Vec2::new(1.01, 1.01), Vec2::new(-1.0, 0.0));
        assert!(hit.wall_id > 0);
        assert!(hit.distance > 0.0);
    }

    #[test]
    fn wall_x_centered_on_vertical_hit() {
        let map = test_map();
        // Origin at (1.5, 1.5), facing east. Hits the vertical wall at x=2.
        // The ray hits at y=1.5 within the cell face, so wall_x = 0.5.
        let hit = cast_ray(&map, Vec2::new(1.5, 1.5), Vec2::new(1.0, 0.0));
        assert_eq!(hit.side, HitSide::Vertical);
        assert!(
            (hit.wall_x - 0.5).abs() < 0.01,
            "wall_x should be ~0.5, got {}",
            hit.wall_x
        );
    }

    #[test]
    fn wall_x_centered_on_horizontal_hit() {
        let map = test_map();
        // Origin at (1.5, 1.5), facing north (+Y). Hits the horizontal wall at y=2.
        // The ray hits at x=1.5 within the cell face, so wall_x = 0.5.
        let hit = cast_ray(&map, Vec2::new(1.5, 1.5), Vec2::new(0.0, 1.0));
        assert_eq!(hit.side, HitSide::Horizontal);
        assert!(
            (hit.wall_x - 0.5).abs() < 0.01,
            "wall_x should be ~0.5, got {}",
            hit.wall_x
        );
    }
}
