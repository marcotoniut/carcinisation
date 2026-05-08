//! First-person enemy state and AI — re-exported from `carcinisation_fps_core`.
//!
//! All core gameplay types (`Enemy`, `EnemyState`, `DamageFlicker`, `Projectile`,
//! `HitscanResult`, etc.) and functions (`tick_single_enemy`, `tick_enemies`,
//! `tick_projectiles`, `hitscan`, etc.) are canonical in `carcinisation_fps_core`
//! and re-exported here for backward compatibility.

// Re-export all public types and functions from fps_core::enemy.
pub use carcinisation_fps_core::enemy::*;

// Re-export DamageKind so callers can use `crate::enemy::DamageKind`.
pub use carcinisation_fps_core::fire_death::{DamageKind, corpse_seed};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::test_map;

    fn make_enemy(x: f32, y: f32) -> Enemy {
        Enemy::new(bevy_math::Vec2::new(x, y), 30, 1.5)
    }

    // --- take_damage ---

    #[test]
    fn take_damage_reduces_health() {
        let mut e = make_enemy(4.0, 4.0);
        e.take_damage(10);
        assert_eq!(e.health, 20);
        assert!(e.is_alive());
        assert!(e.damage_flicker.is_some());
    }

    #[test]
    fn repeated_damage_does_not_restart_active_flicker() {
        let mut e = make_enemy(4.0, 4.0);
        e.take_damage_from(1, DamageKind::Fire, 1.25);
        let first = e.damage_flicker;
        e.take_damage_from(1, DamageKind::Fire, 1.25);
        assert_eq!(e.damage_flicker, first);
    }

    #[test]
    fn fire_death_clears_normal_damage_flicker() {
        let mut e = make_enemy(4.0, 4.0);
        e.take_damage(1);
        assert!(e.damage_flicker.is_some());
        e.take_damage_from(29, DamageKind::Fire, 1.25);
        assert!(matches!(e.state, EnemyState::BurningCorpse { .. }));
        assert!(e.damage_flicker.is_none());
    }

    #[test]
    fn take_damage_transitions_to_dying_at_zero() {
        let mut e = make_enemy(4.0, 4.0);
        e.take_damage(30);
        assert_eq!(e.health, 0);
        assert!(matches!(e.state, EnemyState::Dying { .. }));
        assert!(!e.is_alive());
    }

    #[test]
    fn fire_damage_transitions_to_burning_corpse_at_zero() {
        let mut e = make_enemy(4.0, 4.0);
        e.take_damage_from(30, DamageKind::Fire, 1.25);
        assert_eq!(e.health, 0);
        assert!(matches!(
            e.state,
            EnemyState::BurningCorpse { timer, .. } if (timer - 1.25).abs() < 0.001
        ));
        assert!(!e.is_alive());
    }

    #[test]
    fn take_damage_saturates_at_zero() {
        let mut e = make_enemy(4.0, 4.0);
        e.take_damage(100);
        assert_eq!(e.health, 0);
    }

    // --- tick_enemies state transitions ---

    #[test]
    fn idle_enemy_detects_nearby_player() {
        use bevy_math::Vec2;
        let map = test_map();
        let mut enemies = vec![make_enemy(3.0, 1.5)];
        let _ = tick_enemies(&mut enemies, Vec2::new(1.5, 1.5), &map, 0.016);
        assert!(matches!(enemies[0].state, EnemyState::Chasing));
    }

    #[test]
    fn chasing_enemy_enters_attack_range() {
        use bevy_math::Vec2;
        let map = test_map();
        let mut enemies = vec![make_enemy(2.0, 1.5)];
        enemies[0].state = EnemyState::Chasing;
        let _ = tick_enemies(&mut enemies, Vec2::new(1.5, 1.5), &map, 0.016);
        assert!(matches!(enemies[0].state, EnemyState::Attacking { .. }));
    }

    #[test]
    fn attacking_enemy_spawns_projectile() {
        use bevy_math::Vec2;
        let map = test_map();
        let mut enemies = vec![make_enemy(1.5, 1.5)];
        enemies[0].state = EnemyState::Attacking { cooldown: 0.01 };
        let projectiles = tick_enemies(&mut enemies, Vec2::new(2.0, 1.5), &map, 0.02);
        assert!(!projectiles.is_empty());
        assert!(projectiles[0].damage > 0);
    }

    #[test]
    fn dying_enemy_transitions_to_dead() {
        use bevy_math::Vec2;
        let map = test_map();
        let mut enemies = vec![make_enemy(4.0, 4.0)];
        enemies[0].state = EnemyState::Dying { timer: 0.1 };
        let _ = tick_enemies(&mut enemies, Vec2::new(1.5, 1.5), &map, 0.2);
        assert!(matches!(enemies[0].state, EnemyState::Dead));
    }

    #[test]
    fn burning_corpse_transitions_to_dead_without_attacking() {
        use bevy_math::Vec2;
        let map = test_map();
        let mut enemies = vec![make_enemy(1.5, 1.5)];
        enemies[0].state = EnemyState::BurningCorpse {
            timer: 0.1,
            seed: 123,
        };
        let projectiles = tick_enemies(&mut enemies, Vec2::new(2.0, 1.5), &map, 0.2);
        assert!(projectiles.is_empty());
        assert!(matches!(enemies[0].state, EnemyState::Dead));
    }

    // --- hitscan ---

    #[test]
    fn hitscan_hits_enemy_in_front() {
        use crate::camera::Camera;
        use bevy_math::Vec2;
        let map = test_map();
        let cam = Camera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            ..Default::default()
        };
        let enemies = vec![make_enemy(3.0, 1.5)];
        let result = hitscan(&cam, &enemies, &map);
        assert_eq!(result.enemy_idx, Some(0));
    }

    #[test]
    fn hitscan_misses_enemy_behind_camera() {
        use crate::camera::Camera;
        use bevy_math::Vec2;
        let map = test_map();
        let cam = Camera {
            position: Vec2::new(4.0, 1.5),
            angle: 0.0,
            ..Default::default()
        };
        let enemies = vec![make_enemy(2.0, 1.5)];
        let result = hitscan(&cam, &enemies, &map);
        assert!(result.enemy_idx.is_none());
    }

    #[test]
    fn hitscan_misses_enemy_off_to_side() {
        use crate::camera::Camera;
        use bevy_math::Vec2;
        let map = test_map();
        let cam = Camera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            ..Default::default()
        };
        let enemies = vec![make_enemy(3.0, 3.0)];
        let result = hitscan(&cam, &enemies, &map);
        assert!(result.enemy_idx.is_none());
    }

    // --- projectiles ---

    #[test]
    fn projectile_zero_direction_returns_none() {
        use bevy_math::Vec2;
        let origin = Vec2::new(1.5, 1.5);
        assert!(Projectile::new(origin, origin, 10).is_none());
    }

    #[test]
    fn projectile_hits_wall() {
        use bevy_math::Vec2;
        let map = test_map();
        let mut projs =
            vec![Projectile::new(Vec2::new(1.5, 1.5), Vec2::new(0.5, 1.5), 10).unwrap()];
        for _ in 0..60 {
            let _ = tick_projectiles(&mut projs, Vec2::new(5.0, 5.0), &map, 0.016);
        }
        assert!(
            projs.is_empty(),
            "projectile should be removed after hitting wall"
        );
    }

    #[test]
    fn hitscan_picks_closest_enemy() {
        use crate::camera::Camera;
        use bevy_math::Vec2;
        let map = test_map();
        let cam = Camera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            ..Default::default()
        };
        let enemies = vec![make_enemy(5.0, 1.5), make_enemy(3.0, 1.5)];
        let result = hitscan(&cam, &enemies, &map);
        assert_eq!(result.enemy_idx, Some(1));
    }
}
