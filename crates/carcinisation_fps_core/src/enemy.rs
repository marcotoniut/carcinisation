//! First-person enemy state and AI.

use bevy::prelude::Component;
use bevy::reflect::Reflect;
use bevy_math::Vec2;

use crate::burning::BurnState;
use crate::camera::Camera;
use crate::fire_death::{DamageKind, corpse_seed};
use crate::map::Map;
use crate::raycast::{cast_ray, has_line_of_sight};

/// Headless FPS enemy kind.
///
/// This lives in `carcinisation_fps_core` so single-player and server code can
/// share enemy rules without depending on networking or rendering crates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FpsEnemyKind {
    Basic,
    Mosquiton,
    Spidey,
}

/// Headless enemy AI state shared by local and server-authoritative sims.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FpsEnemyAiState {
    Idle,
    Chasing,
    Attacking,
    Dead,
}

/// Minimal headless enemy sim state consumed and produced by shared AI rules.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnemySim {
    pub kind: FpsEnemyKind,
    pub position: Vec2,
    pub angle: f32,
    pub health: f32,
    pub state: FpsEnemyAiState,
}

impl EnemySim {
    #[must_use]
    pub fn is_alive(self) -> bool {
        self.health > 0.0 && self.state != FpsEnemyAiState::Dead
    }
}

/// Target data visible to shared enemy AI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnemyPlayerTarget {
    pub position: Vec2,
    pub alive: bool,
    /// Stable ID for deterministic tie-breaking when equidistant.
    pub id: u32,
}

/// Explicit config for shared Mosquiton chase/hold behavior.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct MosquitonAiConfig {
    pub move_speed: f32,
    pub preferred_range: f32,
    /// Distance band around `preferred_range` where the current chase/attack
    /// state is preserved to avoid edge flicker.
    pub preferred_range_hysteresis: f32,
    pub aggro_range: f32,
    pub collision_radius: f32,
}

impl Default for MosquitonAiConfig {
    fn default() -> Self {
        Self {
            move_speed: 1.2,
            preferred_range: 3.0,
            preferred_range_hysteresis: 0.2,
            aggro_range: 8.0,
            collision_radius: 0.3,
        }
    }
}

/// Result summary from a shared enemy AI tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EnemyAiOutput {
    pub target_position: Option<Vec2>,
    pub distance_to_target: Option<f32>,
    pub desired_direction: Option<Vec2>,
    pub attempted_step: Vec2,
    pub disposition: EnemyAiDisposition,
    pub moved: bool,
    pub blocked_by_collision: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EnemyAiDisposition {
    #[default]
    None,
    Dead,
    UnsupportedKind,
    NoAlivePlayers,
    OutsideAggroRange,
    HoldingPreferredRange,
    Chasing,
    StalledAtPreferredRange,
    BlockedByCollision,
}

/// Tick shared headless enemy AI.
///
/// Initial Mosquiton behavior is intentionally small and portable:
/// nearest alive target inside aggro range is faced; Mosquitons move toward
/// that target until preferred range, then hold in an attacking-ready state.
/// Dead enemies never move.
pub fn tick_enemy_ai(
    enemy: &mut EnemySim,
    players: &[EnemyPlayerTarget],
    map: &Map,
    dt: f32,
    config: MosquitonAiConfig,
) -> EnemyAiOutput {
    if !enemy.is_alive() {
        enemy.state = FpsEnemyAiState::Dead;
        return EnemyAiOutput {
            disposition: EnemyAiDisposition::Dead,
            ..Default::default()
        };
    }

    match enemy.kind {
        // Spidey uses its own dedicated sim (`tick_spidey_sim`), not this
        // shared AI dispatcher. Mark as unsupported here for safety.
        FpsEnemyKind::Basic | FpsEnemyKind::Spidey => tick_basic_enemy_ai(enemy),
        FpsEnemyKind::Mosquiton => tick_mosquiton_ai(enemy, players, map, dt, config),
    }
}

fn tick_basic_enemy_ai(enemy: &mut EnemySim) -> EnemyAiOutput {
    // Basic enemies intentionally have no shared headless behavior yet.
    // Keeping this no-op prevents callers from accidentally getting Mosquiton AI.
    enemy.state = FpsEnemyAiState::Idle;
    EnemyAiOutput {
        disposition: EnemyAiDisposition::UnsupportedKind,
        ..Default::default()
    }
}

fn tick_mosquiton_ai(
    enemy: &mut EnemySim,
    players: &[EnemyPlayerTarget],
    map: &Map,
    dt: f32,
    config: MosquitonAiConfig,
) -> EnemyAiOutput {
    let Some((target, distance)) = nearest_alive_target(enemy.position, players) else {
        enemy.state = FpsEnemyAiState::Idle;
        return EnemyAiOutput {
            disposition: EnemyAiDisposition::NoAlivePlayers,
            ..Default::default()
        };
    };

    if distance > config.aggro_range {
        enemy.state = FpsEnemyAiState::Idle;
        return EnemyAiOutput {
            target_position: Some(target.position),
            distance_to_target: Some(distance),
            disposition: EnemyAiDisposition::OutsideAggroRange,
            moved: false,
            desired_direction: None,
            attempted_step: Vec2::ZERO,
            blocked_by_collision: false,
        };
    }

    face_target(enemy, target.position);

    let hysteresis = config
        .preferred_range_hysteresis
        .clamp(0.0, config.preferred_range.max(0.0));
    let chase_distance = config.preferred_range + hysteresis;
    let should_chase = if distance > chase_distance {
        true
    } else if distance <= config.preferred_range {
        false
    } else {
        matches!(enemy.state, FpsEnemyAiState::Chasing)
    };

    if !should_chase {
        enemy.state = FpsEnemyAiState::Attacking;
        return EnemyAiOutput {
            target_position: Some(target.position),
            distance_to_target: Some(distance),
            desired_direction: Some((target.position - enemy.position).normalize_or_zero()),
            attempted_step: Vec2::ZERO,
            disposition: EnemyAiDisposition::HoldingPreferredRange,
            moved: false,
            blocked_by_collision: false,
        };
    }

    enemy.state = FpsEnemyAiState::Chasing;
    let before = enemy.position;
    let dir = target.position - enemy.position;
    let len = dir.length();
    let mut attempted_step = Vec2::ZERO;
    let desired_direction = (len > f32::EPSILON).then_some(dir / len);
    if len > f32::EPSILON {
        let max_step = (distance - config.preferred_range).max(0.0);
        let step_len = (config.move_speed * dt).min(max_step);
        if step_len <= f32::EPSILON {
            enemy.state = FpsEnemyAiState::Attacking;
            return EnemyAiOutput {
                target_position: Some(target.position),
                distance_to_target: Some(distance),
                desired_direction,
                attempted_step,
                disposition: EnemyAiDisposition::HoldingPreferredRange,
                moved: false,
                blocked_by_collision: false,
            };
        }
        attempted_step = dir / len * step_len;
        crate::collision::try_move(
            &mut enemy.position,
            attempted_step,
            config.collision_radius,
            map,
        );
    }
    let moved = enemy.position.distance_squared(before) > 0.000_001;
    let stalled_at_preferred_range =
        !moved && attempted_step.length_squared() <= f32::EPSILON && distance <= chase_distance;
    let blocked_by_collision =
        !moved && distance > config.preferred_range && !stalled_at_preferred_range;

    EnemyAiOutput {
        target_position: Some(target.position),
        distance_to_target: Some(distance),
        desired_direction,
        attempted_step,
        disposition: if blocked_by_collision {
            EnemyAiDisposition::BlockedByCollision
        } else if stalled_at_preferred_range {
            EnemyAiDisposition::StalledAtPreferredRange
        } else {
            EnemyAiDisposition::Chasing
        },
        moved,
        blocked_by_collision,
    }
}

fn nearest_alive_target(
    position: Vec2,
    players: &[EnemyPlayerTarget],
) -> Option<(EnemyPlayerTarget, f32)> {
    players
        .iter()
        .copied()
        .filter(|target| target.alive)
        .map(|target| (target, target.position.distance(position)))
        .min_by(|(ta, a), (tb, b)| {
            a.partial_cmp(b)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(ta.id.cmp(&tb.id))
        })
}

fn face_target(enemy: &mut EnemySim, target_position: Vec2) {
    let to_target = target_position - enemy.position;
    if to_target.length_squared() > f32::EPSILON {
        enemy.angle = to_target
            .y
            .atan2(to_target.x)
            .rem_euclid(std::f32::consts::TAU);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DamageFlickerPhase {
    Regular,
    Invert,
}

#[derive(Clone, Copy, Debug, PartialEq, Component)]
pub struct DamageFlicker {
    phase: DamageFlickerPhase,
    phase_remaining_secs: f32,
    remaining_invert_cycles: u8,
    /// Duration of the regular (non-inverted) phase.
    regular_secs: f32,
    /// Duration of the inverted phase.
    invert_secs: f32,
}

impl DamageFlicker {
    /// Create a new flicker with values from `FpsVisualConfig`.
    #[must_use]
    pub fn from_config(config: &crate::config::FpsVisualConfig) -> Self {
        Self {
            phase: DamageFlickerPhase::Regular,
            phase_remaining_secs: config.damage_flicker_regular_secs,
            remaining_invert_cycles: config.damage_flicker_count,
            regular_secs: config.damage_flicker_regular_secs,
            invert_secs: config.damage_flicker_invert_secs,
        }
    }

    /// Create a new flicker using `FpsVisualConfig` defaults.
    ///
    /// Prefer [`from_config`](Self::from_config) when a `FpsVisualConfig` is available.
    #[must_use]
    pub fn new() -> Self {
        Self::from_config(&crate::config::FpsVisualConfig::default())
    }

    #[must_use]
    pub fn showing_invert(self) -> bool {
        self.phase == DamageFlickerPhase::Invert
    }

    /// Advance the flicker using the timing values stored at construction.
    #[must_use]
    pub fn tick(mut self, dt: f32) -> Option<Self> {
        self.phase_remaining_secs -= dt;
        while self.phase_remaining_secs <= 0.0 {
            match self.phase {
                DamageFlickerPhase::Regular => {
                    self.phase = DamageFlickerPhase::Invert;
                    self.phase_remaining_secs += self.invert_secs;
                }
                DamageFlickerPhase::Invert => {
                    if self.remaining_invert_cycles == 0 {
                        return None;
                    }
                    self.remaining_invert_cycles -= 1;
                    self.phase = DamageFlickerPhase::Regular;
                    self.phase_remaining_secs += self.regular_secs;
                }
            }
        }
        Some(self)
    }
}

impl Default for DamageFlicker {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Shared damage helpers
// ---------------------------------------------------------------------------

/// Outcome of [`apply_damage`] — tells the caller whether the entity survived
/// or which death presentation to transition into.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DamageOutcome {
    /// Entity survived — health is still positive.
    Survived,
    /// Lethal hit from a physical source.
    KilledPhysical,
    /// Lethal hit from fire.  Caller should start a burning-corpse presentation
    /// using the provided `timer` and `seed`.
    KilledByFire { timer: f32, seed: u32 },
}

/// Shared damage-application logic used by `Enemy`, `Mosquiton`, and `Spidey`.
///
/// Subtracts `amount` from `*health`, updates `*damage_flicker`, and returns a
/// [`DamageOutcome`] so the caller can apply the type-specific death state.
///
/// `position` is needed to derive the deterministic `corpse_seed` on fire kills.
pub fn apply_damage(
    health: &mut u32,
    damage_flicker: &mut Option<DamageFlicker>,
    amount: u32,
    kind: DamageKind,
    fire_death_secs: f32,
    position: Vec2,
) -> DamageOutcome {
    *health = health.saturating_sub(amount);
    if *health == 0 {
        *damage_flicker = None;
        match kind {
            DamageKind::Physical => DamageOutcome::KilledPhysical,
            DamageKind::Fire => DamageOutcome::KilledByFire {
                timer: fire_death_secs.max(0.0),
                seed: corpse_seed(position),
            },
        }
    } else {
        if damage_flicker.is_none() {
            *damage_flicker = Some(DamageFlicker::new());
        }
        DamageOutcome::Survived
    }
}

/// Whether a damage flicker is currently in the inverted (white-flash) phase.
///
/// Returns `false` when there is no active flicker.
#[must_use]
pub fn is_showing_damage_invert(damage_flicker: &Option<DamageFlicker>) -> bool {
    damage_flicker.is_some_and(DamageFlicker::showing_invert)
}

/// Enemy AI/lifecycle state.
#[derive(Clone, Debug, Component)]
pub enum EnemyState {
    /// Stationary, not yet aware of the player.
    Idle,
    /// Moving toward the player.
    Chasing,
    /// In melee range, waiting for cooldown.
    Attacking { cooldown: f32 },
    /// Playing death animation.
    Dying { timer: f32 },
    /// Inert fire-death presentation before despawn.
    BurningCorpse { timer: f32, seed: u32 },
    /// Fully dead — pending removal or inert.
    Dead,
}

/// A runtime enemy instance.
#[derive(Clone, Debug, Component)]
pub struct Enemy {
    pub position: Vec2,
    pub health: u32,
    pub max_health: u32,
    pub speed: f32,
    pub state: EnemyState,
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
    pub damage_flicker: Option<DamageFlicker>,
    pub burn_state: BurnState,
}

impl Enemy {
    /// Create a new enemy from spawn data.
    #[must_use]
    pub fn new(position: Vec2, health: u32, speed: f32) -> Self {
        Self {
            position,
            health,
            max_health: health,
            speed,
            state: EnemyState::Idle,
            radius: 0.3,
            detect_range: 8.0,
            attack_range: 0.8,
            attack_damage: 10,
            attack_interval: 1.0,
            damage_flicker: None,
            burn_state: BurnState::default(),
        }
    }

    /// Whether this enemy is alive (can be hit and acts).
    #[must_use]
    pub fn is_alive(&self) -> bool {
        !matches!(
            self.state,
            EnemyState::Dying { .. } | EnemyState::BurningCorpse { .. } | EnemyState::Dead
        )
    }

    /// Apply damage. Transitions to Dying if health reaches zero.
    pub fn take_damage(&mut self, amount: u32) {
        self.take_damage_from(amount, DamageKind::Physical, 0.5);
    }

    /// Apply damage with source-specific death presentation.
    pub fn take_damage_from(&mut self, amount: u32, kind: DamageKind, fire_death_secs: f32) {
        if !self.is_alive() {
            return;
        }
        match apply_damage(
            &mut self.health,
            &mut self.damage_flicker,
            amount,
            kind,
            fire_death_secs,
            self.position,
        ) {
            DamageOutcome::Survived => {}
            DamageOutcome::KilledPhysical => {
                self.state = EnemyState::Dying { timer: 0.5 };
            }
            DamageOutcome::KilledByFire { timer, seed } => {
                self.state = EnemyState::BurningCorpse { timer, seed };
            }
        }
    }

    #[must_use]
    pub fn showing_damage_invert(&self) -> bool {
        self.is_alive() && is_showing_damage_invert(&self.damage_flicker)
    }
}

/// Tick a single enemy for one frame. Returns a newly spawned projectile if any.
#[must_use]
pub fn tick_single_enemy(
    enemy: &mut Enemy,
    player_pos: Vec2,
    map: &Map,
    dt: f32,
) -> Option<Projectile> {
    if let Some(flicker) = enemy.damage_flicker {
        enemy.damage_flicker = flicker.tick(dt);
    }

    match &mut enemy.state {
        EnemyState::Dead => {}

        EnemyState::Dying { timer } | EnemyState::BurningCorpse { timer, .. } => {
            *timer -= dt;
            if *timer <= 0.0 {
                enemy.state = EnemyState::Dead;
            }
        }

        EnemyState::Idle => {
            let dist = enemy.position.distance(player_pos);
            if dist < enemy.detect_range && has_line_of_sight(enemy.position, player_pos, map) {
                enemy.state = EnemyState::Chasing;
            }
        }

        EnemyState::Chasing => {
            let to_player = player_pos - enemy.position;
            let dist = to_player.length();

            if dist < enemy.attack_range {
                enemy.state = EnemyState::Attacking {
                    cooldown: enemy.attack_interval,
                };
            } else if dist > 0.01 {
                let move_dir = to_player / dist;
                let step = move_dir * enemy.speed * dt;
                try_move_enemy(enemy, step, map);
            }
        }

        EnemyState::Attacking { cooldown } => {
            let dist = enemy.position.distance(player_pos);

            // Player moved out of range or behind a wall — chase again.
            if dist > enemy.attack_range * 1.5
                || !has_line_of_sight(enemy.position, player_pos, map)
            {
                enemy.state = EnemyState::Chasing;
                return None;
            }

            *cooldown -= dt;
            if *cooldown <= 0.0 {
                let proj = Projectile::new(enemy.position, player_pos, enemy.attack_damage);
                *cooldown = enemy.attack_interval;
                return proj;
            }
        }
    }

    None
}

/// Update all enemies for one frame. Returns newly spawned projectiles.
#[must_use]
pub fn tick_enemies(
    enemies: &mut [Enemy],
    player_pos: Vec2,
    map: &Map,
    dt: f32,
) -> Vec<Projectile> {
    enemies
        .iter_mut()
        .filter_map(|e| tick_single_enemy(e, player_pos, map, dt))
        .collect()
}

/// Attempt to move an enemy by `delta`, checking wall collision.
fn try_move_enemy(enemy: &mut Enemy, delta: Vec2, map: &Map) {
    crate::collision::try_move(&mut enemy.position, delta, enemy.radius, map);
}

/// Result of a hitscan shot.
#[derive(Debug)]
pub struct HitscanResult {
    /// Index of the enemy hit, if any.
    pub enemy_idx: Option<usize>,
    /// Distance to the hit point.
    pub distance: f32,
}

/// Generic hitscan ray-circle intersection against an iterator of targets.
///
/// Each item yielded by `targets` is `(position, radius, alive)`. Only alive
/// targets in front of the camera and closer than the nearest wall are
/// considered. Returns the index and distance of the closest hit, if any.
#[must_use]
pub fn hitscan_generic(
    camera: &Camera,
    map: &Map,
    targets: impl Iterator<Item = (Vec2, f32, bool)>,
) -> Option<(usize, f32)> {
    let dir = camera.direction();
    let origin = camera.position;
    let wall_hit = cast_ray(map, origin, dir);
    let max_dist = wall_hit.distance;

    let mut closest: Option<(usize, f32)> = None;

    for (i, (position, radius, alive)) in targets.enumerate() {
        if !alive {
            continue;
        }

        let to_target = position - origin;
        let along_ray = to_target.dot(dir);

        if along_ray <= 0.0 || along_ray > max_dist {
            continue;
        }

        let perp_dist_sq = to_target.length_squared() - along_ray * along_ray;
        let radius_sq = radius * radius;

        if perp_dist_sq < radius_sq && closest.is_none_or(|(_, d)| along_ray < d) {
            closest = Some((i, along_ray));
        }
    }

    closest
}

/// Fire a hitscan ray from the camera and check for enemy hits.
///
/// Tests each alive enemy for ray-circle intersection, returns the closest
/// hit that is nearer than the first wall.
#[must_use]
pub fn hitscan(camera: &Camera, enemies: &[Enemy], map: &Map) -> HitscanResult {
    let max_dist = cast_ray(map, camera.position, camera.direction()).distance;

    match hitscan_generic(
        camera,
        map,
        enemies.iter().map(|e| (e.position, e.radius, e.is_alive())),
    ) {
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

/// Projectile behaviour variant.
///
/// `BloodShot` is the default (pure damage). `WebShot` applies a temporary
/// speed slow on player hit in addition to damage.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ProjectileKind {
    /// Standard damage-only projectile (Mosquiton blood shot, etc.).
    #[default]
    BloodShot,
    /// Web projectile that applies a movement slow on hit.
    WebShot {
        slow_multiplier: f32,
        slow_duration: f32,
    },
}

/// An enemy projectile moving through map space.
#[derive(Clone, Debug)]
pub struct Projectile {
    pub position: Vec2,
    pub source_position: Vec2,
    pub direction: Vec2,
    pub speed: f32,
    pub radius: f32,
    pub damage: u32,
    pub lifetime: f32,
    /// Initial lifetime at spawn (for arc calculations).
    pub initial_lifetime: f32,
    pub alive: bool,
    pub kind: ProjectileKind,
}

/// Default enemy projectile speed (map units/s).
pub const DEFAULT_PROJECTILE_SPEED: f32 = 4.0;
/// Default enemy projectile collision radius.
pub const DEFAULT_PROJECTILE_HIT_RADIUS: f32 = 0.3;
/// Default enemy projectile lifetime (seconds).
pub const DEFAULT_PROJECTILE_LIFETIME: f32 = 3.0;

impl Projectile {
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
            speed: DEFAULT_PROJECTILE_SPEED,
            radius: DEFAULT_PROJECTILE_HIT_RADIUS,
            damage,
            lifetime: DEFAULT_PROJECTILE_LIFETIME,
            initial_lifetime: DEFAULT_PROJECTILE_LIFETIME,
            alive: true,
            kind: ProjectileKind::BloodShot,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectileImpactKind {
    Hit,
    Destroy,
}

#[derive(Clone, Debug)]
pub struct ProjectileImpact {
    pub position: Vec2,
    pub kind: ProjectileImpactKind,
    pub source_kind: ProjectileKind,
    /// Billboard height at impact (rendering hint from the projectile's arc).
    pub visual_height: f32,
    pub age: f32,
    pub lifetime: f32,
}

impl ProjectileImpact {
    #[must_use]
    pub fn hit(position: Vec2, source_kind: ProjectileKind, visual_height: f32) -> Self {
        Self {
            position,
            kind: ProjectileImpactKind::Hit,
            source_kind,
            visual_height,
            age: 0.0,
            lifetime: 0.18,
        }
    }

    #[must_use]
    pub fn destroy(position: Vec2, source_kind: ProjectileKind, visual_height: f32) -> Self {
        Self {
            position,
            kind: ProjectileImpactKind::Destroy,
            source_kind,
            visual_height,
            age: 0.0,
            lifetime: 0.3,
        }
    }
}

/// Slow effect to apply after a `WebShot` player hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileSlowEffect {
    pub multiplier: f32,
    pub duration: f32,
}

#[derive(Clone, Debug, Default)]
pub struct ProjectileTickResult {
    pub player_damage: u32,
    pub damage_source: Option<Vec2>,
    pub impacts: Vec<ProjectileImpact>,
    /// If a `WebShot` hit the player this tick, the slow to apply.
    pub slow_effect: Option<ProjectileSlowEffect>,
}

/// Tick all projectile impact effects and remove finished ones.
pub fn tick_projectile_impacts(impacts: &mut Vec<ProjectileImpact>, dt: f32) {
    for impact in impacts.iter_mut() {
        impact.age += dt;
    }
    impacts.retain(|impact| impact.age < impact.lifetime);
}

/// Tick all projectiles. Returns damage and impact events this frame.
/// Projectiles that hit the player or a wall are marked `alive = false`.
#[must_use]
pub fn tick_projectiles(
    projectiles: &mut Vec<Projectile>,
    player_pos: Vec2,
    map: &Map,
    dt: f32,
) -> ProjectileTickResult {
    let mut result = ProjectileTickResult::default();

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
            Some(ProjectileCollision::Wall(distance)) => {
                proj.alive = false;
                result.impacts.push(ProjectileImpact::hit(
                    previous_position + proj.direction * distance,
                    proj.kind,
                    0.0,
                ));
            }
            Some(ProjectileCollision::Player(distance)) => {
                proj.alive = false;
                result.player_damage += proj.damage;
                result.damage_source = Some(proj.source_position);
                result.impacts.push(ProjectileImpact::hit(
                    previous_position + proj.direction * distance,
                    proj.kind,
                    0.0,
                ));
                // WebShot: emit slow effect on player hit.
                if let ProjectileKind::WebShot {
                    slow_multiplier,
                    slow_duration,
                } = proj.kind
                {
                    result.slow_effect = Some(ProjectileSlowEffect {
                        multiplier: slow_multiplier,
                        duration: slow_duration,
                    });
                }
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
            result.impacts.push(ProjectileImpact::hit(
                segment_map_exit_point(previous_position, proj.position, map),
                proj.kind,
                0.0,
            ));
        }
    }

    // Remove dead projectiles.
    projectiles.retain(|p| p.alive);

    result
}

enum ProjectileCollision {
    Wall(f32),
    Player(f32),
}

fn earliest_projectile_collision(
    wall_distance: Option<f32>,
    player_distance: Option<f32>,
) -> Option<ProjectileCollision> {
    match (wall_distance, player_distance) {
        (Some(wall), Some(player)) if player < wall => Some(ProjectileCollision::Player(player)),
        (Some(wall), _) => Some(ProjectileCollision::Wall(wall)),
        (None, Some(player)) => Some(ProjectileCollision::Player(player)),
        (None, None) => None,
    }
}

#[must_use]
pub fn segment_circle_hit_distance(
    start: Vec2,
    end: Vec2,
    center: Vec2,
    radius: f32,
) -> Option<f32> {
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

fn segment_map_exit_point(start: Vec2, end: Vec2, map: &Map) -> Vec2 {
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
    camera: &Camera,
    projectiles: &[Projectile],
    map: &Map,
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

    fn make_enemy(x: f32, y: f32) -> Enemy {
        Enemy::new(Vec2::new(x, y), 30, 1.5)
    }

    fn make_mosquiton_sim(x: f32, y: f32) -> EnemySim {
        EnemySim {
            kind: FpsEnemyKind::Mosquiton,
            position: Vec2::new(x, y),
            angle: 0.0,
            health: 100.0,
            state: FpsEnemyAiState::Idle,
        }
    }

    fn target_at(x: f32, y: f32) -> EnemyPlayerTarget {
        EnemyPlayerTarget {
            position: Vec2::new(x, y),
            alive: true,
            id: 1,
        }
    }

    // --- shared enemy AI ---

    #[test]
    fn mosquiton_ai_moves_toward_player_when_far() {
        let map = test_map();
        let mut enemy = make_mosquiton_sim(1.5, 1.5);
        let config = MosquitonAiConfig {
            move_speed: 1.0,
            preferred_range: 1.0,
            collision_radius: 0.2,
            ..Default::default()
        };

        let output = tick_enemy_ai(&mut enemy, &[target_at(5.5, 1.5)], &map, 1.0, config);

        assert!(output.moved);
        assert!(enemy.position.x > 1.5);
        assert_eq!(enemy.state, FpsEnemyAiState::Chasing);
        assert!(enemy.angle.abs() < 0.001);
    }

    #[test]
    fn mosquiton_ai_holds_near_preferred_range() {
        let map = test_map();
        let mut enemy = make_mosquiton_sim(1.5, 1.5);
        let config = MosquitonAiConfig {
            move_speed: 1.0,
            preferred_range: 3.0,
            collision_radius: 0.2,
            ..Default::default()
        };

        let output = tick_enemy_ai(&mut enemy, &[target_at(4.0, 1.5)], &map, 1.0, config);

        assert!(!output.moved);
        assert_eq!(enemy.position, Vec2::new(1.5, 1.5));
        assert_eq!(enemy.state, FpsEnemyAiState::Attacking);
    }

    #[test]
    fn mosquiton_ai_does_not_move_through_walls() {
        let map = test_map();
        let mut enemy = make_mosquiton_sim(2.5, 2.5);
        let config = MosquitonAiConfig {
            move_speed: 1.0,
            preferred_range: 0.5,
            collision_radius: 0.3,
            ..Default::default()
        };

        let output = tick_enemy_ai(&mut enemy, &[target_at(4.5, 2.5)], &map, 1.0, config);

        assert!(!output.moved);
        assert!(enemy.position.x < 3.0);
        assert_eq!(enemy.state, FpsEnemyAiState::Chasing);
    }

    #[test]
    fn mosquiton_ai_slides_when_only_one_axis_is_blocked() {
        #[rustfmt::skip]
        let map = Map {
            width: 6,
            height: 6,
            cells: vec![
                1, 1, 1, 1, 1, 1,
                1, 0, 0, 0, 0, 1,
                1, 0, 0, 1, 0, 1,
                1, 0, 0, 0, 0, 1,
                1, 0, 0, 0, 0, 1,
                1, 1, 1, 1, 1, 1,
            ],
        };
        let mut enemy = make_mosquiton_sim(2.7, 2.5);
        let config = MosquitonAiConfig {
            move_speed: 1.0,
            preferred_range: 0.5,
            collision_radius: 0.3,
            ..Default::default()
        };

        let output = tick_enemy_ai(&mut enemy, &[target_at(4.5, 4.5)], &map, 1.0, config);

        assert!(output.moved);
        assert!(enemy.position.x < 3.0, "x should be blocked by wall");
        assert!(enemy.position.y > 2.5, "y should slide along wall");
        assert_eq!(output.disposition, EnemyAiDisposition::Chasing);
    }

    #[test]
    fn dead_mosquiton_ai_does_not_move() {
        let map = test_map();
        let mut enemy = make_mosquiton_sim(1.5, 1.5);
        enemy.health = 0.0;
        enemy.state = FpsEnemyAiState::Dead;

        let output = tick_enemy_ai(
            &mut enemy,
            &[target_at(5.5, 1.5)],
            &map,
            1.0,
            MosquitonAiConfig::default(),
        );

        assert!(!output.moved);
        assert_eq!(enemy.position, Vec2::new(1.5, 1.5));
        assert_eq!(enemy.state, FpsEnemyAiState::Dead);
    }

    #[test]
    fn mosquiton_ai_preserves_state_inside_preferred_range_hysteresis() {
        let map = test_map();
        let config = MosquitonAiConfig {
            move_speed: 1.0,
            preferred_range: 3.0,
            preferred_range_hysteresis: 0.25,
            collision_radius: 0.2,
            ..Default::default()
        };

        let mut chasing = make_mosquiton_sim(1.5, 1.5);
        chasing.state = FpsEnemyAiState::Chasing;
        let chasing_output = tick_enemy_ai(&mut chasing, &[target_at(4.6, 1.5)], &map, 0.1, config);
        assert_eq!(chasing.state, FpsEnemyAiState::Chasing);
        assert!(chasing_output.moved);

        let mut attacking = make_mosquiton_sim(1.5, 1.5);
        attacking.state = FpsEnemyAiState::Attacking;
        let attacking_output =
            tick_enemy_ai(&mut attacking, &[target_at(4.6, 1.5)], &map, 0.1, config);
        assert_eq!(attacking.state, FpsEnemyAiState::Attacking);
        assert!(!attacking_output.moved);
    }

    #[test]
    fn mosquiton_ai_holds_when_chasing_reaches_preferred_range() {
        let map = test_map();
        let config = MosquitonAiConfig {
            move_speed: 10.0,
            preferred_range: 3.0,
            preferred_range_hysteresis: 0.25,
            collision_radius: 0.2,
            ..Default::default()
        };
        let mut enemy = make_mosquiton_sim(1.5, 1.5);
        enemy.state = FpsEnemyAiState::Chasing;

        let output = tick_enemy_ai(&mut enemy, &[target_at(4.5, 1.5)], &map, 1.0, config);

        assert!(!output.moved);
        assert_eq!(enemy.state, FpsEnemyAiState::Attacking);
        assert_eq!(
            output.disposition,
            EnemyAiDisposition::HoldingPreferredRange
        );
    }

    #[test]
    fn mosquiton_ai_does_not_zero_step_loop_while_chasing() {
        let map = test_map();
        let config = MosquitonAiConfig {
            move_speed: 1.0,
            preferred_range: 3.0,
            preferred_range_hysteresis: 0.25,
            collision_radius: 0.2,
            ..Default::default()
        };
        let mut enemy = make_mosquiton_sim(1.5, 1.5);
        enemy.state = FpsEnemyAiState::Chasing;

        let output = tick_enemy_ai(&mut enemy, &[target_at(4.6, 1.5)], &map, 0.0, config);

        assert!(!output.moved);
        assert_eq!(enemy.state, FpsEnemyAiState::Attacking);
        assert_ne!(
            output.disposition,
            EnemyAiDisposition::StalledAtPreferredRange
        );
    }

    #[test]
    fn basic_enemy_ai_is_explicit_noop() {
        let map = test_map();
        let mut enemy = EnemySim {
            kind: FpsEnemyKind::Basic,
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            health: 100.0,
            state: FpsEnemyAiState::Chasing,
        };

        let output = tick_enemy_ai(
            &mut enemy,
            &[target_at(5.5, 1.5)],
            &map,
            1.0,
            MosquitonAiConfig::default(),
        );

        assert!(!output.moved);
        assert_eq!(enemy.position, Vec2::new(1.5, 1.5));
        assert_eq!(enemy.state, FpsEnemyAiState::Idle);
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
        let map = test_map();
        let mut enemies = vec![make_enemy(3.0, 1.5)];
        // Player close and in LOS.
        let _ = tick_enemies(&mut enemies, Vec2::new(1.5, 1.5), &map, 0.016);
        assert!(matches!(enemies[0].state, EnemyState::Chasing));
    }

    #[test]
    fn chasing_enemy_enters_attack_range() {
        let map = test_map();
        let mut enemies = vec![make_enemy(2.0, 1.5)];
        enemies[0].state = EnemyState::Chasing;
        // Player within attack range.
        let _ = tick_enemies(&mut enemies, Vec2::new(1.5, 1.5), &map, 0.016);
        assert!(matches!(enemies[0].state, EnemyState::Attacking { .. }));
    }

    #[test]
    fn attacking_enemy_spawns_projectile() {
        let map = test_map();
        let mut enemies = vec![make_enemy(1.5, 1.5)];
        enemies[0].state = EnemyState::Attacking { cooldown: 0.01 };
        // Player nearby but not at same position (avoids zero-direction filter).
        let projectiles = tick_enemies(&mut enemies, Vec2::new(2.0, 1.5), &map, 0.02);
        assert!(!projectiles.is_empty());
        assert!(projectiles[0].damage > 0);
    }

    #[test]
    fn dying_enemy_transitions_to_dead() {
        let map = test_map();
        let mut enemies = vec![make_enemy(4.0, 4.0)];
        enemies[0].state = EnemyState::Dying { timer: 0.1 };
        let _ = tick_enemies(&mut enemies, Vec2::new(1.5, 1.5), &map, 0.2);
        assert!(matches!(enemies[0].state, EnemyState::Dead));
    }

    #[test]
    fn burning_corpse_transitions_to_dead_without_attacking() {
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
        let map = test_map();
        let cam = Camera {
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
        let cam = Camera {
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
        let cam = Camera {
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
        assert!(Projectile::new(origin, origin, 10).is_none());
    }

    #[test]
    fn projectile_hits_wall() {
        let map = test_map();
        // Aimed west, will hit the border wall at x=0.
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
    fn projectile_sweeps_wall_collision_during_large_step() {
        let map = test_map();
        let mut proj = Projectile::new(Vec2::new(1.5, 1.5), Vec2::new(0.5, 1.5), 10).unwrap();
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
            vec![Projectile::new(Vec2::new(1.5, 1.5), Vec2::new(0.5, 1.5), 10).unwrap()];
        let mut impacts = Vec::new();
        for _ in 0..60 {
            let result = tick_projectiles(&mut projs, Vec2::new(5.0, 5.0), &map, 0.016);
            impacts.extend(result.impacts);
        }
        assert!(
            impacts
                .iter()
                .any(|impact| impact.kind == ProjectileImpactKind::Hit)
        );
    }

    #[test]
    fn projectile_hits_player() {
        let map = test_map();
        let player = Vec2::new(3.0, 1.5);
        let mut projs = vec![Projectile::new(Vec2::new(1.5, 1.5), player, 25).unwrap()];
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
        let mut proj = Projectile::new(Vec2::new(1.5, 1.5), Vec2::new(6.5, 1.5), 25).unwrap();
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
        let mut proj = Projectile::new(Vec2::new(1.5, 1.5), player, 25).unwrap();
        proj.speed = 100.0;
        let mut projs = vec![proj];

        let result = tick_projectiles(&mut projs, player, &map, 0.1);

        assert_eq!(result.player_damage, 0);
        assert!(projs.is_empty());
        assert!((result.impacts[0].position.x - 1.0).abs() < 0.01);
    }

    #[test]
    fn projectile_out_of_bounds_splash_uses_map_exit_point() {
        let map = Map {
            width: 3,
            height: 3,
            cells: vec![0; 9],
        };
        let mut proj = Projectile::new(Vec2::new(1.5, 1.5), Vec2::new(5.5, 1.5), 10).unwrap();
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
            vec![Projectile::new(Vec2::new(1.5, 1.5), Vec2::new(2.5, 1.5), 10).unwrap()];
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
        let cam = Camera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            ..Default::default()
        };
        let projs = vec![
            Projectile::new(Vec2::new(5.0, 1.5), Vec2::new(1.5, 1.5), 10).unwrap(),
            Projectile::new(Vec2::new(3.0, 1.5), Vec2::new(1.5, 1.5), 10).unwrap(),
        ];

        assert_eq!(hitscan_projectiles(&cam, &projs, &map), Some((1, 1.5)));
    }

    #[test]
    fn projectile_impacts_expire() {
        let mut impacts = vec![ProjectileImpact::hit(
            Vec2::new(1.0, 1.0),
            ProjectileKind::BloodShot,
            0.0,
        )];
        tick_projectile_impacts(&mut impacts, 1.0);
        assert!(impacts.is_empty());
    }

    // --- hitscan ---

    #[test]
    fn hitscan_picks_closest_enemy() {
        let map = test_map();
        let cam = Camera {
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

    // --- ProjectileKind: WebShot vs BloodShot ---

    #[test]
    fn bloodshot_does_not_apply_slow() {
        let map = test_map();
        let player = Vec2::new(3.0, 1.5);
        let mut proj = Projectile::new(Vec2::new(1.5, 1.5), player, 10).unwrap();
        proj.speed = 10.0;
        assert!(matches!(proj.kind, ProjectileKind::BloodShot));
        let mut projs = vec![proj];

        let result = tick_projectiles(&mut projs, player, &map, 0.5);

        assert!(result.player_damage > 0, "BloodShot should deal damage");
        assert!(
            result.slow_effect.is_none(),
            "BloodShot should NOT apply slow"
        );
    }

    #[test]
    fn webshot_applies_damage_and_slow() {
        let map = test_map();
        let player = Vec2::new(3.0, 1.5);
        let mut proj = Projectile::new(Vec2::new(1.5, 1.5), player, 5).unwrap();
        proj.speed = 10.0;
        proj.kind = ProjectileKind::WebShot {
            slow_multiplier: 0.7,
            slow_duration: 3.0,
        };
        let mut projs = vec![proj];

        let result = tick_projectiles(&mut projs, player, &map, 0.5);

        assert_eq!(result.player_damage, 5, "WebShot should deal damage");
        let slow = result.slow_effect.expect("WebShot should apply slow");
        assert!((slow.multiplier - 0.7).abs() < f32::EPSILON);
        assert!((slow.duration - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn webshot_wall_hit_does_not_apply_slow() {
        let map = test_map();
        // Aim into a wall, player elsewhere.
        let mut proj = Projectile::new(Vec2::new(1.5, 1.5), Vec2::new(0.5, 1.5), 5).unwrap();
        proj.speed = 10.0;
        proj.kind = ProjectileKind::WebShot {
            slow_multiplier: 0.7,
            slow_duration: 3.0,
        };
        let mut projs = vec![proj];

        let result = tick_projectiles(&mut projs, Vec2::new(5.0, 5.0), &map, 0.5);

        assert_eq!(result.player_damage, 0);
        assert!(
            result.slow_effect.is_none(),
            "wall-hit WebShot should not apply slow"
        );
    }
}
