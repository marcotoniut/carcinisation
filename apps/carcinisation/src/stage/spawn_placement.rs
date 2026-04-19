//! Shared enemy spawn placement resolution.
//!
//! Used by both the runtime spawn path and the editor preview to derive
//! world-space positions from spawn data.  When an [`EnemySpawn`] carries an
//! `altitude`, screen Y is computed from the stage projection model:
//! `floor_y_for_depth(depth) + altitude`.  Otherwise the raw `coordinates`
//! field is used as-is (legacy behaviour).

use super::{data::EnemySpawn, projection::ProjectionProfile};
use bevy::prelude::Vec2;

/// Resolve the world-space spawn position for an enemy.
///
/// When `altitude` is set, Y is derived from the projection floor line at the
/// spawn's depth.  Otherwise falls back to raw `coordinates`.
///
/// `camera_offset` is added in both paths (same semantics as the existing
/// `offset + coordinates` pattern).
#[must_use]
pub fn resolve_enemy_position(
    spawn: &EnemySpawn,
    camera_offset: Vec2,
    projection: &ProjectionProfile,
) -> Vec2 {
    let local = resolve_enemy_position_local(spawn, projection);
    camera_offset + local
}

/// Resolve spawn position without camera offset (for editor preview).
#[must_use]
pub fn resolve_enemy_position_local(spawn: &EnemySpawn, projection: &ProjectionProfile) -> Vec2 {
    match spawn.altitude {
        Some(alt) => {
            let floor_y = projection.floor_y_for_depth(spawn.depth.to_i8());
            Vec2::new(spawn.coordinates.x, floor_y + alt)
        }
        None => spawn.coordinates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::components::placement::Depth;
    use crate::stage::data::EnemySpawn;

    fn test_profile() -> ProjectionProfile {
        ProjectionProfile {
            horizon_y: 72.0,
            floor_base_y: -14.4,
            bias_power: 3.0,
        }
    }

    fn mosquiton_at(depth: Depth, altitude: f32, x: f32) -> EnemySpawn {
        EnemySpawn::mosquiton_base()
            .with_coordinates(Vec2::new(x, 0.0))
            .with_depth(depth)
            .with_altitude(altitude)
    }

    #[test]
    fn flying_placement_derives_y_from_floor_plus_altitude() {
        let profile = test_profile();
        let spawn = mosquiton_at(Depth::Five, 20.0, 80.0);
        let pos = resolve_enemy_position_local(&spawn, &profile);
        let expected_y = profile.floor_y_for_depth(5) + 20.0;
        assert!((pos.y - expected_y).abs() < 0.01);
        assert!((pos.x - 80.0).abs() < 0.01);
    }

    #[test]
    fn no_altitude_uses_raw_coordinates() {
        let profile = test_profile();
        let spawn = EnemySpawn::mosquiton_base().with_coordinates(Vec2::new(50.0, 100.0));
        let pos = resolve_enemy_position_local(&spawn, &profile);
        assert!((pos.x - 50.0).abs() < 0.01);
        assert!((pos.y - 100.0).abs() < 0.01);
    }

    #[test]
    fn flying_placement_consistent_across_depths() {
        let profile = test_profile();
        for d in 1..=9_i8 {
            let depth = Depth::try_from(d).unwrap();
            let spawn = mosquiton_at(depth, 30.0, 80.0);
            let pos = resolve_enemy_position_local(&spawn, &profile);
            let expected_y = profile.floor_y_for_depth(d) + 30.0;
            assert!(
                (pos.y - expected_y).abs() < 0.01,
                "depth {d}: expected y={expected_y}, got y={}",
                pos.y
            );
        }
    }

    #[test]
    fn zero_altitude_sits_on_floor() {
        let profile = test_profile();
        let spawn = mosquiton_at(Depth::Three, 0.0, 80.0);
        let pos = resolve_enemy_position_local(&spawn, &profile);
        let floor_y = profile.floor_y_for_depth(3);
        assert!((pos.y - floor_y).abs() < 0.01);
    }

    #[test]
    fn camera_offset_applied_correctly() {
        let profile = test_profile();
        let spawn = mosquiton_at(Depth::Five, 20.0, 80.0);
        let offset = Vec2::new(100.0, 50.0);
        let pos = resolve_enemy_position(&spawn, offset, &profile);
        let expected_x = 100.0 + 80.0;
        let expected_y = 50.0 + profile.floor_y_for_depth(5) + 20.0;
        assert!((pos.x - expected_x).abs() < 0.01);
        assert!((pos.y - expected_y).abs() < 0.01);
    }

    #[test]
    fn editor_and_runtime_derive_same_position() {
        let profile = test_profile();
        let spawn = mosquiton_at(Depth::Five, 25.0, 80.0);
        let camera_offset = Vec2::new(100.0, 50.0);

        let runtime_pos = resolve_enemy_position(&spawn, camera_offset, &profile);
        let editor_pos = resolve_enemy_position_local(&spawn, &profile);

        assert!((runtime_pos.x - (editor_pos.x + camera_offset.x)).abs() < 0.01);
        assert!((runtime_pos.y - (editor_pos.y + camera_offset.y)).abs() < 0.01);
    }

    #[test]
    fn altitude_placement_above_floor_for_all_depths() {
        let profile = test_profile();
        let altitude = 45.0;
        for d in 1..=9_i8 {
            let depth = Depth::try_from(d).unwrap();
            let spawn = mosquiton_at(depth, altitude, 80.0);
            let pos = resolve_enemy_position_local(&spawn, &profile);
            let floor_y = profile.floor_y_for_depth(d);
            assert!(
                pos.y > floor_y,
                "depth {d}: entity y ({}) should be above floor y ({floor_y})",
                pos.y,
            );
        }
    }

    #[test]
    fn flight_altitude_keeps_entity_clearly_above_floor() {
        let profile = test_profile();
        // Park-style altitude: 45px at depth 5, floor at ~61
        let spawn = mosquiton_at(Depth::Five, 45.0, 80.0);
        let pos = resolve_enemy_position_local(&spawn, &profile);
        let floor_y = profile.floor_y_for_depth(5);
        let clearance = pos.y - floor_y;
        assert!(
            clearance >= 40.0,
            "flying mosquiton should have >=40px clearance, got {clearance}"
        );
    }

    #[test]
    fn grounded_enemy_uses_raw_coordinates_at_floor() {
        let profile = test_profile();
        // No altitude = grounded, Y comes from coordinates
        let floor_y = profile.floor_y_for_depth(5);
        let spawn = EnemySpawn::mosquiton_base()
            .with_coordinates(Vec2::new(80.0, floor_y))
            .with_depth(Depth::Five);
        let pos = resolve_enemy_position_local(&spawn, &profile);
        assert!(
            (pos.y - floor_y).abs() < 0.01,
            "grounded enemy should be at floor, got y={}, floor={floor_y}",
            pos.y,
        );
    }

    #[test]
    fn flying_and_grounded_produce_different_y() {
        let profile = test_profile();
        let floor_y = profile.floor_y_for_depth(5);
        let flying = mosquiton_at(Depth::Five, 45.0, 80.0);
        let grounded = EnemySpawn::mosquiton_base()
            .with_coordinates(Vec2::new(80.0, floor_y))
            .with_depth(Depth::Five);

        let fly_pos = resolve_enemy_position_local(&flying, &profile);
        let ground_pos = resolve_enemy_position_local(&grounded, &profile);

        assert!(
            (fly_pos.y - ground_pos.y).abs() > 30.0,
            "flying ({}) and grounded ({}) should be clearly separated",
            fly_pos.y,
            ground_pos.y,
        );
    }
}
