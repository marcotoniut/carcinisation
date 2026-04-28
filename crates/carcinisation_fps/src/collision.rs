//! Shared collision utilities for player and entity movement.

use bevy_math::Vec2;

use crate::map::FpMap;

/// Try to move a position by `delta`, checking wall collision per axis.
///
/// Uses axis-separated sliding: tests X and Y independently so the entity
/// slides along walls rather than stopping dead.
pub fn try_move(position: &mut Vec2, delta: Vec2, margin: f32, map: &FpMap) {
    let new_pos = *position + delta;
    let test_x = (new_pos.x + margin * delta.x.signum()).floor() as i32;
    let test_y = (new_pos.y + margin * delta.y.signum()).floor() as i32;

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
}
