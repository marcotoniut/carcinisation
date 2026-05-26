//! Pure Spidey combat simulation — hop movement, web ranged attack, lunge melee.
//!
//! No rendering, no Bevy ECS. The fps crate wraps this with animation/billboard
//! concerns; the server can adopt the same sim path for parity.

use bevy_math::Vec2;

use crate::collision::try_move;
use crate::enemy::Projectile;
use crate::map::Map;
use crate::raycast::has_line_of_sight;

/// Maximum visual height to prevent billboard exceeding the ceiling.
///
/// Derived from `wall_height` (1.0) - `billboard_height` (0.45) = 0.55.
/// Also used by the MP presentation adapter to normalize hop phase from
/// replicated `visual_height`.
pub const MAX_VISUAL_HEIGHT: f32 = 0.55;

/// Extra reach beyond `collision_radius` for lunge melee hit detection.
pub const LUNGE_MELEE_HIT_MARGIN: f32 = 0.3;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Pure simulation state for a Spidey.
///
/// Derives `Component` so the fps crate can use it directly as an ECS
/// component without a wrapper type or conversion boilerplate.
#[derive(Clone, Debug, PartialEq, bevy::prelude::Component)]
pub enum SpideySimState {
    /// Stationary, no target acquired.
    Idle,
    /// Between hops, waiting for next hop timer.
    HopWait { timer: f32 },
    /// Executing a hop toward target.
    HopMove {
        /// Normalized direction of this hop.
        direction: Vec2,
        /// Time remaining in this hop.
        timer: f32,
        /// Total duration of this hop (for visual arc calculation).
        duration: f32,
        /// Per-hop height variation (0.5..1.5 range, from seed).
        height_scale: f32,
    },
    /// Wind-up animation before firing web.
    WebWindup { timer: f32 },
    /// Crouch before lunge.
    LungeWindup { timer: f32 },
    /// Lunging toward player position.
    LungeAttack {
        /// Target position at lunge start (locked in).
        target: Vec2,
        timer: f32,
        dealt_damage: bool,
    },
    /// Brief pause after any attack before re-engaging.
    Recover { timer: f32 },
    /// Playing death animation.
    Dying { timer: f32 },
    /// Inert fire-death before despawn.
    BurningCorpse { timer: f32, seed: u32 },
    /// Fully dead.
    Dead,
}

impl SpideySimState {
    #[must_use]
    pub const fn is_alive(&self) -> bool {
        !matches!(
            self,
            Self::Dying { .. } | Self::BurningCorpse { .. } | Self::Dead
        )
    }
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Gameplay-only configuration for Spidey simulation.
#[derive(Clone, Debug)]
pub struct SpideySimConfig {
    pub move_speed: f32,
    pub collision_radius: f32,
    pub aggro_range: f32,
    /// Minimum seconds between hops.
    pub hop_interval_min: f32,
    /// Maximum seconds between hops.
    pub hop_interval_max: f32,
    /// Distance covered per hop (map units).
    pub hop_distance: f32,
    /// Duration of a single hop (seconds).
    pub hop_duration: f32,
    /// Peak billboard height during a hop (visual only).
    pub hop_visual_height: f32,
    /// Maximum range at which lunge melee is chosen.
    pub lunge_range: f32,
    /// Movement speed during lunge (map units/s).
    pub lunge_speed: f32,
    /// Damage dealt on lunge arrival.
    pub lunge_melee_damage: u32,
    /// Crouch duration before lunge.
    pub lunge_windup_secs: f32,
    /// Maximum lunge duration before auto-recover.
    pub lunge_duration_secs: f32,
    /// Seconds between lunge attacks.
    pub lunge_cooldown: f32,
    /// Maximum range for web attack.
    pub web_range: f32,
    /// Seconds between web attacks.
    pub web_cooldown: f32,
    /// Animation lead before projectile spawns.
    pub web_cue_secs: f32,
    /// Web projectile speed.
    pub web_projectile_speed: f32,
    /// Web projectile damage.
    pub web_projectile_damage: u32,
    /// Recovery pause duration after attacks.
    pub recover_secs: f32,
    /// Death animation duration.
    pub death_secs: f32,
}

impl Default for SpideySimConfig {
    fn default() -> Self {
        crate::config::FpsCombatConfig::default().spidey_sim_config()
    }
}

impl SpideySimConfig {
    /// Apply map-authored movement speed as a scale over movement distances.
    ///
    /// Spidey has no continuous walk state; hops use `hop_distance` and lunges
    /// use `lunge_speed`. `move_speed` remains the authoring baseline used to
    /// scale those movement values consistently.
    #[must_use]
    pub fn with_authored_speed(mut self, speed: f32) -> Self {
        let speed = speed.max(0.0);
        let scale = if self.move_speed > f32::EPSILON {
            speed / self.move_speed
        } else {
            1.0
        };
        self.move_speed = speed;
        self.hop_distance *= scale;
        self.lunge_speed *= scale;
        self
    }
}

// ---------------------------------------------------------------------------
// Sim I/O
// ---------------------------------------------------------------------------

/// Mutable simulation fields passed into `tick_spidey_sim`.
pub struct SpideySim {
    pub position: Vec2,
    pub state: SpideySimState,
    pub web_cooldown: f32,
    pub lunge_cooldown: f32,
    /// When `Some(elapsed)`, a web animation is playing. The projectile
    /// spawns when `elapsed >= config.web_cue_secs`.
    pub web_anim_elapsed: Option<f32>,
    /// Stable per-instance seed for deterministic decisions.
    pub seed: u32,
}

/// Outputs from one simulation tick.
#[derive(Clone, Debug, Default)]
pub struct SpideySimOutput {
    /// Projectile to spawn (if any).
    pub projectile: Option<Projectile>,
    /// Melee damage to apply: (amount, `source_position`).
    pub melee_damage: Option<(u32, Vec2)>,
    /// Velocity this frame (for rendering/animation).
    pub velocity: Vec2,
    /// Visual hop height this frame (0.0 when grounded). Billboard-only.
    pub visual_height: f32,
    /// Normalized presentation phase for hop/lunge arcs. Billboard-only.
    pub visual_phase: f32,
    /// Whether a web animation was started this tick.
    pub started_web_anim: bool,
    /// Whether a lunge was started this tick.
    pub started_lunge: bool,
}

// ---------------------------------------------------------------------------
// Core sim tick
// ---------------------------------------------------------------------------

/// Pure Spidey simulation tick. No rendering, no ECS.
///
/// Mutates `sim` in place and returns semantic outputs. The caller
/// (fps crate or server) is responsible for spawning projectiles,
/// applying damage, and driving animation.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn tick_spidey_sim(
    sim: &mut SpideySim,
    config: &SpideySimConfig,
    player_pos: Vec2,
    map: &Map,
    dt: f32,
) -> SpideySimOutput {
    if config.lunge_range >= config.web_range {
        bevy::log::warn!(
            "spidey: lunge_range ({}) >= web_range ({}) — web attacks will never fire",
            config.lunge_range,
            config.web_range,
        );
    }

    let mut output = SpideySimOutput::default();

    // Tick cooldowns.
    sim.web_cooldown = (sim.web_cooldown - dt).max(0.0);
    sim.lunge_cooldown = (sim.lunge_cooldown - dt).max(0.0);

    // Tick web animation — spawn projectile at cue point.
    // Projectile originates from the spinneret (rear), offset away from the target.
    if let Some(elapsed) = &mut sim.web_anim_elapsed {
        *elapsed += dt;
        if *elapsed >= config.web_cue_secs {
            let to_player = player_pos - sim.position;
            let dir = to_player.normalize_or_zero();
            let spawn_pos = sim.position - dir * config.collision_radius;
            if let Some(proj) = Projectile::new(spawn_pos, player_pos, config.web_projectile_damage)
            {
                let mut p = proj;
                p.speed = config.web_projectile_speed;
                p.radius = 0.15;
                output.projectile = Some(p);
            }
            sim.web_anim_elapsed = None;
        }
    }

    match &mut sim.state {
        SpideySimState::Dead => {}

        SpideySimState::Dying { timer } | SpideySimState::BurningCorpse { timer, .. } => {
            *timer -= dt;
            if *timer <= 0.0 {
                sim.state = SpideySimState::Dead;
            }
        }

        SpideySimState::Recover { timer } => {
            *timer -= dt;
            if *timer <= 0.0 {
                sim.state = SpideySimState::Idle;
            }
        }

        SpideySimState::Idle => {
            let to_player = player_pos - sim.position;
            let dist = to_player.length();

            if dist > config.aggro_range {
                return output;
            }

            // Decide: leap, web, or start hopping.
            if try_start_lunge(sim, config, player_pos, dist, map) {
                output.started_lunge = true;
            } else if try_start_web(sim, config, player_pos, dist, map, &mut output) {
                // Web wind-up started.
            } else {
                start_hop_wait(sim, config);
            }
        }

        SpideySimState::HopWait { timer } => {
            *timer -= dt;
            if *timer <= 0.0 {
                let to_player = player_pos - sim.position;
                let dist = to_player.length();

                // Re-evaluate attack opportunities at hop boundary.
                if try_start_lunge(sim, config, player_pos, dist, map) {
                    output.started_lunge = true;
                } else if try_start_web(sim, config, player_pos, dist, map, &mut output) {
                    // Web wind-up started.
                } else if dist > 0.01 {
                    // Start a hop toward the player with randomized height
                    // and lateral jitter for zigzag approach.
                    let base_dir = to_player / dist;
                    let jitter_angle = hop_lateral_jitter(sim.seed);
                    sim.seed = sim.seed.wrapping_add(1);
                    let direction = Vec2::new(
                        base_dir
                            .x
                            .mul_add(jitter_angle.cos(), -(base_dir.y * jitter_angle.sin())),
                        base_dir
                            .x
                            .mul_add(jitter_angle.sin(), base_dir.y * jitter_angle.cos()),
                    );
                    let height_scale = hop_height_scale(sim.seed);
                    sim.seed = sim.seed.wrapping_add(1);
                    sim.state = SpideySimState::HopMove {
                        direction,
                        timer: config.hop_duration,
                        duration: config.hop_duration,
                        height_scale,
                    };
                } else {
                    start_hop_wait(sim, config);
                }
            }
        }

        SpideySimState::HopMove {
            direction,
            timer,
            duration,
            height_scale,
        } => {
            let elapsed = *duration - *timer;
            *timer -= dt;

            // Move toward target. Velocity = hop_distance / hop_duration so
            // designers tune the per-hop travel distance directly.
            let hop_speed = config.hop_distance / config.hop_duration.max(f32::EPSILON);
            let step = *direction * hop_speed * dt;
            let pos_before = sim.position;
            try_move(&mut sim.position, step, config.collision_radius, map);
            output.velocity = (sim.position - pos_before) / dt.max(f32::EPSILON);

            // Parabolic visual height: peak at midpoint, varied by height_scale.
            // Clamped so the billboard top doesn't exceed the ceiling (wall_height=1.0).
            let t_norm = (dt.mul_add(0.5, elapsed) / duration.max(f32::EPSILON)).clamp(0.0, 1.0);
            let raw_height =
                config.hop_visual_height * *height_scale * 4.0 * t_norm * (1.0 - t_norm);
            output.visual_height = raw_height.min(MAX_VISUAL_HEIGHT);
            output.visual_phase = t_norm;

            if *timer <= 0.0 {
                // Hop finished — check for attack or wait for next hop.
                let to_player = player_pos - sim.position;
                let dist = to_player.length();

                if try_start_lunge(sim, config, player_pos, dist, map) {
                    output.started_lunge = true;
                } else if try_start_web(sim, config, player_pos, dist, map, &mut output) {
                    // Web wind-up started.
                } else {
                    start_hop_wait(sim, config);
                }
            }
        }

        SpideySimState::WebWindup { timer } => {
            *timer -= dt;
            if *timer <= 0.0 {
                // Transition to recover after web fires (projectile spawns via anim cue).
                sim.state = SpideySimState::Recover {
                    timer: config.recover_secs,
                };
            }
        }

        SpideySimState::LungeWindup { timer } => {
            *timer -= dt;
            if *timer <= 0.0 {
                sim.state = SpideySimState::LungeAttack {
                    target: player_pos,
                    timer: config.lunge_duration_secs,
                    dealt_damage: false,
                };
            }
        }

        SpideySimState::LungeAttack {
            target,
            timer,
            dealt_damage,
        } => {
            let to_target = *target - sim.position;
            let dist = to_target.length();

            if dist > 0.01 {
                let dir = to_target / dist;
                let step_len = (config.lunge_speed * dt).min(dist);
                let step = dir * step_len;
                let pos_before = sim.position;
                try_move(&mut sim.position, step, config.collision_radius, map);
                let actual_move = sim.position - pos_before;
                output.velocity = actual_move / dt.max(f32::EPSILON);

                // Visual leap arc.
                let total = config.lunge_duration_secs;
                let elapsed = total - *timer;
                let t_norm = (dt.mul_add(0.5, elapsed) / total.max(f32::EPSILON)).clamp(0.0, 1.0);
                let raw_lunge_height =
                    config.hop_visual_height * 2.0 * 4.0 * t_norm * (1.0 - t_norm);
                output.visual_height = raw_lunge_height.min(MAX_VISUAL_HEIGHT);
                output.visual_phase = t_norm;

                // Check if blocked (wall collision prevented movement).
                let blocked = actual_move.length_squared() < (step_len * 0.1).powi(2)
                    && step_len > f32::EPSILON;
                if blocked {
                    // Blocked by wall — no damage, recover.
                    sim.state = SpideySimState::Recover {
                        timer: config.recover_secs,
                    };
                    sim.lunge_cooldown = config.lunge_cooldown;
                    return output;
                }
            }

            // Check arrival (close enough to target or to player).
            let dist_to_player = sim.position.distance(player_pos);
            if !*dealt_damage && dist_to_player <= config.collision_radius + LUNGE_MELEE_HIT_MARGIN
            {
                output.melee_damage = Some((config.lunge_melee_damage, sim.position));
                *dealt_damage = true;
            }

            *timer -= dt;
            if *timer <= 0.0 || dist <= config.collision_radius {
                sim.state = SpideySimState::Recover {
                    timer: config.recover_secs,
                };
                sim.lunge_cooldown = config.lunge_cooldown;
            }
        }
    }

    output
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Deterministic hop interval based on seed.
/// Uses Knuth's multiplicative hash (2654435761 = golden ratio × 2^32).
fn hop_interval(config: &SpideySimConfig, seed: u32) -> f32 {
    let range = config.hop_interval_max - config.hop_interval_min;
    let t = ((seed.wrapping_mul(2_654_435_761) >> 16) as f32) / 65536.0;
    range.mul_add(t, config.hop_interval_min)
}

/// Deterministic lateral jitter angle for hop direction (radians).
/// Range: roughly -45° to +45° (-0.78 to +0.78 rad).
/// Uses MSDOS LCG constants (214013/2531011) for decorrelation from height.
fn hop_lateral_jitter(seed: u32) -> f32 {
    let t = ((seed.wrapping_mul(214_013).wrapping_add(2_531_011) >> 16) as f32) / 65536.0;
    (t - 0.5) * std::f32::consts::FRAC_PI_2 // [-PI/4, +PI/4]
}

/// Deterministic hop height variation (0.5..1.5 range).
/// Uses Numerical Recipes LCG constants (1664525/1013904223) to avoid
/// correlation with jitter (which uses different constants).
fn hop_height_scale(seed: u32) -> f32 {
    let t = ((seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223) >> 16) as f32) / 65536.0;
    0.5 + t // range [0.5, 1.5]
}

fn start_hop_wait(sim: &mut SpideySim, config: &SpideySimConfig) {
    let interval = hop_interval(config, sim.seed);
    sim.seed = sim.seed.wrapping_add(1);
    sim.state = SpideySimState::HopWait { timer: interval };
}

fn try_start_lunge(
    sim: &mut SpideySim,
    config: &SpideySimConfig,
    player_pos: Vec2,
    dist: f32,
    map: &Map,
) -> bool {
    if dist <= config.lunge_range
        && sim.lunge_cooldown <= 0.0
        && has_line_of_sight(sim.position, player_pos, map)
    {
        sim.web_anim_elapsed = None;
        sim.state = SpideySimState::LungeWindup {
            timer: config.lunge_windup_secs,
        };
        true
    } else {
        false
    }
}

fn try_start_web(
    sim: &mut SpideySim,
    config: &SpideySimConfig,
    player_pos: Vec2,
    dist: f32,
    map: &Map,
    output: &mut SpideySimOutput,
) -> bool {
    if sim.web_cooldown <= 0.0
        && sim.web_anim_elapsed.is_none()
        && dist <= config.web_range
        && dist > config.lunge_range
        && has_line_of_sight(sim.position, player_pos, map)
    {
        sim.web_anim_elapsed = Some(0.0);
        sim.web_cooldown = config.web_cooldown;
        // This state covers windup plus a short post-fire hold; projectile
        // cue is driven separately by `web_anim_elapsed`.
        sim.state = SpideySimState::WebWindup {
            timer: config.web_cue_secs + config.recover_secs,
        };
        output.started_web_anim = true;
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::map::test_map;

    fn make_sim(x: f32, y: f32) -> SpideySim {
        let pos = Vec2::new(x, y);
        SpideySim {
            position: pos,
            state: SpideySimState::Idle,
            web_cooldown: 0.0,
            lunge_cooldown: 0.0,
            web_anim_elapsed: None,
            seed: crate::fire_death::corpse_seed(pos),
        }
    }

    fn default_config() -> SpideySimConfig {
        SpideySimConfig::default()
    }

    // -- Hop respects walls --

    #[test]
    fn hop_respects_walls() {
        // test_map has wall at column 0 (border). Place Spidey near west wall,
        // hop direction pointing west.
        let map = test_map();
        let config = default_config();
        let mut sim = make_sim(1.3, 3.5);
        sim.state = SpideySimState::HopMove {
            direction: Vec2::new(-1.0, 0.0),
            timer: config.hop_duration,
            duration: config.hop_duration,
            height_scale: 1.0,
        };

        // Tick enough to complete the hop.
        for _ in 0..100 {
            let _ = tick_spidey_sim(&mut sim, &config, Vec2::new(0.5, 3.5), &map, 0.016);
        }

        // Should not have crossed through the wall at x=1.0.
        assert!(
            sim.position.x >= 1.0,
            "spidey should not hop through wall: {:?}",
            sim.position
        );
    }

    // -- Chooses leap within range --

    #[test]
    fn chooses_lunge_within_lunge_range() {
        let map = test_map();
        let config = default_config();
        let player = Vec2::new(2.5, 3.5);
        // Place within lunge_range of player.
        let mut sim = make_sim(2.5 + config.lunge_range * 0.5, 3.5);
        sim.lunge_cooldown = 0.0;

        let output = tick_spidey_sim(&mut sim, &config, player, &map, 0.016);

        assert!(
            matches!(sim.state, SpideySimState::LungeWindup { .. }),
            "should enter LungeWindup, got {:?}",
            sim.state
        );
        assert!(output.started_lunge);
    }

    // -- Chooses web at distance --

    #[test]
    fn chooses_web_at_distance() {
        let map = test_map();
        let config = SpideySimConfig {
            web_range: 10.0,
            aggro_range: 12.0,
            ..default_config()
        };
        // Place beyond leap range but within web range.
        let player = Vec2::new(5.5, 3.5);
        let mut sim = make_sim(1.5, 3.5);
        sim.web_cooldown = 0.0;

        let output = tick_spidey_sim(&mut sim, &config, player, &map, 0.016);

        assert!(
            matches!(sim.state, SpideySimState::WebWindup { .. }),
            "should enter WebWindup, got {:?}",
            sim.state
        );
        assert!(output.started_web_anim);
    }

    // -- Leap deals damage once --

    #[test]
    fn lunge_deals_damage_once() {
        let map = test_map();
        let config = default_config();
        let player = Vec2::new(2.0, 3.5);
        let mut sim = make_sim(2.0, 3.5); // On top of player.
        sim.state = SpideySimState::LungeAttack {
            target: player,
            timer: 0.5,
            dealt_damage: false,
        };

        let out1 = tick_spidey_sim(&mut sim, &config, player, &map, 0.016);
        assert!(out1.melee_damage.is_some(), "first tick should deal damage");
        assert_eq!(out1.melee_damage.unwrap().0, config.lunge_melee_damage);

        // If still in LungeAttack, second tick should not re-deal.
        if matches!(sim.state, SpideySimState::LungeAttack { .. }) {
            let out2 = tick_spidey_sim(&mut sim, &config, player, &map, 0.016);
            assert!(
                out2.melee_damage.is_none(),
                "second tick should not re-deal damage"
            );
        }
    }

    // -- Blocked leap deals no damage --

    #[test]
    fn blocked_lunge_deals_no_damage() {
        // Build a map with a wall between spidey and target.
        let map = Map {
            width: 5,
            height: 3,
            cells: vec![1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 1, 1, 1, 1],
        };
        let config = SpideySimConfig {
            lunge_speed: 10.0,
            lunge_duration_secs: 1.0,
            collision_radius: 0.2,
            ..default_config()
        };
        let player = Vec2::new(3.5, 1.5);
        let mut sim = make_sim(1.5, 1.5);
        sim.state = SpideySimState::LungeAttack {
            target: player,
            timer: 1.0,
            dealt_damage: false,
        };

        // Tick until state changes (blocked by wall).
        let mut any_damage = false;
        for _ in 0..200 {
            let out = tick_spidey_sim(&mut sim, &config, player, &map, 0.016);
            if out.melee_damage.is_some() {
                any_damage = true;
            }
            if !matches!(sim.state, SpideySimState::LungeAttack { .. }) {
                break;
            }
        }

        assert!(!any_damage, "blocked lunge should not deal damage");
        assert!(
            matches!(sim.state, SpideySimState::Recover { .. }),
            "should recover after blocked lunge, got {:?}",
            sim.state
        );
    }

    // -- Web emits projectile at cue --

    #[test]
    fn web_emits_projectile_at_cue() {
        let map = test_map();
        let config = SpideySimConfig {
            web_cue_secs: 0.3,
            web_range: 10.0,
            aggro_range: 12.0,
            ..default_config()
        };
        let player = Vec2::new(5.5, 3.5);
        let mut sim = make_sim(1.5, 3.5);
        sim.web_anim_elapsed = Some(0.0);
        sim.web_cooldown = 10.0; // Prevent re-trigger.
        sim.state = SpideySimState::WebWindup {
            timer: config.web_cue_secs + config.recover_secs,
        };

        // Not yet at cue.
        let out1 = tick_spidey_sim(&mut sim, &config, player, &map, 0.1);
        assert!(out1.projectile.is_none());
        assert!(sim.web_anim_elapsed.is_some());

        // Past cue.
        let out2 = tick_spidey_sim(&mut sim, &config, player, &map, 0.3);
        assert!(out2.projectile.is_some());
        assert!(sim.web_anim_elapsed.is_none());

        let proj = out2.projectile.unwrap();
        assert_eq!(proj.speed, config.web_projectile_speed);
        assert_eq!(proj.damage, config.web_projectile_damage);
    }

    // -- Cooldowns prevent spam --

    #[test]
    fn web_cooldown_prevents_spam() {
        let map = test_map();
        let config = SpideySimConfig {
            web_range: 10.0,
            aggro_range: 12.0,
            lunge_range: 0.5, // Keep leap range tiny so web is chosen.
            ..default_config()
        };
        let player = Vec2::new(5.5, 3.5);
        let mut sim = make_sim(1.5, 3.5);

        // First tick starts web anim.
        let out1 = tick_spidey_sim(&mut sim, &config, player, &map, 0.016);
        assert!(out1.started_web_anim);

        // Force back to idle, clear anim.
        sim.state = SpideySimState::Idle;
        sim.web_anim_elapsed = None;

        // Second tick should NOT start web (cooldown active).
        let out2 = tick_spidey_sim(&mut sim, &config, player, &map, 0.016);
        assert!(!out2.started_web_anim);
        assert!(sim.web_cooldown > 0.0);
    }

    #[test]
    fn lunge_cooldown_prevents_spam() {
        let map = test_map();
        let config = default_config();
        let player = Vec2::new(2.0, 3.5);
        let mut sim = make_sim(2.0 + config.lunge_range * 0.5, 3.5);

        // First tick starts leap.
        let out1 = tick_spidey_sim(&mut sim, &config, player, &map, 0.016);
        assert!(out1.started_lunge);

        // Force back to idle, set leap on cooldown.
        sim.state = SpideySimState::Idle;
        sim.lunge_cooldown = config.lunge_cooldown;

        // Second tick should NOT start leap.
        let out2 = tick_spidey_sim(&mut sim, &config, player, &map, 0.016);
        assert!(!out2.started_lunge);
        assert!(
            !matches!(sim.state, SpideySimState::LungeWindup { .. }),
            "should not lunge on cooldown"
        );
    }

    // -- Death transitions --

    #[test]
    fn dying_transitions_to_dead() {
        let map = test_map();
        let config = default_config();
        let mut sim = make_sim(1.5, 1.5);
        sim.state = SpideySimState::Dying { timer: 0.1 };

        let _ = tick_spidey_sim(&mut sim, &config, Vec2::ZERO, &map, 0.2);

        assert_eq!(sim.state, SpideySimState::Dead);
    }

    #[test]
    fn burning_corpse_transitions_to_dead() {
        let map = test_map();
        let config = default_config();
        let mut sim = make_sim(1.5, 1.5);
        sim.state = SpideySimState::BurningCorpse {
            timer: 0.1,
            seed: 42,
        };

        let _ = tick_spidey_sim(&mut sim, &config, Vec2::ZERO, &map, 0.2);

        assert_eq!(sim.state, SpideySimState::Dead);
    }

    #[test]
    fn dead_does_nothing() {
        let map = test_map();
        let config = default_config();
        let player = Vec2::new(1.5, 1.5);
        let mut sim = make_sim(3.0, 3.5);
        sim.state = SpideySimState::Dead;
        let pos_before = sim.position;

        let output = tick_spidey_sim(&mut sim, &config, player, &map, 1.0);

        assert_eq!(sim.position, pos_before);
        assert!(output.projectile.is_none());
        assert!(output.melee_damage.is_none());
    }

    // -- Visual height during hop --

    #[test]
    fn hop_produces_visual_height() {
        let map = test_map();
        let config = default_config();
        let player = Vec2::new(5.5, 3.5);
        let mut sim = make_sim(1.5, 3.5);
        sim.state = SpideySimState::HopMove {
            direction: Vec2::new(1.0, 0.0),
            timer: config.hop_duration,
            duration: config.hop_duration,
            height_scale: 1.0,
        };

        // Tick to roughly midpoint of hop.
        let out = tick_spidey_sim(&mut sim, &config, player, &map, config.hop_duration * 0.5);

        assert!(
            out.visual_height > 0.0,
            "hop midpoint should have positive visual height: {}",
            out.visual_height
        );
    }

    // -- Idle ignores distant player --

    #[test]
    fn idle_ignores_distant_player() {
        let map = test_map();
        let config = default_config();
        let player = Vec2::new(100.0, 100.0); // Way out of aggro range.
        let mut sim = make_sim(1.5, 3.5);

        let _ = tick_spidey_sim(&mut sim, &config, player, &map, 0.016);

        assert_eq!(sim.state, SpideySimState::Idle);
    }

    // -- Recover transitions to idle --

    #[test]
    fn recover_transitions_to_idle() {
        let map = test_map();
        let config = default_config();
        let mut sim = make_sim(1.5, 1.5);
        sim.state = SpideySimState::Recover { timer: 0.1 };

        let _ = tick_spidey_sim(&mut sim, &config, Vec2::ZERO, &map, 0.2);

        assert_eq!(sim.state, SpideySimState::Idle);
    }

    // -- Config default matches combat config --

    #[test]
    fn config_default_matches_combat_config() {
        let c = SpideySimConfig::default();
        let combat = crate::config::FpsCombatConfig::default();
        assert_eq!(c.lunge_melee_damage, combat.spidey_lunge_melee_damage);
        assert_eq!(c.web_cooldown, combat.spidey_web_cooldown);
        assert_eq!(c.lunge_cooldown, combat.spidey_lunge_cooldown);
        assert_eq!(c.web_projectile_speed, combat.spidey_web_projectile_speed);
        assert_eq!(c.web_projectile_damage, combat.spidey_web_projectile_damage);
        assert_eq!(c.collision_radius, combat.spidey_collision_radius);
    }
}
