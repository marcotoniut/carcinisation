//! Enemy hit reactions: poise/stagger and knockback (Phase 11, weapon-only).
//!
//! # Authority and data flow
//!
//! Reactions are **server-authoritative** and live **inside the shared enemy
//! sim** (`MosquitonSim`/`SpideySim` carry an [`EnemyReactionState`]), so
//! single-player and the server run the exact same code path:
//!
//! 1. A hit site (hitscan damage, `CombatSet` on the server) builds a
//!    [`PendingHitReaction`] from the firing weapon's
//!    [`WeaponReactionProfile`]. Server combat queues it via
//!    [`EnemyReactionState::queue_hit`] because the server enemy sim has
//!    already ticked this frame. Single-player combat runs before enemy sim,
//!    so it must queue via [`EnemyReactionState::queue_hit_after_current_tick`]
//!    to preserve the same next-sim-tick timing.
//! 2. The next enemy sim tick (`tick_mosquiton_sim`/`tick_spidey_sim`)
//!    consumes the pending hit via [`EnemyReactionState::tick`]: poise damage
//!    accumulates, stun may trigger, and a knockback impulse starts. The sim
//!    applies the returned knockback displacement through `try_move`
//!    (wall-safe) and gates AI decisions on [`EnemyReactionState::is_stunned`].
//!
//! Enemies are not client-predicted, so reactions replicate naturally through
//! the existing `NetEnemy` position/state — no protocol change.
//!
//! # Poise model (anti-stunlock)
//!
//! Poise is an **up-counting damage accumulator**, not a per-hit stun timer:
//! each hit adds `poise_damage`; when the accumulator crosses
//! [`EnemyReactionConfig::poise_threshold`] the enemy is hit-stunned for
//! [`EnemyReactionConfig::hit_stun_secs`] and the accumulator **resets to
//! zero** — the next stun requires depleting a full bar again, so sustained
//! fire produces periodic staggers, never a permanent stun. After
//! [`EnemyReactionConfig::poise_regen_delay_secs`] without a hit, accumulated
//! poise damage drains at [`EnemyReactionConfig::poise_regen_per_sec`].
//!
//! # Interruptibility (Phase 11 decision)
//!
//! Hit-stun prevents **starting** new actions (attacks, lunges, hops,
//! strafing/pursuit movement). **Committed actions run to completion**: an
//! in-flight melee swing, web/lunge windup, lunge, or airborne hop is not
//! interrupted. This preserves the gameplay value of committed attacks
//! (dodging a lunge matters); per-weapon interruption is future work.
//!
//! The flamethrower does **not** feed poise (continuous exposure × per-tick
//! poise would re-create permanent stun), consistent with flame ignoring part
//! metadata.

use bevy_math::Vec2;

use crate::collision::PartReactionProfile;
use crate::occupancy::OccupancyImpulse;

// ---------------------------------------------------------------------------
// Config (static tuning)
// ---------------------------------------------------------------------------

/// Enemy-side poise/stun tuning shared by all reacting enemy kinds.
///
/// Values are provisional Phase 11 defaults; lives in
/// `FpsCombatConfig::enemy_reaction` (RON-overridable, `#[serde(default)]`).
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, bevy::prelude::Reflect)]
#[serde(default)]
pub struct EnemyReactionConfig {
    /// Accumulated poise damage that triggers a hit-stun.
    pub poise_threshold: f32,
    /// Drain rate of accumulated poise damage once the regen delay expires.
    pub poise_regen_per_sec: f32,
    /// Delay after the most recent hit before poise starts draining.
    pub poise_regen_delay_secs: f32,
    /// Hit-stun duration when the threshold is crossed.
    pub hit_stun_secs: f32,
}

impl Default for EnemyReactionConfig {
    fn default() -> Self {
        Self {
            poise_threshold: 100.0,
            poise_regen_per_sec: 50.0,
            poise_regen_delay_secs: 1.5,
            hit_stun_secs: 0.4,
        }
    }
}

impl EnemyReactionConfig {
    #[must_use]
    fn sanitized(self) -> Self {
        let default = Self::default();
        Self {
            poise_threshold: if self.poise_threshold.is_finite()
                && self.poise_threshold > f32::EPSILON
            {
                self.poise_threshold
            } else {
                default.poise_threshold
            },
            poise_regen_per_sec: if self.poise_regen_per_sec.is_finite() {
                self.poise_regen_per_sec.max(0.0)
            } else {
                0.0
            },
            poise_regen_delay_secs: if self.poise_regen_delay_secs.is_finite() {
                self.poise_regen_delay_secs.max(0.0)
            } else {
                0.0
            },
            hit_stun_secs: if self.hit_stun_secs.is_finite() {
                self.hit_stun_secs.max(0.0)
            } else {
                default.hit_stun_secs
            },
        }
    }
}

/// Weapon-authored reaction contribution (weapon-only in Phase 11 — no part
/// modifiers yet).
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, bevy::prelude::Reflect)]
#[serde(default)]
pub struct WeaponReactionProfile {
    /// Poise damage per hit.
    pub poise_damage: f32,
    /// Total knockback displacement per hit (map units).
    pub knockback_distance: f32,
    /// Knockback decay duration (seconds).
    pub knockback_duration: f32,
}

impl Default for WeaponReactionProfile {
    fn default() -> Self {
        Self::NONE
    }
}

impl WeaponReactionProfile {
    /// No reaction at all (e.g. the flamethrower, which must not feed poise).
    pub const NONE: Self = Self {
        poise_damage: 0.0,
        knockback_distance: 0.0,
        knockback_duration: 0.0,
    };
}

/// Bundled reaction tuning: enemy-side poise rules + per-weapon profiles.
///
/// Provisional defaults (documented, tune in `combat.ron` later):
/// pistol = light poise chip with a tiny nudge; melee = heavier poise and a
/// meaningful shove; flamethrower has **no** profile by design.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, bevy::prelude::Reflect)]
#[serde(default)]
pub struct EnemyReactionTuning {
    pub enemy: EnemyReactionConfig,
    pub pistol: WeaponReactionProfile,
    pub melee: WeaponReactionProfile,
}

impl Default for EnemyReactionTuning {
    fn default() -> Self {
        Self {
            enemy: EnemyReactionConfig::default(),
            // 4 pistol shots to stagger; barely perceptible nudge.
            pistol: WeaponReactionProfile {
                poise_damage: 25.0,
                knockback_distance: 0.05,
                knockback_duration: 0.12,
            },
            // 2 melee hits to stagger; a real shove.
            melee: WeaponReactionProfile {
                poise_damage: 60.0,
                knockback_distance: 0.3,
                knockback_duration: 0.18,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Pending hit (hit site → sim transport)
// ---------------------------------------------------------------------------

/// A hit's reaction payload, written by a hit site and consumed by the enemy
/// sim on its next tick. Multiple hits in one tick merge via
/// [`EnemyReactionState::queue_hit`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PendingHitReaction {
    /// Knockback direction (shot travel direction), normalized or zero.
    pub direction: Vec2,
    /// Poise damage of this hit.
    pub poise_damage: f32,
    /// Total knockback displacement (map units).
    pub knockback_distance: f32,
    /// Knockback decay duration (seconds).
    pub knockback_duration: f32,
}

impl PendingHitReaction {
    /// Build a pending reaction from a weapon profile and the shot direction.
    ///
    /// Fail-safe: non-finite or negative profile values are clamped to zero
    /// (no reaction) so bad config can never produce NaN positions or
    /// negative poise.
    #[must_use]
    pub fn from_profile(profile: &WeaponReactionProfile, direction: Vec2) -> Self {
        Self::from_profiles(profile, PartReactionProfile::NEUTRAL, direction)
    }

    /// Build a pending reaction from the weapon profile **and** the hit part's
    /// reaction profile (Phase 12): `resolved = weapon × part`.
    ///
    /// Poise damage is `weapon.poise_damage × part.poise_scale`; knockback
    /// distance is `weapon.knockback_distance × part.knockback_scale`. Knockback
    /// *duration* is a weapon property and is not part-scaled.
    ///
    /// Fail-safe: non-finite or negative **weapon** values clamp to zero (no
    /// reaction); a non-finite or negative **part scale** clamps to the neutral
    /// `1.0` (so bad part data degrades to weapon-only, never to NaN/negative).
    /// With [`PartReactionProfile::NEUTRAL`] this is identical to the Phase 11
    /// weapon-only result.
    #[must_use]
    pub fn from_profiles(
        profile: &WeaponReactionProfile,
        part: PartReactionProfile,
        direction: Vec2,
    ) -> Self {
        let sanitize = |v: f32| if v.is_finite() { v.max(0.0) } else { 0.0 };
        // A part scale of `0.0` is a *valid* authored value (e.g. an armour
        // plate that grants no poise). Only invalid data — negative or
        // non-finite — degrades to the neutral `1.0` (weapon-only), never to a
        // reaction-killing `0.0`.
        let scale = |v: f32| if v.is_finite() && v >= 0.0 { v } else { 1.0 };
        Self {
            direction: direction.normalize_or_zero(),
            poise_damage: sanitize(profile.poise_damage) * scale(part.poise_scale),
            knockback_distance: sanitize(profile.knockback_distance) * scale(part.knockback_scale),
            knockback_duration: sanitize(profile.knockback_duration),
        }
    }

    /// Merge another same-tick hit into this one: poise damage accumulates;
    /// knockback displacements combine as a vector sum (two opposing shots
    /// cancel); duration takes the longer of the two.
    pub fn merge(&mut self, other: Self) {
        self.poise_damage += other.poise_damage;
        let sum =
            self.direction * self.knockback_distance + other.direction * other.knockback_distance;
        let dist = sum.length();
        if dist > f32::EPSILON {
            self.direction = sum / dist;
            self.knockback_distance = dist;
        } else {
            self.knockback_distance = 0.0;
        }
        self.knockback_duration = self.knockback_duration.max(other.knockback_duration);
    }

    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.poise_damage <= 0.0
            && (self.knockback_distance <= 0.0
                || self.knockback_duration <= 0.0
                || self.direction == Vec2::ZERO)
    }
}

// ---------------------------------------------------------------------------
// Runtime state (lives in the shared enemy sim)
// ---------------------------------------------------------------------------

/// Per-enemy reaction runtime state.
///
/// Carried by `MosquitonSim`/`SpideySim` and persisted across ticks by the
/// same wrappers that persist cooldowns (`ServerMosquitonSim` /
/// `ServerSpideySim` on the server, the `Mosquiton`/`Spidey` structs in
/// single-player). `Default` is the inert fresh state: full poise (zero
/// accumulated damage), no stun, no knockback, nothing pending.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct EnemyReactionState {
    /// Accumulated poise damage counting **up** toward
    /// [`EnemyReactionConfig::poise_threshold`]. `0.0` = full poise. Stored as
    /// an accumulator (rather than a remaining-poise meter) so the zero
    /// default is correct without config access.
    pub poise_damage: f32,
    /// Remaining delay before poise regen resumes.
    pub regen_delay_remaining: f32,
    /// Remaining hit-stun. While positive the sim must not start new actions.
    pub hit_stun_remaining: f32,
    /// Active knockback impulse, integrated by the sim via `try_move`.
    pub knockback: Option<OccupancyImpulse>,
    /// Hit queued by a hit site, consumed on the next sim tick.
    pub pending: Option<PendingHitReaction>,
    /// Hit queued by a pre-sim hit site, promoted after the current sim tick.
    ///
    /// Single-player combat currently runs before the enemy sim in the same
    /// frame, while the server enemy sim runs before combat. Local combat uses
    /// this slot so hits written by SP combat match the server's next-tick
    /// consumption timing.
    pub pending_next: Option<PendingHitReaction>,
}

impl EnemyReactionState {
    /// Queue a hit from a hit site. Same-tick hits merge.
    pub fn queue_hit(&mut self, hit: PendingHitReaction) {
        merge_pending_hit(&mut self.pending, hit);
    }

    /// Queue a hit from combat that runs before the current sim tick.
    ///
    /// The hit is not consumed by the immediately following [`Self::tick`];
    /// it is promoted afterward and consumed on the next tick. This preserves
    /// server/SP timing parity without moving flamethrower/burn processing.
    pub fn queue_hit_after_current_tick(&mut self, hit: PendingHitReaction) {
        merge_pending_hit(&mut self.pending_next, hit);
    }

    /// Whether the enemy is currently hit-stunned (AI must not start new
    /// actions; committed actions still complete).
    #[must_use]
    pub fn is_stunned(&self) -> bool {
        self.hit_stun_remaining > 0.0
    }

    /// Drop all transient reaction effects (pending hit, knockback, stun).
    /// Called by the sim for dead/dying enemies so corpses never react.
    pub fn clear(&mut self) {
        self.pending = None;
        self.pending_next = None;
        self.knockback = None;
        self.hit_stun_remaining = 0.0;
    }

    /// Advance the reaction state by `dt`: consume any pending hit, tick
    /// stun and poise regen, and return this frame's knockback displacement.
    ///
    /// The caller (the enemy sim) must apply the returned displacement through
    /// wall-aware movement (`try_move`) with its own collision radius.
    #[must_use]
    pub fn tick(&mut self, cfg: &EnemyReactionConfig, dt: f32) -> Vec2 {
        let cfg = cfg.sanitized();
        // 1. Consume the hit queued since the last sim tick.
        if let Some(hit) = self.pending.take() {
            if hit.poise_damage > 0.0 {
                self.poise_damage += hit.poise_damage;
                self.regen_delay_remaining = cfg.poise_regen_delay_secs;
                if self.poise_damage >= cfg.poise_threshold {
                    self.hit_stun_remaining = cfg.hit_stun_secs;
                    // Full reset: the next stun requires depleting a full bar
                    // again — this is the anti-stunlock core.
                    self.poise_damage = 0.0;
                }
            }
            if hit.knockback_distance > 0.0
                && hit.knockback_duration > 0.0
                && hit.direction != Vec2::ZERO
            {
                // Linear decay integrates to strength * duration / 2, so this
                // strength yields exactly `knockback_distance` total travel
                // (before wall clamping). Latest hit replaces any prior
                // impulse; same-tick hits already merged in `queue_hit`.
                self.knockback = Some(OccupancyImpulse {
                    direction: hit.direction,
                    strength: 2.0 * hit.knockback_distance / hit.knockback_duration,
                    remaining: hit.knockback_duration,
                    duration: hit.knockback_duration,
                });
            }
        }

        // 2. Tick stun.
        self.hit_stun_remaining = (self.hit_stun_remaining - dt).max(0.0);

        // 3. Poise regen after delay.
        if self.regen_delay_remaining > 0.0 {
            self.regen_delay_remaining = (self.regen_delay_remaining - dt).max(0.0);
        } else if self.poise_damage > 0.0 {
            self.poise_damage = cfg
                .poise_regen_per_sec
                .mul_add(-dt, self.poise_damage)
                .max(0.0);
        }

        // 4. Knockback displacement for this frame.
        match &mut self.knockback {
            Some(impulse) => {
                let displacement = impulse.tick(dt);
                if impulse.is_expired() {
                    self.knockback = None;
                }
                self.promote_deferred_hits();
                displacement
            }
            None => {
                self.promote_deferred_hits();
                Vec2::ZERO
            }
        }
    }

    fn promote_deferred_hits(&mut self) {
        if let Some(hit) = self.pending_next.take() {
            merge_pending_hit(&mut self.pending, hit);
        }
    }
}

fn merge_pending_hit(slot: &mut Option<PendingHitReaction>, hit: PendingHitReaction) {
    if hit.is_noop() {
        return;
    }
    match slot {
        Some(pending) => pending.merge(hit),
        None => *slot = Some(hit),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn cfg() -> EnemyReactionConfig {
        EnemyReactionConfig {
            poise_threshold: 100.0,
            poise_regen_per_sec: 50.0,
            poise_regen_delay_secs: 1.0,
            hit_stun_secs: 0.4,
        }
    }

    fn hit(poise: f32) -> PendingHitReaction {
        PendingHitReaction {
            direction: Vec2::X,
            poise_damage: poise,
            knockback_distance: 0.0,
            knockback_duration: 0.0,
        }
    }

    #[test]
    fn poise_below_threshold_does_not_stun() {
        let mut state = EnemyReactionState::default();
        state.queue_hit(hit(25.0));
        let _ = state.tick(&cfg(), 1.0 / 30.0);
        assert!(!state.is_stunned());
        assert!(state.poise_damage > 0.0, "poise damage accumulated");
    }

    #[test]
    fn repeated_hits_cross_threshold_and_stun_then_reset() {
        let mut state = EnemyReactionState::default();
        let cfg = cfg();
        for i in 0..4 {
            state.queue_hit(hit(25.0));
            let _ = state.tick(&cfg, 0.001);
            if i < 3 {
                assert!(!state.is_stunned(), "hit {i} must not stun yet");
            }
        }
        assert!(state.is_stunned(), "4th hit (100 poise) stuns");
        assert_eq!(state.poise_damage, 0.0, "poise resets on stun");

        // A 5th hit immediately after must NOT trivially re-stun: the bar was
        // reset, so it needs a full 100 again (anti-stunlock).
        state.queue_hit(hit(25.0));
        let _ = state.tick(&cfg, 0.001);
        assert!(state.poise_damage > 0.0 && state.poise_damage < cfg.poise_threshold);
    }

    #[test]
    fn poise_regenerates_after_delay() {
        let mut state = EnemyReactionState::default();
        let cfg = cfg();
        state.queue_hit(hit(50.0));
        let _ = state.tick(&cfg, 0.001);
        let accumulated = state.poise_damage;
        assert!(accumulated > 49.0);

        // During the regen delay, poise does not drain.
        let _ = state.tick(&cfg, 0.5);
        assert_eq!(state.poise_damage, accumulated, "no drain during delay");

        // Past the delay, poise drains toward zero.
        let _ = state.tick(&cfg, 0.6); // finishes delay (1.0s total elapsed)
        let _ = state.tick(&cfg, 0.5); // drains 25
        assert!(state.poise_damage < accumulated);
        let _ = state.tick(&cfg, 10.0);
        assert_eq!(state.poise_damage, 0.0, "fully regenerated");
    }

    #[test]
    fn stun_expires_after_duration() {
        let mut state = EnemyReactionState::default();
        let cfg = cfg();
        state.queue_hit(hit(150.0));
        let _ = state.tick(&cfg, 0.001);
        assert!(state.is_stunned());
        let _ = state.tick(&cfg, cfg.hit_stun_secs + 0.01);
        assert!(!state.is_stunned());
    }

    #[test]
    fn knockback_displacement_totals_distance() {
        let mut state = EnemyReactionState::default();
        let cfg = cfg();
        state.queue_hit(PendingHitReaction {
            direction: Vec2::X,
            poise_damage: 0.0,
            knockback_distance: 0.3,
            knockback_duration: 0.18,
        });
        let mut total = Vec2::ZERO;
        for _ in 0..30 {
            total += state.tick(&cfg, 0.018);
        }
        assert!(state.knockback.is_none(), "impulse expired");
        // Discrete forward integration of the linear decay overshoots by
        // (n+1)/n (~10% at 10 ticks of 0.018s), so accept that band.
        assert!(
            total.x >= 0.29 && total.x <= 0.35,
            "integrated displacement ≈ authored distance: {total:?}"
        );
        assert!(total.y.abs() < 1e-4);
    }

    #[test]
    fn same_tick_hits_merge() {
        let mut state = EnemyReactionState::default();
        state.queue_hit(PendingHitReaction {
            direction: Vec2::X,
            poise_damage: 30.0,
            knockback_distance: 0.2,
            knockback_duration: 0.1,
        });
        state.queue_hit(PendingHitReaction {
            direction: Vec2::NEG_X,
            poise_damage: 40.0,
            knockback_distance: 0.2,
            knockback_duration: 0.2,
        });
        let p = state.pending.unwrap();
        assert_eq!(p.poise_damage, 70.0, "poise sums");
        assert_eq!(p.knockback_distance, 0.0, "opposed knockback cancels");
        assert_eq!(p.knockback_duration, 0.2, "max duration kept");
    }

    #[test]
    fn deferred_hit_promotes_after_current_tick() {
        let mut state = EnemyReactionState::default();
        let cfg = cfg();
        state.queue_hit_after_current_tick(hit(150.0));

        let _ = state.tick(&cfg, 0.001);
        assert!(
            !state.is_stunned(),
            "pre-sim combat hit must not stun during the current tick"
        );
        assert!(
            state.pending.is_some(),
            "deferred hit promoted for next tick"
        );
        assert!(state.pending_next.is_none());

        let _ = state.tick(&cfg, 0.001);
        assert!(
            state.is_stunned(),
            "deferred hit consumed on following tick"
        );
    }

    #[test]
    fn from_profile_sanitizes_bad_values() {
        let profile = WeaponReactionProfile {
            poise_damage: f32::NAN,
            knockback_distance: -1.0,
            knockback_duration: f32::INFINITY,
        };
        let p = PendingHitReaction::from_profile(&profile, Vec2::new(3.0, 0.0));
        assert_eq!(p.poise_damage, 0.0);
        assert_eq!(p.knockback_distance, 0.0);
        assert_eq!(p.knockback_duration, 0.0);
        assert_eq!(p.direction, Vec2::X, "direction normalized");
    }

    #[test]
    fn tick_sanitizes_bad_config_values() {
        let bad_cfg = EnemyReactionConfig {
            poise_threshold: f32::NAN,
            poise_regen_per_sec: f32::INFINITY,
            poise_regen_delay_secs: -5.0,
            hit_stun_secs: f32::NEG_INFINITY,
        };
        let mut state = EnemyReactionState::default();
        state.queue_hit(hit(150.0));

        let _ = state.tick(&bad_cfg, 0.001);

        assert!(
            state.is_stunned(),
            "non-finite stun duration falls back to default"
        );
        assert_eq!(
            state.poise_damage, 0.0,
            "NaN threshold falls back to default and resets on threshold crossing"
        );
        assert!(state.regen_delay_remaining.is_finite());
        assert!(state.hit_stun_remaining.is_finite());
    }

    #[test]
    fn negative_stun_duration_clamps_to_zero() {
        let bad_cfg = EnemyReactionConfig {
            hit_stun_secs: -1.0,
            ..cfg()
        };
        let mut state = EnemyReactionState::default();
        state.queue_hit(hit(150.0));

        let _ = state.tick(&bad_cfg, 0.001);

        assert!(
            !state.is_stunned(),
            "finite negative stun duration clamps to zero"
        );
        assert!(state.hit_stun_remaining.is_finite());
    }

    #[test]
    fn clear_drops_transient_effects() {
        let mut state = EnemyReactionState::default();
        let cfg = cfg();
        state.queue_hit(PendingHitReaction {
            direction: Vec2::X,
            poise_damage: 150.0,
            knockback_distance: 0.3,
            knockback_duration: 0.2,
        });
        let _ = state.tick(&cfg, 0.001);
        assert!(state.is_stunned());
        assert!(state.knockback.is_some());
        state.queue_hit(hit(10.0));
        state.queue_hit_after_current_tick(hit(10.0));
        state.clear();
        assert!(!state.is_stunned());
        assert!(state.knockback.is_none());
        assert!(state.pending.is_none());
        assert!(state.pending_next.is_none());
    }

    // -----------------------------------------------------------------------
    // Phase 12: weapon profile × part reaction profile
    // -----------------------------------------------------------------------

    fn pistol() -> WeaponReactionProfile {
        // Stable local fixture (independent of tuning defaults): 25 poise, a
        // small nudge.
        WeaponReactionProfile {
            poise_damage: 25.0,
            knockback_distance: 0.08,
            knockback_duration: 0.12,
        }
    }

    fn part(poise_scale: f32, knockback_scale: f32) -> PartReactionProfile {
        PartReactionProfile {
            poise_scale,
            knockback_scale,
        }
    }

    /// 1. Default parity: a neutral part profile reproduces the Phase 11
    ///    weapon-only result exactly (this is what every shipped part uses).
    #[test]
    fn neutral_part_profile_matches_weapon_only() {
        let dir = Vec2::X;
        let weapon = pistol();
        let weapon_only = PendingHitReaction::from_profile(&weapon, dir);
        let with_neutral =
            PendingHitReaction::from_profiles(&weapon, PartReactionProfile::NEUTRAL, dir);
        assert_eq!(
            weapon_only, with_neutral,
            "NEUTRAL part ⇒ identical reaction"
        );
        // And the default `PartMetadata`/`PartReactionProfile` IS neutral.
        assert_eq!(PartReactionProfile::default(), PartReactionProfile::NEUTRAL);
        assert_eq!(with_neutral.poise_damage, 25.0);
        assert_eq!(with_neutral.knockback_distance, 0.08);
    }

    /// 2. Amplified part profile: more poise pressure ⇒ stun crosses sooner.
    #[test]
    fn amplified_part_profile_increases_poise_pressure() {
        let dir = Vec2::X;
        let weapon = pistol(); // 25 poise
        let amplified = PendingHitReaction::from_profiles(&weapon, part(2.0, 1.0), dir);
        assert_eq!(amplified.poise_damage, 50.0, "poise ×2.0");

        // Neutral: 4 hits (4×25=100) to reach threshold. Amplified (×2 ⇒ 50):
        // 2 hits. Verify amplified stuns on the 2nd hit, neutral does not.
        let cfg = cfg();
        let mut amp = EnemyReactionState::default();
        amp.queue_hit(amplified);
        let _ = amp.tick(&cfg, 0.001);
        assert!(!amp.is_stunned(), "1 amplified hit (50) below threshold");
        amp.queue_hit(amplified);
        let _ = amp.tick(&cfg, 0.001);
        assert!(amp.is_stunned(), "2 amplified hits (100) stun");

        let mut neu = EnemyReactionState::default();
        for _ in 0..2 {
            neu.queue_hit(PendingHitReaction::from_profiles(
                &weapon,
                PartReactionProfile::NEUTRAL,
                dir,
            ));
            let _ = neu.tick(&cfg, 0.001);
        }
        assert!(!neu.is_stunned(), "2 neutral hits (50) do not stun");
    }

    /// 3. Reduced part profile: less poise pressure ⇒ stun takes longer.
    #[test]
    fn reduced_part_profile_decreases_poise_pressure() {
        let weapon = pistol(); // 25 poise
        let reduced = PendingHitReaction::from_profiles(&weapon, part(0.5, 1.0), Vec2::X);
        assert_eq!(reduced.poise_damage, 12.5, "poise ×0.5");

        // Neutral stuns in 4 hits; reduced (12.5) needs 8.
        let cfg = cfg();
        let mut state = EnemyReactionState::default();
        for _ in 0..7 {
            state.queue_hit(reduced);
            let _ = state.tick(&cfg, 0.001);
        }
        assert!(!state.is_stunned(), "7 reduced hits (87.5) below threshold");
        state.queue_hit(reduced);
        let _ = state.tick(&cfg, 0.001);
        assert!(state.is_stunned(), "8th reduced hit (100) stuns");
    }

    /// 4. Knockback modifier: the part scales the resulting impulse.
    #[test]
    fn part_profile_scales_knockback_impulse() {
        let weapon = pistol(); // 0.08 distance
        let strong = PendingHitReaction::from_profiles(&weapon, part(1.0, 1.25), Vec2::X);
        let weak = PendingHitReaction::from_profiles(&weapon, part(1.0, 0.25), Vec2::X);
        assert!(
            (strong.knockback_distance - 0.10).abs() < 1e-6,
            "0.08 ×1.25"
        );
        assert!((weak.knockback_distance - 0.02).abs() < 1e-6, "0.08 ×0.25");

        let cfg = cfg();
        let integrate = |hit: PendingHitReaction| {
            let mut s = EnemyReactionState::default();
            s.queue_hit(hit);
            let mut total = Vec2::ZERO;
            for _ in 0..30 {
                total += s.tick(&cfg, 0.018);
            }
            total.x
        };
        let strong_disp = integrate(strong);
        let weak_disp = integrate(weak);
        assert!(
            strong_disp > weak_disp * 4.0,
            "stronger knockback scale ⇒ larger displacement: {strong_disp} vs {weak_disp}"
        );
    }

    /// 5. Lethal hit: even an amplified pending reaction is dropped on death.
    #[test]
    fn lethal_clears_amplified_pending_reaction() {
        let weapon = pistol();
        let amplified = PendingHitReaction::from_profiles(&weapon, part(2.0, 2.0), Vec2::X);
        let mut state = EnemyReactionState::default();
        state.queue_hit(amplified);
        state.queue_hit_after_current_tick(amplified);
        // Lethal hit ⇒ sim calls clear().
        state.clear();
        assert!(state.pending.is_none());
        assert!(state.pending_next.is_none());
        assert!(state.knockback.is_none());
        assert!(!state.is_stunned());
    }

    /// 6. SP/server parity: the resolver is the single shared function both
    ///    authorities call (SP `apply_hitscan_damage`, server `process_combat`),
    ///    so identical (weapon, part, direction) ⇒ identical resolved reaction.
    #[test]
    fn resolved_reaction_is_authority_independent() {
        let weapon = pistol();
        let p = part(1.5, 0.75);
        let dir = Vec2::new(0.6, -0.8); // arbitrary, non-axis
        let sp = PendingHitReaction::from_profiles(&weapon, p, dir);
        let server = PendingHitReaction::from_profiles(&weapon, p, dir);
        assert_eq!(
            sp, server,
            "shared resolver ⇒ bit-identical on both authorities"
        );
        // Hand-computed to pin the resolution rule.
        assert!((sp.poise_damage - 25.0 * 1.5).abs() < 1e-6);
        assert!((sp.knockback_distance - 0.08 * 0.75).abs() < 1e-6);
        assert_eq!(
            sp.knockback_duration, weapon.knockback_duration,
            "duration not part-scaled"
        );
    }

    /// 7. Anti-stunlock survives part amplification: after a stun the bar fully
    ///    resets, so even a high-poise part cannot chain-stun on the next hit.
    #[test]
    fn amplified_hits_preserve_anti_stunlock() {
        let weapon = pistol();
        // ×2 ⇒ 50 poise/hit. Two hits stun (100), bar resets to 0.
        let amplified = PendingHitReaction::from_profiles(&weapon, part(2.0, 1.0), Vec2::X);
        let cfg = cfg();
        let mut state = EnemyReactionState::default();
        state.queue_hit(amplified);
        let _ = state.tick(&cfg, 0.001);
        state.queue_hit(amplified);
        let _ = state.tick(&cfg, 0.001);
        assert!(state.is_stunned(), "two amplified hits stun");
        assert_eq!(state.poise_damage, 0.0, "bar resets on stun");

        // Immediately after the reset, a single amplified hit (50) must NOT
        // re-stun — a fresh full 100 is required (anti-stunlock holds).
        state.queue_hit(amplified);
        let _ = state.tick(&cfg, 0.001);
        assert!(
            state.poise_damage > 0.0 && state.poise_damage < cfg.poise_threshold,
            "post-reset hit accumulates without trivially re-stunning"
        );
    }

    /// Fail-safe: bad part scales degrade to weapon-only (neutral), never NaN.
    #[test]
    fn non_finite_part_scale_degrades_to_neutral() {
        let weapon = pistol();
        for bad in [f32::NAN, f32::INFINITY, -1.0] {
            let r = PendingHitReaction::from_profiles(&weapon, part(bad, bad), Vec2::X);
            assert_eq!(
                r.poise_damage, weapon.poise_damage,
                "bad poise scale ⇒ ×1.0"
            );
            assert_eq!(
                r.knockback_distance, weapon.knockback_distance,
                "bad knockback scale ⇒ ×1.0"
            );
            assert!(r.poise_damage.is_finite() && r.knockback_distance.is_finite());
        }
    }
}
