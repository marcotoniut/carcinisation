//! Shared collision utilities for player and entity movement.

use bevy_math::Vec2;

use crate::map::Map;

/// Try to move a position by `delta`, checking wall collision per axis.
///
/// Uses axis-separated sliding: tests X and Y independently so the entity
/// slides along walls rather than stopping dead.
pub fn try_move(position: &mut Vec2, delta: Vec2, margin: f32, map: &Map) {
    let new_pos = *position + delta;
    let test_x = margin.mul_add(delta.x.signum(), new_pos.x).floor() as i32;
    let test_y = margin.mul_add(delta.y.signum(), new_pos.y).floor() as i32;

    if map.get(test_x, position.y.floor() as i32) == 0 {
        position.x = new_pos.x;
    }
    if map.get(position.x.floor() as i32, test_y) == 0 {
        position.y = new_pos.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::test_map;

    #[test]
    fn move_into_open_space() {
        let map = test_map();
        let mut pos = Vec2::new(1.5, 1.5);
        try_move(&mut pos, Vec2::new(0.5, 0.0), 0.2, &map);
        assert!((pos.x - 2.0).abs() < 0.01);
    }

    #[test]
    fn blocked_by_wall() {
        let map = test_map();
        let mut pos = Vec2::new(1.1, 1.5);
        // Move west into the border wall.
        try_move(&mut pos, Vec2::new(-0.5, 0.0), 0.2, &map);
        // X should not change (wall at x=0).
        assert!((pos.x - 1.1).abs() < 0.01);
    }

    #[test]
    fn slides_along_wall() {
        let map = test_map();
        let mut pos = Vec2::new(1.1, 1.5);
        // Move diagonally into the west wall — should slide on Y.
        try_move(&mut pos, Vec2::new(-0.5, 0.5), 0.2, &map);
        assert!((pos.x - 1.1).abs() < 0.01, "X blocked");
        assert!((pos.y - 2.0).abs() < 0.01, "Y slides");
    }

    #[test]
    fn cannot_move_outside_map_bounds() {
        let map = test_map();
        // Near west edge — large westward delta should be blocked by OOB wall.
        let mut pos = Vec2::new(1.1, 1.5);
        try_move(&mut pos, Vec2::new(-2.0, 0.0), 0.2, &map);
        assert!(
            (pos.x - 1.1).abs() < 0.01,
            "should not escape west: {pos:?}"
        );

        // Near north edge — large northward delta should be blocked.
        let mut pos = Vec2::new(1.5, 1.1);
        try_move(&mut pos, Vec2::new(0.0, -2.0), 0.2, &map);
        assert!(
            (pos.y - 1.1).abs() < 0.01,
            "should not escape north: {pos:?}"
        );

        // Near east edge.
        let mut pos = Vec2::new(6.9, 1.5);
        try_move(&mut pos, Vec2::new(2.0, 0.0), 0.2, &map);
        assert!(
            (pos.x - 6.9).abs() < 0.01,
            "should not escape east: {pos:?}"
        );

        // Near south edge.
        let mut pos = Vec2::new(1.5, 6.9);
        try_move(&mut pos, Vec2::new(0.0, 2.0), 0.2, &map);
        assert!(
            (pos.y - 6.9).abs() < 0.01,
            "should not escape south: {pos:?}"
        );
    }

    #[test]
    fn cannot_move_into_interior_wall() {
        let map = test_map();
        // Wall at cell (3,2). Approach from (2.5, 2.5) moving east.
        let mut pos = Vec2::new(2.5, 2.5);
        try_move(&mut pos, Vec2::new(0.5, 0.0), 0.2, &map);
        assert!(
            pos.x < 3.0,
            "should be blocked by interior wall at (3,2): {pos:?}"
        );
    }
}
