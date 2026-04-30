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
const PROJECTILE_LIFETIME: f32 = 3.0;

/// An enemy projectile moving through map space.
#[derive(Clone, Debug)]
pub struct FpProjectile {
    pub position: Vec2,
    pub source_position: Vec2,
    pub direction: Vec2,
    pub speed: f32,
    pub radius: f32,
    pub damage: u32,
    pub lifetime: f32,
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
            source_position: origin,
            direction: diff / len,
            speed: PROJECTILE_SPEED,
            radius: PROJECTILE_HIT_RADIUS,
            damage,
            lifetime: PROJECTILE_LIFETIME,
            alive: true,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FpProjectileImpactKind {
    Hit,
    Destroy,
}

#[derive(Clone, Debug)]
pub struct FpProjectileImpact {
    pub position: Vec2,
    pub kind: FpProjectileImpactKind,
    pub age: f32,
    pub lifetime: f32,
}

impl FpProjectileImpact {
    #[must_use]
    pub fn hit(position: Vec2) -> Self {
        Self {
            position,
            kind: FpProjectileImpactKind::Hit,
            age: 0.0,
            lifetime: 0.18,
        }
    }

    #[must_use]
    pub fn destroy(position: Vec2) -> Self {
        Self {
            position,
            kind: FpProjectileImpactKind::Destroy,
            age: 0.0,
            lifetime: 0.3,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct FpProjectileTickResult {
    pub player_damage: u32,
    pub damage_source: Option<Vec2>,
    pub impacts: Vec<FpProjectileImpact>,
}

/// Tick all projectile impact effects and remove finished ones.
pub fn tick_projectile_impacts(impacts: &mut Vec<FpProjectileImpact>, dt: f32) {
    for impact in impacts.iter_mut() {
        impact.age += dt;
    }
    impacts.retain(|impact| impact.age < impact.lifetime);
}

/// Tick all projectiles. Returns damage and impact events this frame.
/// Projectiles that hit the player or a wall are marked `alive = false`.
#[must_use]
pub fn tick_projectiles(
    projectiles: &mut Vec<FpProjectile>,
    player_pos: Vec2,
    map: &FpMap,
    dt: f32,
) -> FpProjectileTickResult {
    let mut result = FpProjectileTickResult::default();

    for proj in projectiles.iter_mut() {
        if !proj.alive {
            continue;
        }

        proj.lifetime -= dt;
        if proj.lifetime <= 0.0 {
            proj.alive = false;
            continue;
        }

        let previous_position = proj.position;
        let step = proj.direction * proj.speed * dt;
        let travel_distance = step.length();
        if travel_distance <= f32::EPSILON {
            continue;
        }
        let next_position = previous_position + step;

        let wall_hit = cast_ray(map, previous_position, proj.direction);
        let wall_distance = (wall_hit.wall_id > 0 && wall_hit.distance <= travel_distance)
            .then_some(wall_hit.distance);
        let player_distance =
            segment_circle_hit_distance(previous_position, next_position, player_pos, proj.radius);

        match earliest_projectile_collision(wall_distance, player_distance) {
            Some(FpProjectileCollision::Wall(distance)) => {
                proj.alive = false;
                result.impacts.push(FpProjectileImpact::hit(
                    previous_position + proj.direction * distance,
                ));
            }
            Some(FpProjectileCollision::Player(distance)) => {
                proj.alive = false;
                result.player_damage += proj.damage;
                result.damage_source = Some(proj.source_position);
                result.impacts.push(FpProjectileImpact::hit(
                    previous_position + proj.direction * distance,
                ));
            }
            None => {
                proj.position = next_position;
            }
        }

        // Despawn if the segment left an unbounded map without hitting a wall.
        let gx = proj.position.x.floor() as i32;
        let gy = proj.position.y.floor() as i32;
        if proj.alive && (gx < 0 || gy < 0 || gx >= map.width as i32 || gy >= map.height as i32) {
            proj.alive = false;
            result
                .impacts
                .push(FpProjectileImpact::hit(segment_map_exit_point(
                    previous_position,
                    proj.position,
                    map,
                )));
        }
    }

    // Remove dead projectiles.
    projectiles.retain(|p| p.alive);

    result
}

enum FpProjectileCollision {
    Wall(f32),
    Player(f32),
}

fn earliest_projectile_collision(
    wall_distance: Option<f32>,
    player_distance: Option<f32>,
) -> Option<FpProjectileCollision> {
    match (wall_distance, player_distance) {
        (Some(wall), Some(player)) if player < wall => Some(FpProjectileCollision::Player(player)),
        (Some(wall), _) => Some(FpProjectileCollision::Wall(wall)),
        (None, Some(player)) => Some(FpProjectileCollision::Player(player)),
        (None, None) => None,
    }
}

fn segment_circle_hit_distance(start: Vec2, end: Vec2, center: Vec2, radius: f32) -> Option<f32> {
    let segment = end - start;
    let len_sq = segment.length_squared();
    if len_sq <= f32::EPSILON {
        return (start.distance(center) <= radius).then_some(0.0);
    }

    let from_center = start - center;
    let a = len_sq;
    let b = 2.0 * from_center.dot(segment);
    let c = from_center.length_squared() - radius * radius;
    if c <= 0.0 {
        return Some(0.0);
    }

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }

    let sqrt_discriminant = discriminant.sqrt();
    let first_t = (-b - sqrt_discriminant) / (2.0 * a);
    let second_t = (-b + sqrt_discriminant) / (2.0 * a);
    [first_t, second_t]
        .into_iter()
        .filter(|t| (0.0..=1.0).contains(t))
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|t| t * segment.length())
}

fn segment_map_exit_point(start: Vec2, end: Vec2, map: &FpMap) -> Vec2 {
    let delta = end - start;
    let max_x = map.width as f32;
    let max_y = map.height as f32;
    let mut best_t: Option<f32> = None;

    for t in [
        (delta.x < 0.0).then_some((0.0 - start.x) / delta.x),
        (delta.x > 0.0).then_some((max_x - start.x) / delta.x),
        (delta.y < 0.0).then_some((0.0 - start.y) / delta.y),
        (delta.y > 0.0).then_some((max_y - start.y) / delta.y),
    ]
    .into_iter()
    .flatten()
    {
        if !(0.0..=1.0).contains(&t) {
            continue;
        }
        let point = start + delta * t;
        if point.x >= -f32::EPSILON
            && point.y >= -f32::EPSILON
            && point.x <= max_x + f32::EPSILON
            && point.y <= max_y + f32::EPSILON
            && best_t.is_none_or(|current| t < current)
        {
            best_t = Some(t);
        }
    }

    best_t.map_or(end, |t| start + delta * t)
}

/// Hitscan against active projectiles, returning the closest shootable projectile.
#[must_use]
pub fn hitscan_projectiles(
    camera: &FpCamera,
    projectiles: &[FpProjectile],
    map: &FpMap,
) -> Option<(usize, f32)> {
    let dir = camera.direction();
    let origin = camera.position;
    let wall_hit = cast_ray(map, origin, dir);
    let max_dist = wall_hit.distance;

    let mut closest: Option<(usize, f32)> = None;
    for (idx, projectile) in projectiles.iter().enumerate() {
        if !projectile.alive {
            continue;
        }

        let to_projectile = projectile.position - origin;
        let along_ray = to_projectile.dot(dir);
        if along_ray <= 0.0 || along_ray > max_dist {
            continue;
        }

        let perp_dist_sq = to_projectile.length_squared() - along_ray * along_ray;
        let radius_sq = projectile.radius * projectile.radius;
        if perp_dist_sq < radius_sq && closest.is_none_or(|(_, dist)| along_ray < dist) {
            closest = Some((idx, along_ray));
        }
    }

    closest
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
    fn projectile_sweeps_wall_collision_during_large_step() {
        let map = test_map();
        let mut proj = FpProjectile::new(Vec2::new(1.5, 1.5), Vec2::new(0.5, 1.5), 10).unwrap();
        proj.speed = 100.0;
        let mut projs = vec![proj];

        let result = tick_projectiles(&mut projs, Vec2::new(5.0, 5.0), &map, 0.1);

        assert!(projs.is_empty());
        assert_eq!(result.impacts.len(), 1);
        assert!(
            (result.impacts[0].position.x - 1.0).abs() < 0.01,
            "impact should be at the wall boundary, got {:?}",
            result.impacts[0].position
        );
    }

    #[test]
    fn projectile_wall_hit_creates_splash() {
        let map = test_map();
        let mut projs =
            vec![FpProjectile::new(Vec2::new(1.5, 1.5), Vec2::new(0.5, 1.5), 10).unwrap()];
        let mut impacts = Vec::new();
        for _ in 0..60 {
            let result = tick_projectiles(&mut projs, Vec2::new(5.0, 5.0), &map, 0.016);
            impacts.extend(result.impacts);
        }
        assert!(
            impacts
                .iter()
                .any(|impact| impact.kind == FpProjectileImpactKind::Hit)
        );
    }

    #[test]
    fn projectile_hits_player() {
        let map = test_map();
        let player = Vec2::new(3.0, 1.5);
        let mut projs = vec![FpProjectile::new(Vec2::new(1.5, 1.5), player, 25).unwrap()];
        let mut total_damage = 0;
        let mut damage_source = None;
        for _ in 0..60 {
            let result = tick_projectiles(&mut projs, player, &map, 0.016);
            total_damage += result.player_damage;
            damage_source = damage_source.or(result.damage_source);
        }
        assert_eq!(total_damage, 25);
        assert_eq!(damage_source, Some(Vec2::new(1.5, 1.5)));
        assert!(projs.is_empty());
    }

    #[test]
    fn projectile_sweeps_player_collision_during_large_step() {
        let map = test_map();
        let player = Vec2::new(3.0, 1.5);
        let mut proj = FpProjectile::new(Vec2::new(1.5, 1.5), Vec2::new(6.5, 1.5), 25).unwrap();
        proj.speed = 100.0;
        let mut projs = vec![proj];

        let result = tick_projectiles(&mut projs, player, &map, 0.1);

        assert_eq!(result.player_damage, 25);
        assert!(projs.is_empty());
        assert!(
            result.impacts[0].position.x < player.x,
            "impact should be at first contact with player radius"
        );
    }

    #[test]
    fn swept_projectile_wall_blocks_player_behind_wall() {
        let map = test_map();
        let player = Vec2::new(0.5, 1.5);
        let mut proj = FpProjectile::new(Vec2::new(1.5, 1.5), player, 25).unwrap();
        proj.speed = 100.0;
        let mut projs = vec![proj];

        let result = tick_projectiles(&mut projs, player, &map, 0.1);

        assert_eq!(result.player_damage, 0);
        assert!(projs.is_empty());
        assert!((result.impacts[0].position.x - 1.0).abs() < 0.01);
    }

    #[test]
    fn projectile_out_of_bounds_splash_uses_map_exit_point() {
        let map = FpMap {
            width: 3,
            height: 3,
            cells: vec![0; 9],
        };
        let mut proj = FpProjectile::new(Vec2::new(1.5, 1.5), Vec2::new(5.5, 1.5), 10).unwrap();
        proj.speed = 100.0;
        let mut projs = vec![proj];

        let result = tick_projectiles(&mut projs, Vec2::new(1.5, 2.5), &map, 0.1);

        assert!(projs.is_empty());
        assert_eq!(result.impacts.len(), 1);
        assert!(
            (result.impacts[0].position.x - 3.0).abs() < 0.01,
            "impact should be at map exit boundary, got {:?}",
            result.impacts[0].position
        );
    }

    #[test]
    fn projectile_lifetime_expires() {
        let map = test_map();
        let mut projs =
            vec![FpProjectile::new(Vec2::new(1.5, 1.5), Vec2::new(2.5, 1.5), 10).unwrap()];
        projs[0].lifetime = 0.01;
        let result = tick_projectiles(&mut projs, Vec2::new(5.0, 5.0), &map, 0.02);
        assert_eq!(result.player_damage, 0);
        assert!(result.impacts.is_empty());
        assert!(projs.is_empty());
    }

    #[test]
    fn tick_projectiles_empty_list() {
        let map = test_map();
        let mut projs = Vec::new();
        let result = tick_projectiles(&mut projs, Vec2::ZERO, &map, 0.016);
        assert_eq!(result.player_damage, 0);
        assert!(result.impacts.is_empty());
    }

    #[test]
    fn hitscan_projectiles_returns_closest_projectile() {
        let map = test_map();
        let cam = FpCamera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            ..Default::default()
        };
        let projs = vec![
            FpProjectile::new(Vec2::new(5.0, 1.5), Vec2::new(1.5, 1.5), 10).unwrap(),
            FpProjectile::new(Vec2::new(3.0, 1.5), Vec2::new(1.5, 1.5), 10).unwrap(),
        ];

        assert_eq!(hitscan_projectiles(&cam, &projs, &map), Some((1, 1.5)));
    }

    #[test]
    fn projectile_impacts_expire() {
        let mut impacts = vec![FpProjectileImpact::hit(Vec2::new(1.0, 1.0))];
        tick_projectile_impacts(&mut impacts, 1.0);
        assert!(impacts.is_empty());
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
