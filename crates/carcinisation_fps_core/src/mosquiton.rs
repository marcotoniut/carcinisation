//! Pure Mosquiton combat simulation — state machine, cooldowns, attack decisions.
//!
//! No rendering, no Bevy ECS. The fps crate wraps this with animation/billboard
//! concerns; the server can adopt the same sim path for parity.

use bevy_math::Vec2;

use crate::collision::try_move;
use crate::config;
use crate::enemy::Projectile;
use crate::map::Map;
use crate::raycast::has_line_of_sight;
use crate::reaction::{EnemyReactionConfig, EnemyReactionState};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Pure simulation state for a Mosquiton.
#[derive(Clone, Debug, PartialEq)]
pub enum MosquitonSimState {
    /// Moving toward the player.
    Pursue,
    /// At preferred range, strafing and shooting.
    RangedAttack { strafe_dir: f32 },
    /// Close enough for melee.
    MeleeAttack { timer: f32, dealt_damage: bool },
    /// Brief pause after melee before re-engaging.
    Recover { timer: f32 },
    /// Playing death animation.
    Dying { timer: f32 },
    /// Inert fire-death before despawn.
    BurningCorpse { timer: f32, seed: u32 },
    /// Fully dead.
    Dead,
}

impl MosquitonSimState {
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

/// Gameplay-only configuration for Mosquiton simulation.
/// All values derive from `config.rs` constants by default.
#[derive(Clone, Debug)]
pub struct MosquitonSimConfig {
    pub move_speed: f32,
    pub preferred_range: f32,
    pub melee_range: f32,
    pub shoot_range: f32,
    pub shoot_cooldown: f32,
    pub melee_cooldown: f32,
    pub melee_attack_duration: f32,
    pub melee_damage: u32,
    pub blood_shot_speed: f32,
    pub blood_shot_damage: u32,
    pub collision_radius: f32,
    pub shoot_cue_secs: f32,
    /// Poise/stun rules for hit reactions (shared enemy tuning).
    pub reaction: EnemyReactionConfig,
}

impl Default for MosquitonSimConfig {
    fn default() -> Self {
        config::FpsCombatConfig::default().mosquiton_sim_config()
    }
}

// ---------------------------------------------------------------------------
// Sim I/O
// ---------------------------------------------------------------------------

/// Mutable simulation fields passed into `tick_mosquiton_sim`.
pub struct MosquitonSim {
    pub position: Vec2,
    pub state: MosquitonSimState,
    pub shoot_cooldown: f32,
    pub melee_cooldown: f32,
    pub decision_timer: f32,
    /// When `Some(elapsed)`, a shoot animation is playing. The projectile
    /// spawns when `elapsed >= config.shoot_cue_secs`.
    pub shoot_anim_elapsed: Option<f32>,
    /// Stable per-instance seed for deterministic decisions (e.g. initial strafe direction).
    /// Should be set once at spawn from position bits and not changed.
    pub seed: u32,
    /// Hit-reaction runtime state (poise, stun, knockback). Persisted across
    /// ticks by the SP wrapper / server component like the cooldowns above.
    pub reaction: EnemyReactionState,
}

/// Outputs from one simulation tick.
#[derive(Clone, Debug, Default)]
pub struct MosquitonSimOutput {
    /// Projectile to spawn (if any).
    pub projectile: Option<Projectile>,
    /// Melee damage to apply: (amount, `source_position`).
    pub melee_damage: Option<(u32, Vec2)>,
    /// Velocity this frame (for rendering/animation).
    pub velocity: Vec2,
    /// Whether a shoot animation was started this tick.
    pub started_shoot_anim: bool,
    /// Whether a melee attack was started this tick.
    pub started_melee: bool,
}

// ---------------------------------------------------------------------------
// Core sim tick
// ---------------------------------------------------------------------------

/// Pure Mosquiton simulation tick. No rendering, no ECS.
///
/// Mutates `sim` in place and returns semantic outputs. The caller
/// (fps crate or server) is responsible for spawning projectiles,
/// applying damage, and driving animation.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn tick_mosquiton_sim(
    sim: &mut MosquitonSim,
    config: &MosquitonSimConfig,
    player_pos: Vec2,
    map: &Map,
    dt: f32,
) -> MosquitonSimOutput {
    let mut output = MosquitonSimOutput::default();

    // Tick cooldowns.
    sim.shoot_cooldown = (sim.shoot_cooldown - dt).max(0.0);
    sim.melee_cooldown = (sim.melee_cooldown - dt).max(0.0);
    sim.decision_timer = (sim.decision_timer - dt).max(0.0);

    // Hit reactions: consume hits queued by the damage path, tick poise/stun,
    // and apply knockback through wall-aware movement. Dead/dying enemies do
    // not react (corpses never slide or stagger).
    if sim.state.is_alive() {
        let knockback = sim.reaction.tick(&config.reaction, dt);
        if knockback != Vec2::ZERO {
            try_move(&mut sim.position, knockback, config.collision_radius, map);
        }
    } else {
        sim.reaction.clear();
    }
    let stunned = sim.reaction.is_stunned();

    // Tick shoot animation — spawn projectile at cue point.
    if let Some(elapsed) = &mut sim.shoot_anim_elapsed {
        *elapsed += dt;
        if *elapsed >= config.shoot_cue_secs {
            if let Some(proj) = Projectile::new(sim.position, player_pos, config.blood_shot_damage)
            {
                let mut p = proj;
                p.speed = config.blood_shot_speed;
                p.radius = 0.2;
                output.projectile = Some(p);
            }
            sim.shoot_anim_elapsed = None;
        }
    }

    match &mut sim.state {
        MosquitonSimState::Dead => {}

        MosquitonSimState::Dying { timer } | MosquitonSimState::BurningCorpse { timer, .. } => {
            *timer -= dt;
            if *timer <= 0.0 {
                sim.state = MosquitonSimState::Dead;
            }
        }

        MosquitonSimState::Recover { timer } => {
            *timer -= dt;
            if *timer <= 0.0 {
                sim.state = MosquitonSimState::Pursue;
            }
        }

        MosquitonSimState::Pursue => {
            // Hit-stun: no movement, no new attacks. (Committed states like
            // MeleeAttack are not gated — they run to completion.)
            if stunned {
                return output;
            }
            let to_player = player_pos - sim.position;
            let dist = to_player.length();

            if dist <= config.melee_range {
                if sim.melee_cooldown <= 0.0 {
                    start_melee(sim, config, &mut output);
                } else {
                    output.velocity = back_off(&mut sim.position, to_player, dist, config, map, dt);
                }
                return output;
            }

            if dist <= config.preferred_range {
                let strafe_dir = if sim.seed & 1 == 0 { 1.0 } else { -1.0 };
                sim.state = MosquitonSimState::RangedAttack { strafe_dir };
                return output;
            }

            // Move toward player.
            if dist > 0.01 {
                let move_dir = to_player / dist;
                let step = move_dir * config.move_speed * dt;
                output.velocity = step / dt.max(f32::EPSILON);
                try_move(&mut sim.position, step, config.collision_radius, map);
            }

            // Start shoot animation if cooldown ready and LOS clear.
            try_start_shoot(sim, config, player_pos, dist, map, &mut output);
        }

        MosquitonSimState::RangedAttack { strafe_dir } => {
            // Hit-stun: no strafing, no new attacks.
            if stunned {
                return output;
            }
            let to_player = player_pos - sim.position;
            let dist = to_player.length();

            if dist <= config.melee_range {
                if sim.melee_cooldown <= 0.0 {
                    start_melee(sim, config, &mut output);
                } else {
                    output.velocity = back_off(&mut sim.position, to_player, dist, config, map, dt);
                }
                return output;
            }

            if dist > config.preferred_range * 1.5 {
                sim.state = MosquitonSimState::Pursue;
                return output;
            }

            if sim.decision_timer <= 0.0 {
                *strafe_dir *= -1.0;
                sim.decision_timer = 0.75;
            }

            // Strafe perpendicular.
            if dist > 0.01 {
                let dir_to_player = to_player / dist;
                let strafe = Vec2::new(-dir_to_player.y, dir_to_player.x) * *strafe_dir;
                let step = strafe * config.move_speed * 0.5 * dt;
                output.velocity = step / dt.max(f32::EPSILON);
                try_move(&mut sim.position, step, config.collision_radius, map);
            }

            // Start shoot animation on cooldown.
            try_start_shoot(sim, config, player_pos, dist, map, &mut output);
        }

        MosquitonSimState::MeleeAttack {
            timer,
            dealt_damage,
        } => {
            let dist = sim.position.distance(player_pos);

            if dist > config.melee_range * 1.5 {
                sim.state = MosquitonSimState::Pursue;
                return output;
            }

            if !*dealt_damage {
                output.melee_damage = Some((config.melee_damage, sim.position));
                *dealt_damage = true;
                sim.melee_cooldown = config.melee_cooldown;
            }

            *timer -= dt;
            if *timer <= 0.0 {
                sim.state = MosquitonSimState::Recover { timer: 0.2 };
            }
        }
    }

    output
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const fn start_melee(
    sim: &mut MosquitonSim,
    config: &MosquitonSimConfig,
    output: &mut MosquitonSimOutput,
) {
    sim.shoot_anim_elapsed = None;
    sim.state = MosquitonSimState::MeleeAttack {
        timer: config.melee_attack_duration,
        dealt_damage: false,
    };
    output.started_melee = true;
}

fn back_off(
    position: &mut Vec2,
    to_player: Vec2,
    dist: f32,
    config: &MosquitonSimConfig,
    map: &Map,
    dt: f32,
) -> Vec2 {
    if dist <= 0.01 {
        return Vec2::ZERO;
    }
    let step = -(to_player / dist) * config.move_speed * 0.35 * dt;
    try_move(position, step, config.collision_radius, map);
    step / dt.max(f32::EPSILON)
}

fn try_start_shoot(
    sim: &mut MosquitonSim,
    config: &MosquitonSimConfig,
    player_pos: Vec2,
    dist: f32,
    map: &Map,
    output: &mut MosquitonSimOutput,
) {
    if sim.shoot_cooldown <= 0.0
        && sim.shoot_anim_elapsed.is_none()
        && dist < config.shoot_range
        && has_line_of_sight(sim.position, player_pos, map)
    {
        sim.shoot_anim_elapsed = Some(0.0);
        sim.shoot_cooldown = config.shoot_cooldown;
        output.started_shoot_anim = true;
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

    fn make_sim(x: f32, y: f32) -> MosquitonSim {
        let pos = Vec2::new(x, y);
        MosquitonSim {
            position: pos,
            state: MosquitonSimState::Pursue,
            shoot_cooldown: 0.0,
            melee_cooldown: 0.0,
            decision_timer: 0.0,
            shoot_anim_elapsed: None,
            seed: crate::fire_death::corpse_seed(pos),
            reaction: crate::reaction::EnemyReactionState::default(),
        }
    }

    fn stun_hit() -> crate::reaction::PendingHitReaction {
        crate::reaction::PendingHitReaction {
            direction: Vec2::X,
            poise_damage: 1_000.0, // far past any threshold → immediate stun
            knockback_distance: 0.0,
            knockback_duration: 0.0,
        }
    }

    fn default_config() -> MosquitonSimConfig {
        MosquitonSimConfig::default()
    }

    #[test]
    fn pursue_moves_toward_player() {
        let map = test_map();
        let config = default_config();
        let mut sim = make_sim(1.5, 1.5);
        // Place player beyond preferred_range so Pursue continues.
        let player = Vec2::new(1.5 + config.preferred_range + 1.0, 1.5);
        let pos_before = sim.position;

        let _ = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.016);

        assert!(
            sim.position.x > pos_before.x,
            "should move east toward player"
        );
        assert!(matches!(sim.state, MosquitonSimState::Pursue));
    }

    #[test]
    fn switches_to_ranged_at_preferred_range() {
        let map = test_map();
        let config = default_config();
        let player = Vec2::new(1.5, 1.5);
        // Place at preferred_range distance.
        let mut sim = make_sim(1.5 + config.preferred_range - 0.1, 1.5);

        let _ = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.016);

        assert!(matches!(sim.state, MosquitonSimState::RangedAttack { .. }));
    }

    #[test]
    fn switches_to_melee_when_close() {
        let map = test_map();
        let config = default_config();
        let player = Vec2::new(1.5, 1.5);
        let mut sim = make_sim(config.melee_range.mul_add(0.5, 1.5), 1.5);

        let output = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.016);

        assert!(matches!(sim.state, MosquitonSimState::MeleeAttack { .. }));
        assert!(output.started_melee);
    }

    #[test]
    fn melee_deals_damage_once() {
        let map = test_map();
        let config = default_config();
        let player = Vec2::new(1.5, 1.5);
        let mut sim = make_sim(config.melee_range.mul_add(0.3, 1.5), 1.5);
        sim.state = MosquitonSimState::MeleeAttack {
            timer: 0.5,
            dealt_damage: false,
        };

        let out1 = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.016);
        let out2 = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.016);

        assert!(out1.melee_damage.is_some(), "first tick should deal damage");
        assert_eq!(out1.melee_damage.unwrap().0, config.melee_damage);
        assert!(
            out2.melee_damage.is_none(),
            "second tick should not re-deal"
        );
    }

    #[test]
    fn ranged_attack_starts_shoot_anim() {
        let map = test_map();
        let config = MosquitonSimConfig {
            shoot_range: 10.0,
            preferred_range: 2.0,
            ..default_config()
        };
        let player = Vec2::new(5.5, 1.5);
        let mut sim = make_sim(1.5, 1.5);
        sim.shoot_cooldown = 0.0;

        let output = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.016);

        assert!(output.started_shoot_anim, "should start shoot anim");
        assert!(sim.shoot_anim_elapsed.is_some());
        assert!(
            output.projectile.is_none(),
            "projectile should not spawn yet"
        );
    }

    #[test]
    fn projectile_spawns_at_cue_point() {
        let map = test_map();
        let config = MosquitonSimConfig {
            shoot_cue_secs: 0.5,
            shoot_range: 10.0,
            preferred_range: 2.0,
            ..default_config()
        };
        let player = Vec2::new(5.5, 1.5);
        let mut sim = make_sim(1.5, 1.5);
        sim.shoot_anim_elapsed = Some(0.0);
        // Prevent re-triggering after projectile spawns.
        sim.shoot_cooldown = 10.0;

        // Not yet at cue.
        let out1 = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.3);
        assert!(out1.projectile.is_none());
        assert!(sim.shoot_anim_elapsed.is_some());

        // Past cue.
        let out2 = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.3);
        assert!(out2.projectile.is_some());
        assert!(sim.shoot_anim_elapsed.is_none());

        let proj = out2.projectile.unwrap();
        assert_eq!(proj.speed, config.blood_shot_speed);
        assert_eq!(proj.damage, config.blood_shot_damage);
    }

    #[test]
    fn shoot_cooldown_prevents_rapid_fire() {
        let map = test_map();
        let config = MosquitonSimConfig {
            shoot_range: 10.0,
            preferred_range: 2.0,
            ..default_config()
        };
        let player = Vec2::new(5.5, 1.5);
        let mut sim = make_sim(1.5, 1.5);

        // First tick starts shoot anim.
        let out1 = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.016);
        assert!(out1.started_shoot_anim);

        // Clear shoot anim (simulate projectile spawned).
        sim.shoot_anim_elapsed = None;

        // Second tick should NOT start another shoot anim (cooldown active).
        let out2 = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.016);
        assert!(!out2.started_shoot_anim);
        assert!(sim.shoot_cooldown > 0.0);
    }

    #[test]
    fn dead_does_nothing() {
        let map = test_map();
        let config = default_config();
        let player = Vec2::new(1.5, 1.5);
        let mut sim = make_sim(3.0, 1.5);
        sim.state = MosquitonSimState::Dead;
        let pos_before = sim.position;

        let output = tick_mosquiton_sim(&mut sim, &config, player, &map, 1.0);

        assert_eq!(sim.position, pos_before);
        assert!(output.projectile.is_none());
        assert!(output.melee_damage.is_none());
    }

    #[test]
    fn dying_transitions_to_dead() {
        let map = test_map();
        let config = default_config();
        let mut sim = make_sim(1.5, 1.5);
        sim.state = MosquitonSimState::Dying { timer: 0.1 };

        let _ = tick_mosquiton_sim(&mut sim, &config, Vec2::ZERO, &map, 0.2);

        assert_eq!(sim.state, MosquitonSimState::Dead);
    }

    #[test]
    fn recover_transitions_to_pursue() {
        let map = test_map();
        let config = default_config();
        let mut sim = make_sim(1.5, 1.5);
        sim.state = MosquitonSimState::Recover { timer: 0.1 };

        let _ = tick_mosquiton_sim(&mut sim, &config, Vec2::ZERO, &map, 0.2);

        assert_eq!(sim.state, MosquitonSimState::Pursue);
    }

    #[test]
    fn no_shoot_without_los() {
        let map = Map {
            width: 5,
            height: 3,
            cells: vec![1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 1, 1, 1, 1],
        };
        let config = MosquitonSimConfig {
            shoot_range: 10.0,
            ..default_config()
        };
        let player = Vec2::new(3.5, 1.5);
        let mut sim = make_sim(1.5, 1.5);
        sim.shoot_cooldown = 0.0;

        let output = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.016);

        assert!(!output.started_shoot_anim);
        assert!(!has_line_of_sight(sim.position, player, &map));
    }

    #[test]
    fn config_default_matches_combat_config() {
        let c = MosquitonSimConfig::default();
        let combat = config::FpsCombatConfig::default();
        assert_eq!(c.melee_range, combat.mosquiton_melee_range);
        assert_eq!(c.preferred_range, combat.mosquiton_preferred_range);
        assert_eq!(c.shoot_range, combat.mosquiton_shoot_range);
        assert_eq!(c.shoot_cooldown, combat.mosquiton_shoot_cooldown);
        assert_eq!(c.melee_cooldown, combat.mosquiton_melee_cooldown);
        assert_eq!(c.blood_shot_speed, combat.mosquiton_blood_shot_speed);
        assert_eq!(c.shoot_cue_secs, combat.mosquiton_shoot_cue_secs);
    }

    #[test]
    fn stunned_pursue_does_not_move_or_shoot() {
        let map = test_map();
        let config = MosquitonSimConfig {
            shoot_range: 10.0,
            ..default_config()
        };
        let mut sim = make_sim(1.5, 1.5);
        sim.reaction.queue_hit(stun_hit());
        let player = Vec2::new(1.5 + config.preferred_range + 1.0, 1.5);
        let pos_before = sim.position;

        let output = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.016);

        assert!(sim.reaction.is_stunned());
        assert_eq!(sim.position, pos_before, "stunned: no pursuit movement");
        assert!(!output.started_shoot_anim, "stunned: no new shot");
        assert!(matches!(sim.state, MosquitonSimState::Pursue));
    }

    #[test]
    fn stunned_does_not_start_melee() {
        let map = test_map();
        let config = default_config();
        let player = Vec2::new(1.5, 1.5);
        let mut sim = make_sim(config.melee_range.mul_add(0.5, 1.5), 1.5);
        sim.reaction.queue_hit(stun_hit());

        let output = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.016);

        assert!(!output.started_melee, "stunned: melee must not start");
        assert!(matches!(sim.state, MosquitonSimState::Pursue));
    }

    #[test]
    fn committed_melee_completes_while_stunned() {
        // Interruptibility decision: committed actions run to completion.
        let map = test_map();
        let config = default_config();
        let player = Vec2::new(1.5, 1.5);
        let mut sim = make_sim(config.melee_range.mul_add(0.3, 1.5), 1.5);
        sim.state = MosquitonSimState::MeleeAttack {
            timer: 0.5,
            dealt_damage: false,
        };
        sim.reaction.queue_hit(stun_hit());

        let output = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.016);

        assert!(sim.reaction.is_stunned());
        assert!(
            output.melee_damage.is_some(),
            "in-flight melee still lands despite stun"
        );
    }

    #[test]
    fn knockback_moves_enemy_along_shot_direction() {
        let map = test_map();
        let config = default_config();
        let mut sim = make_sim(3.5, 3.5);
        // Player far away so the sim has no movement of its own this tick.
        let player = Vec2::new(3.5, 3.5 + config.preferred_range + 3.0);
        sim.reaction.queue_hit(crate::reaction::PendingHitReaction {
            direction: Vec2::X,
            poise_damage: 0.0,
            knockback_distance: 0.3,
            knockback_duration: 0.1,
        });
        let x_before = sim.position.x;

        // Stun-free knockback: tick through the impulse duration.
        for _ in 0..10 {
            let _ = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.02);
        }

        assert!(
            sim.position.x > x_before + 0.2,
            "knocked back along +X: {} -> {}",
            x_before,
            sim.position.x
        );
    }

    #[test]
    fn knockback_respects_walls() {
        let map = test_map();
        let config = default_config();
        // Adjacent to the west border wall (x=0 column is solid).
        let mut sim = make_sim(1.2, 1.5);
        let player = Vec2::new(6.5, 1.5);
        sim.state = MosquitonSimState::Recover { timer: 10.0 }; // no own movement
        sim.reaction.queue_hit(crate::reaction::PendingHitReaction {
            direction: Vec2::NEG_X, // into the wall
            poise_damage: 0.0,
            knockback_distance: 1.0,
            knockback_duration: 0.1,
        });

        for _ in 0..10 {
            let _ = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.02);
        }

        assert!(
            sim.position.x > 1.0,
            "wall clamps knockback: {}",
            sim.position.x
        );
    }

    #[test]
    fn dead_enemy_does_not_react() {
        let map = test_map();
        let config = default_config();
        let mut sim = make_sim(3.0, 1.5);
        sim.state = MosquitonSimState::Dead;
        sim.reaction.queue_hit(crate::reaction::PendingHitReaction {
            direction: Vec2::X,
            poise_damage: 1_000.0,
            knockback_distance: 1.0,
            knockback_duration: 0.2,
        });
        let pos_before = sim.position;

        let _ = tick_mosquiton_sim(&mut sim, &config, Vec2::ZERO, &map, 0.02);

        assert_eq!(sim.position, pos_before, "corpse never slides");
        assert!(!sim.reaction.is_stunned());
        assert!(sim.reaction.pending.is_none(), "pending dropped on corpse");
    }

    #[test]
    fn stun_expires_and_behaviour_resumes() {
        let map = test_map();
        let config = default_config();
        let mut sim = make_sim(1.5, 1.5);
        sim.reaction.queue_hit(stun_hit());
        let player = Vec2::new(1.5 + config.preferred_range + 1.0, 1.5);

        // Consume + ride out the stun.
        let _ = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.016);
        let _ = tick_mosquiton_sim(
            &mut sim,
            &config,
            player,
            &map,
            config.reaction.hit_stun_secs + 0.05,
        );
        assert!(!sim.reaction.is_stunned());

        let pos_before = sim.position;
        let _ = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.016);
        assert!(sim.position.x > pos_before.x, "pursuit resumes after stun");
    }

    #[test]
    fn melee_out_of_range_reverts_to_pursue() {
        let map = test_map();
        let config = default_config();
        let player = Vec2::new(5.5, 1.5);
        let mut sim = make_sim(1.5, 1.5);
        sim.state = MosquitonSimState::MeleeAttack {
            timer: 0.5,
            dealt_damage: false,
        };

        let _ = tick_mosquiton_sim(&mut sim, &config, player, &map, 0.016);

        assert_eq!(sim.state, MosquitonSimState::Pursue);
    }
}
