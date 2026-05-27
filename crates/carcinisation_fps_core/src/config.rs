//! Shared FPS gameplay constants.
//!
//! Single source of truth for all gameplay tuning values used by both
//! singleplayer and multiplayer. Server and client import from here.
//!
//! When adding a new weapon, enemy type, or mechanic — add its constants here
//! so both SP and MP automatically use the same values.

use bevy::prelude::ReflectResource;
use carapace::constrained::{FiniteF32, PositiveFiniteF32};
use std::num::{NonZeroU64, NonZeroUsize};

/// Hot-reloadable FPS movement tuning.
///
/// Loaded from `assets/config/fp/movement.ron`.
/// Used by both singleplayer and multiplayer (client + server).
#[derive(
    Clone, Copy, Debug, serde::Deserialize, bevy::prelude::Resource, bevy::prelude::Reflect,
)]
#[reflect(Resource)]
#[serde(rename = "FpsMovementConfig")]
pub struct FpsMovementConfig {
    /// Movement speed in map-units per second.
    pub move_speed: f32,
    /// Turn speed in radians per second.
    pub turn_speed: f32,
    /// Collision margin in map-units (distance kept from walls).
    pub collision_margin: f32,
    /// Duration of a 180° quick turn in seconds. 90° turns complete in half this time.
    pub quick_turn_duration_secs: f32,
}

impl FpsMovementConfig {
    #[must_use]
    pub fn load() -> Self {
        carcinisation_core::ron_config!("assets/config/fp/movement.ron")
    }
}

impl Default for FpsMovementConfig {
    fn default() -> Self {
        Self {
            move_speed: 2.0,
            turn_speed: 2.0,
            collision_margin: 0.2,
            quick_turn_duration_secs: 0.4,
        }
    }
}

/// Shared flamethrower gameplay and stream tuning.
///
/// Loaded from `assets/config/attacks/player_flamethrower.ron`.
/// Used by both singleplayer and multiplayer (client + server).
#[derive(
    Clone, Copy, Debug, serde::Deserialize, bevy::prelude::Resource, bevy::prelude::Reflect,
)]
#[reflect(Resource)]
#[serde(rename = "PlayerFlamethrowerConfig")]
pub struct PlayerFlamethrowerConfig {
    /// Maximum distance a flame can reach from the player (world units).
    /// Controls both the damage hitbox range and the visual stream lifetime.
    pub range: f32,
    /// Half-width of the flame damage line (world units).
    /// The server checks perpendicular distance from the flame centre-line;
    /// targets within this distance are considered hit.
    pub hit_half_width: f32,
    /// Travel speed of flame stream samples (world units per second).
    /// Shared between 1P and 3P visual rendering.
    pub speed: f32,
    /// Minimum interval between flame sample emissions (milliseconds).
    /// Lower values produce a denser stream. Shared between 1P and 3P.
    pub emit_interval_ms: NonZeroU64,
    /// Damage applied per flamethrower tick while a target is in the flame.
    pub damage_per_tick: u32,
    /// Interval between damage ticks (milliseconds).
    pub tick_ms: NonZeroU64,
    /// Maximum ammo pool. Ammo drains continuously while firing.
    pub max_ammo: f32,
    /// Ammo consumed per millisecond of sustained fire.
    pub ammo_drain_per_ms: f32,
    /// How long a burning corpse persists before despawning (seconds).
    pub burning_corpse_duration_secs: f32,
    /// Damage dealt to the player per contact tick when touching a burning corpse.
    pub burning_corpse_contact_damage: u32,
    /// Interval between burning-corpse contact damage ticks (milliseconds).
    pub burning_corpse_contact_tick_ms: NonZeroU64,
    /// Radius around a burning corpse that triggers contact damage (world units).
    pub burning_corpse_contact_radius: f32,
    /// Damage applied to nearby enemies each tick by a burning corpse (crossfire).
    pub burning_corpse_crossfire_damage: u32,
    /// Number of flame sprites placed around a burning corpse perimeter.
    pub burning_flame_count: NonZeroUsize,
    /// Inward padding from the sprite edge when placing perimeter flames (pixels).
    pub burning_flame_perimeter_padding_px: f32,
    /// Random positional jitter applied to each perimeter flame (pixels).
    pub burning_flame_jitter_px: f32,
    /// Minimum random scale multiplier for perimeter flames.
    pub burning_flame_scale_min: f32,
    /// Maximum random scale multiplier for perimeter flames.
    pub burning_flame_scale_max: f32,
}

impl PlayerFlamethrowerConfig {
    #[must_use]
    pub fn load() -> Self {
        carcinisation_core::ron_config!("assets/config/attacks/player_flamethrower.ron")
    }

    /// Maximum age of a flame sample before it expires (seconds).
    #[must_use]
    pub fn max_stream_age(&self) -> f32 {
        self.range / self.speed
    }

    /// Build a `FireDeathConfig` from the combat config values.
    #[must_use]
    pub const fn fire_death_config(&self) -> crate::fire_death::FireDeathConfig {
        crate::fire_death::FireDeathConfig {
            burning_corpse_duration_secs: self.burning_corpse_duration_secs,
            burning_flame_count: self.burning_flame_count,
            burning_flame_perimeter_padding_px: self.burning_flame_perimeter_padding_px,
            burning_flame_jitter_px: self.burning_flame_jitter_px,
            burning_flame_scale_min: self.burning_flame_scale_min,
            burning_flame_scale_max: self.burning_flame_scale_max,
        }
    }

    #[must_use]
    pub const fn burning_corpse_contact_tick_secs(&self) -> f32 {
        std::time::Duration::from_millis(self.burning_corpse_contact_tick_ms.get()).as_secs_f32()
    }
}

/// Spidey enemy combat tuning — extracted from [`FpsCombatConfig`] to reduce
/// per-field serde default boilerplate.
#[derive(Clone, Copy, Debug, serde::Deserialize, bevy::prelude::Reflect)]
pub struct SpideyCombatConfig {
    pub move_speed: f32,
    pub collision_radius: f32,
    pub aggro_range: f32,
    pub hop_interval_min: f32,
    pub hop_interval_max: f32,
    pub hop_distance: f32,
    pub hop_duration: f32,
    pub hop_visual_height: f32,
    pub lunge_range: f32,
    pub lunge_speed: f32,
    pub lunge_melee_damage: u32,
    pub lunge_windup_secs: f32,
    pub lunge_duration_secs: f32,
    pub lunge_cooldown: f32,
    pub web_range: f32,
    pub web_cooldown: f32,
    pub web_cue_secs: f32,
    pub web_projectile_speed: f32,
    pub web_projectile_damage: u32,
    pub health: u32,
    pub web_slow_multiplier: f32,
    pub web_slow_duration: f32,
    pub recover_secs: f32,
    pub death_secs: f32,
}

impl Default for SpideyCombatConfig {
    fn default() -> Self {
        Self {
            move_speed: 2.0,
            collision_radius: 0.25,
            aggro_range: 8.0,
            hop_interval_min: 0.4,
            hop_interval_max: 1.0,
            hop_distance: 1.2,
            hop_duration: 0.4,
            hop_visual_height: 0.3,
            lunge_range: 2.0,
            lunge_speed: 7.0,
            lunge_melee_damage: 20,
            lunge_windup_secs: 0.2,
            lunge_duration_secs: 0.7,
            lunge_cooldown: 3.0,
            web_range: 6.0,
            web_cooldown: 3.0,
            web_cue_secs: 1.0,
            web_projectile_speed: 3.0,
            web_projectile_damage: 5,
            health: 100,
            web_slow_multiplier: 0.7,
            web_slow_duration: 3.0,
            recover_secs: 0.5,
            death_secs: 0.6,
        }
    }
}

/// Hot-reloadable FPS combat tuning.
///
/// Loaded from `assets/config/fp/combat.ron`.
/// Used by both singleplayer and multiplayer (client + server).
#[derive(
    Clone, Copy, Debug, serde::Deserialize, bevy::prelude::Resource, bevy::prelude::Reflect,
)]
#[reflect(Resource)]
#[serde(rename = "FpsCombatConfig")]
pub struct FpsCombatConfig {
    // -- Pistol --
    /// Damage per hitscan shot.
    pub hitscan_damage: f32,
    /// Minimum seconds between pistol shots.
    pub fire_cooldown_secs: f32,
    // -- Flamethrower --
    /// Flamethrower damage per second (continuous while held).
    pub flame_dps: f32,
    // -- Enemy Projectiles --
    /// Enemy projectile speed in world-units per second.
    pub projectile_speed: f32,
    /// Enemy projectile collision radius.
    pub projectile_hit_radius: f32,
    /// Enemy projectile lifetime in seconds before auto-despawn.
    pub projectile_lifetime: f32,
    // -- Mosquiton --
    /// Seconds between Mosquiton ranged attacks (shoot cooldown).
    pub mosquiton_shoot_cooldown: f32,
    /// Seconds between Mosquiton melee attacks.
    pub mosquiton_melee_cooldown: f32,
    /// Duration of the melee attack animation in seconds.
    pub mosquiton_melee_attack_duration: f32,
    /// Damage per Mosquiton ranged projectile.
    pub mosquiton_projectile_damage: f32,
    /// Damage per Mosquiton melee hit.
    pub mosquiton_melee_damage: f32,
    /// Melee engagement range in world units.
    pub mosquiton_melee_range: f32,
    /// Maximum range at which a Mosquiton can fire ranged attacks.
    pub mosquiton_shoot_range: f32,
    /// Preferred engagement range — Mosquiton holds at this distance.
    pub mosquiton_preferred_range: f32,
    /// Mosquiton blood-shot projectile speed in world-units per second.
    pub mosquiton_blood_shot_speed: f32,
    /// Mosquiton collision radius for wall avoidance.
    pub mosquiton_collision_radius: f32,
    /// Mosquiton default health.
    pub mosquiton_health: u32,
    /// Delay from shoot animation start to projectile spawn (seconds).
    pub mosquiton_shoot_cue_secs: f32,
    // -- Burn Contact --
    /// Radius for burning corpse contact damage.
    pub burn_contact_radius: f32,
    /// Damage per burn contact tick.
    pub burn_contact_damage: f32,
    /// Seconds between burn contact damage ticks.
    pub burn_contact_tick_secs: f32,
    // -- Death / Respawn --
    /// Duration of enemy death animation before transitioning to Dead state.
    pub enemy_death_anim_secs: f32,
    /// Seconds before a dead enemy entity is despawned.
    pub enemy_despawn_delay: f32,
    /// Seconds before a dead player respawns.
    pub player_respawn_delay_secs: f32,
    // -- Spidey --
    /// Spidey enemy combat tuning.
    #[serde(default)]
    pub spidey: SpideyCombatConfig,
    // -- Ground Fire --
    /// Lifetime of a ground fire hazard in seconds (full + fade phases).
    pub ground_fire_lifetime_secs: f32,
    /// Seconds at which the ground fire begins fading (half size, half damage).
    pub ground_fire_fade_start_secs: f32,
    /// Damage radius of ground fire in world units.
    pub ground_fire_radius: f32,
    /// Damage per ground fire contact tick.
    pub ground_fire_damage: f32,
    /// Seconds between ground fire contact damage ticks.
    pub ground_fire_tick_secs: f32,
    /// Visual spread radius for ground fire flame placement (world units).
    pub ground_fire_visual_radius: f32,
    /// Number of flame sprites per ground fire.
    pub ground_fire_flame_count: NonZeroUsize,
    /// Maximum number of ground fires that can exist simultaneously.
    pub ground_fire_max: usize,
}

impl FpsCombatConfig {
    #[must_use]
    pub fn load() -> Self {
        carcinisation_core::ron_config!("assets/config/fp/combat.ron")
    }

    /// Legacy alias — equivalent to `self.mosquiton_shoot_cooldown`.
    #[must_use]
    pub const fn mosquiton_attack_interval(&self) -> f32 {
        self.mosquiton_shoot_cooldown
    }

    /// Build a `MosquitonSimConfig` from the combat config values.
    #[must_use]
    pub const fn mosquiton_sim_config(&self) -> crate::mosquiton::MosquitonSimConfig {
        crate::mosquiton::MosquitonSimConfig {
            move_speed: 2.0,
            preferred_range: self.mosquiton_preferred_range,
            melee_range: self.mosquiton_melee_range,
            shoot_range: self.mosquiton_shoot_range,
            shoot_cooldown: self.mosquiton_shoot_cooldown,
            melee_cooldown: self.mosquiton_melee_cooldown,
            melee_attack_duration: self.mosquiton_melee_attack_duration,
            melee_damage: self.mosquiton_melee_damage as u32,
            blood_shot_speed: self.mosquiton_blood_shot_speed,
            blood_shot_damage: self.mosquiton_projectile_damage as u32,
            collision_radius: self.mosquiton_collision_radius,
            shoot_cue_secs: self.mosquiton_shoot_cue_secs,
        }
    }

    /// Build a `SpideySimConfig` from the combat config values.
    #[must_use]
    pub const fn spidey_sim_config(&self) -> crate::spidey::SpideySimConfig {
        let s = &self.spidey;
        crate::spidey::SpideySimConfig {
            move_speed: s.move_speed,
            collision_radius: s.collision_radius,
            aggro_range: s.aggro_range,
            hop_interval_min: s.hop_interval_min,
            hop_interval_max: s.hop_interval_max,
            hop_distance: s.hop_distance,
            hop_duration: s.hop_duration,
            hop_visual_height: s.hop_visual_height,
            lunge_range: s.lunge_range,
            lunge_speed: s.lunge_speed,
            lunge_melee_damage: s.lunge_melee_damage,
            lunge_windup_secs: s.lunge_windup_secs,
            lunge_duration_secs: s.lunge_duration_secs,
            lunge_cooldown: s.lunge_cooldown,
            web_range: s.web_range,
            web_cooldown: s.web_cooldown,
            web_cue_secs: s.web_cue_secs,
            web_projectile_speed: s.web_projectile_speed,
            web_projectile_damage: s.web_projectile_damage,
            recover_secs: s.recover_secs,
            death_secs: s.death_secs,
        }
    }

    /// Build a `GroundFireConfig` from the combat config values.
    #[must_use]
    pub const fn ground_fire_config(&self) -> crate::ground_fire::GroundFireConfig {
        crate::ground_fire::GroundFireConfig {
            lifetime_secs: self.ground_fire_lifetime_secs,
            fade_start_secs: self.ground_fire_fade_start_secs,
            radius: self.ground_fire_radius,
            damage_per_tick: self.ground_fire_damage,
            tick_secs: self.ground_fire_tick_secs,
            flame_count: self.ground_fire_flame_count,
            max_fires: self.ground_fire_max,
            visual_radius: self.ground_fire_visual_radius,
        }
    }
}

impl Default for FpsCombatConfig {
    fn default() -> Self {
        Self {
            hitscan_damage: 37.0,
            fire_cooldown_secs: 0.33,
            flame_dps: 580.0,
            projectile_speed: 4.0,
            projectile_hit_radius: 0.3,
            projectile_lifetime: 3.0,
            mosquiton_shoot_cooldown: 2.0,
            mosquiton_melee_cooldown: 2.0,
            mosquiton_melee_attack_duration: 0.6,
            mosquiton_projectile_damage: 10.0,
            mosquiton_melee_damage: 15.0,
            mosquiton_melee_range: 0.8,
            mosquiton_shoot_range: 8.0,
            mosquiton_preferred_range: 4.0,
            mosquiton_blood_shot_speed: 4.0,
            mosquiton_collision_radius: 0.3,
            mosquiton_health: 40,
            mosquiton_shoot_cue_secs: 1.0,
            spidey: SpideyCombatConfig::default(),
            burn_contact_radius: 1.1,
            burn_contact_damage: 5.0,
            burn_contact_tick_secs: 0.5,
            enemy_death_anim_secs: 0.5,
            enemy_despawn_delay: 5.0,
            player_respawn_delay_secs: 3.0,
            ground_fire_lifetime_secs: 15.0,
            ground_fire_fade_start_secs: 10.0,
            ground_fire_radius: 0.8,
            ground_fire_damage: 3.0,
            ground_fire_tick_secs: 0.5,
            ground_fire_visual_radius: 0.35,
            ground_fire_flame_count: NonZeroUsize::new(6).unwrap(),
            ground_fire_max: 32,
        }
    }
}

/// Hot-reloadable FPS visual tuning.
///
/// Loaded from `assets/config/fp/visuals.ron`.
/// Client-only: controls damage flicker presentation.
#[derive(
    Clone, Copy, Debug, serde::Deserialize, bevy::prelude::Resource, bevy::prelude::Reflect,
)]
#[reflect(Resource)]
#[serde(rename = "FpsVisualConfig")]
pub struct FpsVisualConfig {
    /// Number of invert cycles in a damage flicker effect.
    pub damage_flicker_count: u8,
    /// Duration of the regular (non-inverted) phase in seconds.
    pub damage_flicker_regular_secs: f32,
    /// Duration of the inverted phase in seconds.
    pub damage_flicker_invert_secs: f32,
    /// Amplitude of the camera view bob in pixels while walking.
    #[serde(default = "default_view_bob_amplitude")]
    pub view_bob_amplitude: f32,
    /// Frequency multiplier for view bob relative to weapon bob phase.
    #[serde(default = "default_view_bob_freq_mult")]
    pub view_bob_freq_mult: f32,
    /// Distance below which view bob is at full strength (map units).
    #[serde(default = "default_view_bob_near")]
    pub view_bob_near: f32,
    /// Distance above which view bob is at half strength (map units).
    /// Beyond 2x this distance, bob is zero.
    #[serde(default = "default_view_bob_mid")]
    pub view_bob_mid: f32,
}

impl FpsVisualConfig {
    #[must_use]
    pub fn load() -> Self {
        carcinisation_core::ron_config!("assets/config/fp/visuals.ron")
    }
}

const fn default_view_bob_amplitude() -> f32 {
    1.5
}
const fn default_view_bob_freq_mult() -> f32 {
    2.0
}
const fn default_view_bob_near() -> f32 {
    3.0
}
const fn default_view_bob_mid() -> f32 {
    6.0
}

impl Default for FpsVisualConfig {
    fn default() -> Self {
        Self {
            damage_flicker_count: 4,
            damage_flicker_regular_secs: 0.1,
            damage_flicker_invert_secs: 0.075,
            view_bob_amplitude: default_view_bob_amplitude(),
            view_bob_freq_mult: default_view_bob_freq_mult(),
            view_bob_near: default_view_bob_near(),
            view_bob_mid: default_view_bob_mid(),
        }
    }
}

/// A single size/behaviour tier for screen particles.
///
/// Tiers are selected by weighted random sampling in `choose_health_tier`.
/// Weights are normalised into probabilities internally.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename = "SizeTierConfig")]
pub struct SizeTierConfig {
    /// Base radius in pixels before scale multiplication (`.max(1)` after rounding).
    pub radius_px: f32,
    /// Speed multiplier — higher = faster upward impulse + upward deceleration.
    pub speed_scale: f32,
    /// Lifetime multiplier applied to the base lifetime range.
    pub life_scale: f32,
    /// Selection weight (higher = more likely).
    pub weight: f32,
    /// When `true`, particle always uses the highlight palette index.
    pub always_highlight: bool,
    /// Fraction of lifetime during which the highlight palette index is shown.
    /// Only meaningful when `always_highlight` is false and random highlight
    /// triggers; for `always_highlight` tiers this value is always used.
    pub highlight_window: f32,
}

/// Hot-reloadable FPS screen particle tuning.
///
/// Loaded from `assets/config/fp/screen_particles.ron`.
/// Controls health-pickup burst particles: count, size, lifetime, physics,
/// dither-fade, anti-cluster spawning, and palette indices.
#[derive(Clone, Debug, serde::Deserialize, bevy::prelude::Resource)]
#[serde(rename = "ScreenParticleConfig")]
pub struct ScreenParticleConfig {
    // -- Counts --
    /// Number of particles per burst.
    pub particle_count: NonZeroUsize,
    /// Maximum concurrent particles (FIFO eviction when exceeded).
    pub max_particles: NonZeroUsize,

    // -- Lifetime / Physics --
    /// Minimum particle lifetime in seconds.
    pub lifetime_min: PositiveFiniteF32,
    /// Maximum particle lifetime in seconds.
    pub lifetime_max: PositiveFiniteF32,
    /// Upward acceleration in pixels/s² (negative = up in screen coords).
    pub upward_accel: FiniteF32,
    /// Initial upward impulse in pixels/s (before `speed_scale`).
    pub pop_impulse: PositiveFiniteF32,
    /// Drag multiplier applied at 60 fps: `drag.powf(dt * 60)`.
    pub drag: f32,

    // -- Appearance --
    /// Probability of an extra-bright highlight variant per particle.
    pub highlight_chance: f32,
    /// Normalised age at which dither fade-in begins.
    /// Valid range: `0.0 ..= 1.0`.
    pub dither_fade_start: f32,
    /// Strength multiplier for the dither threshold (clamped to 0..16).
    pub dither_fade_strength: FiniteF32,
    /// Width/height aspect ratio for the diamond rasterisation shape.
    pub diamond_aspect: PositiveFiniteF32,

    // -- Spawn area / Anti-cluster --
    /// Bias exponent for peripheral spawn offset.
    /// Values below 1.0 push samples edgeward; values above 1.0 pull toward centre.
    pub spawn_periphery_bias: PositiveFiniteF32,
    /// Minimum distance between particle centres (in unscaled prototype pixels).
    pub min_spawn_distance: f32,
    /// Anti-cluster rejection attempts per particle.
    pub spawn_rejection_attempts: NonZeroUsize,
    /// Maximum dt for particle physics (clamped to avoid explosion on frame spike).
    pub max_particle_dt: PositiveFiniteF32,

    // -- Coordinate system --
    /// Reference framebuffer height used to scale particle sizes and physics.
    pub prototype_reference_height: PositiveFiniteF32,
    /// Spawn anchor X-coordinate as a fraction of framebuffer width (0..1).
    pub spawn_anchor_x: f32,
    /// Spawn anchor Y-coordinate as a fraction of framebuffer height (0..1).
    pub spawn_anchor_y: f32,
    /// Vertical jitter half-range as a fraction of framebuffer height.
    /// Total spread is 2 × `spawn_area_h` × height.
    pub spawn_area_h: f32,

    // -- Palette --
    /// Palette index for particle body (non-transparent, non-zero).
    pub palette_light: u8,
    /// Palette index for particle highlight (non-transparent, different from `palette_light`).
    pub palette_highlight: u8,

    // -- Size tiers --
    /// Weighted size/behaviour tiers. Must contain at least one entry.
    pub size_tiers: vec1::Vec1<SizeTierConfig>,
}

impl ScreenParticleConfig {
    #[must_use]
    pub fn load() -> Self {
        let config: Self = carcinisation_core::ron_config!("assets/config/fp/screen_particles.ron");
        config.validate_or_panic();
        config
    }

    /// Validate configuration invariants.
    ///
    /// Returns a list of validation error messages (empty = valid).
    /// At startup, embedded RON is expected to pass; if it fails validation
    /// the errors are logged for debugging.
    #[must_use]
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.max_particles.get() < self.particle_count.get() {
            errors.push("max_particles must be >= particle_count".into());
        }
        if self.lifetime_max.get() < self.lifetime_min.get() {
            errors.push("lifetime_max must be >= lifetime_min".into());
        }
        if !self.drag.is_finite() || self.drag <= 0.0 || self.drag > 1.0 {
            errors.push("drag must be in (0..1]".into());
        }
        if !is_finite_unit_inclusive(self.highlight_chance) {
            errors.push("highlight_chance must be in 0..=1".into());
        }
        if !self.dither_fade_start.is_finite()
            || self.dither_fade_start <= 0.0
            || self.dither_fade_start >= 1.0
        {
            errors.push("dither_fade_start must be in 0..1".into());
        }
        if self.dither_fade_strength.get() < 0.0 {
            errors.push("dither_fade_strength must be >= 0".into());
        }
        if !is_finite_non_negative(self.min_spawn_distance) {
            errors.push("min_spawn_distance must be >= 0".into());
        }
        if !is_finite_unit_inclusive(self.spawn_anchor_x) {
            errors.push("spawn_anchor_x must be in 0..=1".into());
        }
        if !is_finite_unit_inclusive(self.spawn_anchor_y) {
            errors.push("spawn_anchor_y must be in 0..=1".into());
        }
        if !is_finite_non_negative(self.spawn_area_h) {
            errors.push("spawn_area_h must be >= 0".into());
        }
        if self.palette_light == 0 {
            errors.push("palette_light must be non-zero (index 0 = transparent)".into());
        }
        if self.palette_highlight == 0 {
            errors.push("palette_highlight must be non-zero (index 0 = transparent)".into());
        }
        if self.palette_light == self.palette_highlight {
            errors.push("palette_light and palette_highlight must be different".into());
        }

        for (i, tier) in self.size_tiers.iter().enumerate() {
            if !is_finite_positive(tier.weight) {
                errors.push(format!("size_tiers[{i}].weight must be > 0"));
            }
            if !is_finite_positive(tier.radius_px) {
                errors.push(format!("size_tiers[{i}].radius_px must be > 0"));
            }
            if !is_finite_positive(tier.speed_scale) {
                errors.push(format!("size_tiers[{i}].speed_scale must be > 0"));
            }
            if !is_finite_positive(tier.life_scale) {
                errors.push(format!("size_tiers[{i}].life_scale must be > 0"));
            }
            if !tier.highlight_window.is_finite()
                || tier.highlight_window <= 0.0
                || tier.highlight_window > 1.0
            {
                errors.push(format!(
                    "size_tiers[{i}].highlight_window must be > 0 and <= 1"
                ));
            }
        }

        errors
    }

    /// Panic if the configuration is invalid.
    ///
    /// Startup uses this for fail-fast embedded config validation. Hot reload
    /// wraps this in `catch_unwind`, logs, and keeps the previous valid value.
    ///
    /// # Panics
    ///
    /// If any field violates its validation constraint.
    pub fn validate_or_panic(&self) {
        let errors = self.validate();
        assert!(
            errors.is_empty(),
            "invalid ScreenParticleConfig: {}",
            errors.join("; ")
        );
    }
}

fn is_finite_positive(value: f32) -> bool {
    value.is_finite() && value > 0.0
}

fn is_finite_non_negative(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

fn is_finite_unit_inclusive(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

impl Default for ScreenParticleConfig {
    fn default() -> Self {
        Self {
            particle_count: NonZeroUsize::new(16).unwrap(),
            max_particles: NonZeroUsize::new(128).unwrap(),
            lifetime_min: PositiveFiniteF32::new(0.70).unwrap(),
            lifetime_max: PositiveFiniteF32::new(1.05).unwrap(),
            upward_accel: FiniteF32::new(-90.0).unwrap(),
            pop_impulse: PositiveFiniteF32::new(30.0).unwrap(),
            drag: 0.97,
            highlight_chance: 0.05,
            dither_fade_start: 0.55,
            dither_fade_strength: FiniteF32::new(1.0).unwrap(),
            diamond_aspect: PositiveFiniteF32::new(0.55).unwrap(),
            spawn_periphery_bias: PositiveFiniteF32::new(0.7).unwrap(),
            min_spawn_distance: 14.0,
            spawn_rejection_attempts: NonZeroUsize::new(8).unwrap(),
            max_particle_dt: PositiveFiniteF32::new(0.05).unwrap(),
            prototype_reference_height: PositiveFiniteF32::new(180.0).unwrap(),
            spawn_anchor_x: 0.50,
            spawn_anchor_y: 0.55,
            spawn_area_h: 0.10,
            palette_light: 3,
            palette_highlight: 4,
            size_tiers: vec1::vec1![
                SizeTierConfig {
                    radius_px: 2.0,
                    speed_scale: 1.70,
                    life_scale: 1.00,
                    weight: 0.70,
                    always_highlight: false,
                    highlight_window: 0.55,
                },
                SizeTierConfig {
                    radius_px: 5.0,
                    speed_scale: 0.80,
                    life_scale: 1.00,
                    weight: 0.22,
                    always_highlight: false,
                    highlight_window: 0.55,
                },
                SizeTierConfig {
                    radius_px: 9.0,
                    speed_scale: 0.40,
                    life_scale: 0.70,
                    weight: 0.08,
                    always_highlight: true,
                    highlight_window: 0.65,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fps_configs_load() {
        let _ = FpsMovementConfig::load();
        let _ = PlayerFlamethrowerConfig::load();
        let _ = FpsCombatConfig::load();
        let _ = FpsVisualConfig::load();
    }

    #[test]
    fn screen_particle_config_default_validates_ok() {
        let config = ScreenParticleConfig::default();
        let errors = config.validate();
        assert!(
            errors.is_empty(),
            "default config should validate: {errors:?}"
        );
    }

    #[test]
    fn screen_particle_config_ron_parses() {
        let config = ScreenParticleConfig::load();
        let errors = config.validate();
        assert!(
            errors.is_empty(),
            "embedded RON should validate: {errors:?}"
        );
        assert_eq!(config.particle_count.get(), 16);
        assert_eq!(config.max_particles.get(), 128);
        assert!((config.lifetime_min.get() - 0.70).abs() < f32::EPSILON);
        assert!((config.lifetime_max.get() - 1.05).abs() < f32::EPSILON);
        assert_eq!(config.size_tiers.len(), 3);
    }

    #[test]
    fn screen_particle_config_validation_catches_bad_values() {
        let mut config = ScreenParticleConfig {
            palette_light: 0,
            palette_highlight: 0,
            spawn_anchor_x: 2.0,
            ..Default::default()
        };
        config.size_tiers[0].weight = -1.0;
        let errors = config.validate();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("weight")));
        assert!(errors.iter().any(|e| e.contains("palette_light")));
        assert!(errors.iter().any(|e| e.contains("palette_highlight")));
        assert!(errors.iter().any(|e| e.contains("spawn_anchor_x")));
    }

    fn assert_screen_particle_invalid(
        mut config: ScreenParticleConfig,
        mutate: impl FnOnce(&mut ScreenParticleConfig),
        expected: &str,
    ) {
        mutate(&mut config);
        let errors = config.validate();
        assert!(
            errors.iter().any(|e| e.contains(expected)),
            "expected validation error containing {expected:?}, got {errors:?}"
        );
    }

    #[test]
    fn screen_particle_config_validation_catches_particle_edge_values() {
        let config = ScreenParticleConfig::default();

        assert_screen_particle_invalid(config.clone(), |c| c.dither_fade_start = 1.0, "dither");
        assert_screen_particle_invalid(
            config.clone(),
            |c| c.size_tiers[0].highlight_window = 0.0,
            "highlight_window",
        );
        assert_screen_particle_invalid(
            config.clone(),
            |c| c.size_tiers[0].life_scale = 0.0,
            "life_scale",
        );
        assert_screen_particle_invalid(
            config,
            |c| c.size_tiers[0].speed_scale = 0.0,
            "speed_scale",
        );
    }
}
