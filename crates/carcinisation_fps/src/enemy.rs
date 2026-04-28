//! First-person enemy state and AI.

use bevy_math::Vec2;

use crate::camera::FpCamera;
use crate::map::FpMap;
use crate::raycast::cast_ray;

/// Enemy AI/lifecycle state.
#[derive(Clone, Debug)]
pub enum FpEnemyState {
    /// Stationary, not yet aware of the player.
    Idle,
    /// Moving toward the player.
    Chasing,
    /// In melee range, waiting for cooldown.
    Attacking { cooldown: f32 },
    /// Playing death animation.
    Dying { timer: f32 },
    /// Fully dead — pending removal or inert.
    Dead,
}

/// A runtime enemy instance.
#[derive(Clone, Debug)]
pub struct FpEnemy {
    pub position: Vec2,
    pub health: u32,
    pub max_health: u32,
    pub speed: f32,
    pub state: FpEnemyState,
    /// Collision/hitscan radius in map units.
    pub radius: f32,
    /// Detection range for chasing.
    pub detect_range: f32,
    /// Melee attack range.
    pub attack_range: f32,
    /// Damage dealt per attack.
    pub attack_damage: u32,
    /// Seconds between attacks.
    pub attack_interval: f32,
}

impl FpEnemy {
    /// Create a new enemy from spawn data.
    #[must_use]
    pub fn new(position: Vec2, health: u32, speed: f32) -> Self {
        Self {
            position,
            health,
            max_health: health,
            speed,
            state: FpEnemyState::Idle,
            radius: 0.3,
            detect_range: 8.0,
            attack_range: 0.8,
            attack_damage: 10,
            attack_interval: 1.0,
        }
    }

    /// Whether this enemy is alive (can be hit and acts).
    #[must_use]
    pub fn is_alive(&self) -> bool {
        !matches!(self.state, FpEnemyState::Dying { .. } | FpEnemyState::Dead)
    }

    /// Apply damage. Transitions to Dying if health reaches zero.
    pub fn take_damage(&mut self, amount: u32) {
        self.health = self.health.saturating_sub(amount);
        if self.health == 0 && self.is_alive() {
            self.state = FpEnemyState::Dying { timer: 0.5 };
        }
    }
}

/// Update all enemies for one frame. Returns newly spawned projectiles.
#[must_use]
pub fn tick_enemies(
    enemies: &mut [FpEnemy],
    player_pos: Vec2,
    map: &FpMap,
    dt: f32,
) -> Vec<FpProjectile> {
    let mut new_projectiles = Vec::new();

    for enemy in enemies.iter_mut() {
        match &mut enemy.state {
            FpEnemyState::Dead => continue,

            FpEnemyState::Dying { timer } => {
                *timer -= dt;
                if *timer <= 0.0 {
                    enemy.state = FpEnemyState::Dead;
                }
                continue;
            }

            FpEnemyState::Idle => {
                let dist = enemy.position.distance(player_pos);
                if dist < enemy.detect_range && has_line_of_sight(enemy.position, player_pos, map) {
                    enemy.state = FpEnemyState::Chasing;
                }
            }

            FpEnemyState::Chasing => {
                let to_player = player_pos - enemy.position;
                let dist = to_player.length();

                if dist < enemy.attack_range {
                    enemy.state = FpEnemyState::Attacking {
                        cooldown: enemy.attack_interval,
                    };
                } else if dist > 0.01 {
                    let move_dir = to_player / dist;
                    let step = move_dir * enemy.speed * dt;
                    try_move_enemy(enemy, step, map);
                }
            }

            FpEnemyState::Attacking { cooldown } => {
                let dist = enemy.position.distance(player_pos);

                // Player moved out of range or behind a wall — chase again.
                if dist > enemy.attack_range * 1.5
                    || !has_line_of_sight(enemy.position, player_pos, map)
                {
                    enemy.state = FpEnemyState::Chasing;
                    continue;
                }

                *cooldown -= dt;
                if *cooldown <= 0.0 {
                    // Spawn projectile instead of instant damage.
                    if let Some(proj) =
                        FpProjectile::new(enemy.position, player_pos, enemy.attack_damage)
                    {
                        new_projectiles.push(proj);
                    }
                    *cooldown = enemy.attack_interval;
                }
            }
        }
    }

    new_projectiles
}

/// Attempt to move an enemy by `delta`, checking wall collision.
fn try_move_enemy(enemy: &mut FpEnemy, delta: Vec2, map: &FpMap) {
    crate::collision::try_move(&mut enemy.position, delta, enemy.radius, map);
}

/// Check line of sight between two points using raycasting.
///
/// Direction is normalized so `cast_ray` returns true Euclidean distance,
/// making the comparison against `dist` valid.
fn has_line_of_sight(from: Vec2, to: Vec2, map: &FpMap) -> bool {
    let dir = to - from;
    let dist = dir.length();
    if dist < 0.01 {
        return true;
    }
    let hit = cast_ray(map, from, dir / dist);
    hit.distance > dist
}

/// Result of a hitscan shot.
#[derive(Debug)]
pub struct HitscanResult {
    /// Index of the enemy hit, if any.
    pub enemy_idx: Option<usize>,
    /// Distance to the hit point.
    pub distance: f32,
}

/// Fire a hitscan ray from the camera and check for enemy hits.
///
/// Tests each alive enemy for ray-circle intersection, returns the closest
/// hit that is nearer than the first wall.
#[must_use]
pub fn hitscan(camera: &FpCamera, enemies: &[FpEnemy], map: &FpMap) -> HitscanResult {
    let dir = camera.direction();
    let origin = camera.position;

    // Wall distance limits the hitscan range.
    let wall_hit = cast_ray(map, origin, dir);
    let max_dist = wall_hit.distance;

    let mut closest: Option<(usize, f32)> = None;

    for (i, enemy) in enemies.iter().enumerate() {
        if !enemy.is_alive() {
            continue;
        }

        // Ray-circle intersection: perpendicular distance from ray to enemy center.
        let to_enemy = enemy.position - origin;
        let along_ray = to_enemy.dot(dir);

        // Enemy is behind camera or beyond wall.
        if along_ray <= 0.0 || along_ray > max_dist {
            continue;
        }

        let perp_dist_sq = to_enemy.length_squared() - along_ray * along_ray;
        let radius_sq = enemy.radius * enemy.radius;

        if perp_dist_sq < radius_sq && closest.is_none_or(|(_, d)| along_ray < d) {
            closest = Some((i, along_ray));
        }
    }

    match closest {
        Some((idx, dist)) => HitscanResult {
            enemy_idx: Some(idx),
            distance: dist,
        },
        None => HitscanResult {
            enemy_idx: None,
            distance: max_dist,
        },
    }
}

// ---------------------------------------------------------------------------
// Projectiles
// ---------------------------------------------------------------------------

const PROJECTILE_SPEED: f32 = 4.0;
const PROJECTILE_HIT_RADIUS: f32 = 0.3;

/// An enemy projectile moving through map space.
#[derive(Clone, Debug)]
pub struct FpProjectile {
    pub position: Vec2,
    pub direction: Vec2,
    pub speed: f32,
    pub damage: u32,
    pub alive: bool,
}

impl FpProjectile {
    /// Spawn a projectile from `origin` aimed at `target`.
    /// Returns `None` if origin and target are too close (zero direction).
    #[must_use]
    pub fn new(origin: Vec2, target: Vec2, damage: u32) -> Option<Self> {
        let diff = target - origin;
        let len = diff.length();
        if len < 0.01 {
            return None;
        }
        Some(Self {
            position: origin,
            direction: diff / len,
            speed: PROJECTILE_SPEED,
            damage,
            alive: true,
        })
    }
}

/// Tick all projectiles. Returns total damage dealt to the player this frame.
/// Projectiles that hit the player or a wall are marked `alive = false`.
#[must_use]
pub fn tick_projectiles(
    projectiles: &mut Vec<FpProjectile>,
    player_pos: Vec2,
    map: &FpMap,
    dt: f32,
) -> u32 {
    let mut player_damage = 0;

    for proj in projectiles.iter_mut() {
        if !proj.alive {
            continue;
        }

        proj.position += proj.direction * proj.speed * dt;

        // Out of bounds?
        let gx = proj.position.x.floor() as i32;
        let gy = proj.position.y.floor() as i32;
        if gx < 0 || gy < 0 || gx >= map.width as i32 || gy >= map.height as i32 {
            proj.alive = false;
            continue;
        }

        // Hit wall?
        let cell = map.get(gx, gy);
        if cell > 0 {
            proj.alive = false;
            continue;
        }

        // Hit player?
        if proj.position.distance(player_pos) < PROJECTILE_HIT_RADIUS {
            player_damage += proj.damage;
            proj.alive = false;
        }
    }

    // Remove dead projectiles.
    projectiles.retain(|p| p.alive);

    player_damage
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::test_map;

    fn make_enemy(x: f32, y: f32) -> FpEnemy {
        FpEnemy::new(Vec2::new(x, y), 30, 1.5)
    }

    // --- take_damage ---

    #[test]
    fn take_damage_reduces_health() {
        let mut e = make_enemy(4.0, 4.0);
        e.take_damage(10);
        assert_eq!(e.health, 20);
        assert!(e.is_alive());
    }

    #[test]
    fn take_damage_transitions_to_dying_at_zero() {
        let mut e = make_enemy(4.0, 4.0);
        e.take_damage(30);
        assert_eq!(e.health, 0);
        assert!(matches!(e.state, FpEnemyState::Dying { .. }));
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
        let map = test_map();
        let mut enemies = vec![make_enemy(3.0, 1.5)];
        // Player close and in LOS.
        let _ = tick_enemies(&mut enemies, Vec2::new(1.5, 1.5), &map, 0.016);
        assert!(matches!(enemies[0].state, FpEnemyState::Chasing));
    }

    #[test]
    fn chasing_enemy_enters_attack_range() {
        let map = test_map();
        let mut enemies = vec![make_enemy(2.0, 1.5)];
        enemies[0].state = FpEnemyState::Chasing;
        // Player within attack range.
        let _ = tick_enemies(&mut enemies, Vec2::new(1.5, 1.5), &map, 0.016);
        assert!(matches!(enemies[0].state, FpEnemyState::Attacking { .. }));
    }

    #[test]
    fn attacking_enemy_spawns_projectile() {
        let map = test_map();
        let mut enemies = vec![make_enemy(1.5, 1.5)];
        enemies[0].state = FpEnemyState::Attacking { cooldown: 0.01 };
        // Player nearby but not at same position (avoids zero-direction filter).
        let projectiles = tick_enemies(&mut enemies, Vec2::new(2.0, 1.5), &map, 0.02);
        assert!(!projectiles.is_empty());
        assert!(projectiles[0].damage > 0);
    }

    #[test]
    fn dying_enemy_transitions_to_dead() {
        let map = test_map();
        let mut enemies = vec![make_enemy(4.0, 4.0)];
        enemies[0].state = FpEnemyState::Dying { timer: 0.1 };
        let _ = tick_enemies(&mut enemies, Vec2::new(1.5, 1.5), &map, 0.2);
        assert!(matches!(enemies[0].state, FpEnemyState::Dead));
    }

    // --- hitscan ---

    #[test]
    fn hitscan_hits_enemy_in_front() {
        let map = test_map();
        let cam = FpCamera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0, // facing east
            ..Default::default()
        };
        let enemies = vec![make_enemy(3.0, 1.5)]; // directly ahead
        let result = hitscan(&cam, &enemies, &map);
        assert_eq!(result.enemy_idx, Some(0));
    }

    #[test]
    fn hitscan_misses_enemy_behind_camera() {
        let map = test_map();
        let cam = FpCamera {
            position: Vec2::new(4.0, 1.5),
            angle: 0.0, // facing east
            ..Default::default()
        };
        let enemies = vec![make_enemy(2.0, 1.5)]; // behind
        let result = hitscan(&cam, &enemies, &map);
        assert!(result.enemy_idx.is_none());
    }

    #[test]
    fn hitscan_misses_enemy_off_to_side() {
        let map = test_map();
        let cam = FpCamera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            ..Default::default()
        };
        let enemies = vec![make_enemy(3.0, 3.0)]; // far off to the side
        let result = hitscan(&cam, &enemies, &map);
        assert!(result.enemy_idx.is_none());
    }

    // --- projectiles ---

    #[test]
    fn projectile_zero_direction_returns_none() {
        let origin = Vec2::new(1.5, 1.5);
        assert!(FpProjectile::new(origin, origin, 10).is_none());
    }

    #[test]
    fn projectile_hits_wall() {
        let map = test_map();
        // Aimed west, will hit the border wall at x=0.
        let mut projs =
            vec![FpProjectile::new(Vec2::new(1.5, 1.5), Vec2::new(0.5, 1.5), 10).unwrap()];
        for _ in 0..60 {
            let _ = tick_projectiles(&mut projs, Vec2::new(5.0, 5.0), &map, 0.016);
        }
        assert!(
            projs.is_empty(),
            "projectile should be removed after hitting wall"
        );
    }

    #[test]
    fn projectile_hits_player() {
        let map = test_map();
        let player = Vec2::new(3.0, 1.5);
        let mut projs = vec![FpProjectile::new(Vec2::new(1.5, 1.5), player, 25).unwrap()];
        let mut total_damage = 0;
        for _ in 0..60 {
            total_damage += tick_projectiles(&mut projs, player, &map, 0.016);
        }
        assert_eq!(total_damage, 25);
        assert!(projs.is_empty());
    }

    #[test]
    fn tick_projectiles_empty_list() {
        let map = test_map();
        let mut projs = Vec::new();
        let dmg = tick_projectiles(&mut projs, Vec2::ZERO, &map, 0.016);
        assert_eq!(dmg, 0);
    }

    // --- hitscan ---

    #[test]
    fn hitscan_picks_closest_enemy() {
        let map = test_map();
        let cam = FpCamera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            ..Default::default()
        };
        let enemies = vec![
            make_enemy(5.0, 1.5), // far
            make_enemy(3.0, 1.5), // close
        ];
        let result = hitscan(&cam, &enemies, &map);
        assert_eq!(result.enemy_idx, Some(1));
    }
}
