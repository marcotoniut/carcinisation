//! Shared FPS gameplay constants.
//!
//! Single source of truth for all gameplay tuning values used by both
//! singleplayer and multiplayer. Server and client import from here.
//!
//! When adding a new weapon, enemy type, or mechanic — add its constants here
//! so both SP and MP automatically use the same values.

use bevy::prelude::ReflectResource;

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
    pub emit_interval_ms: u64,
    /// Damage applied per flamethrower tick while a target is in the flame.
    pub damage_per_tick: u32,
    /// Interval between damage ticks (milliseconds).
    pub tick_ms: u64,
    /// Maximum ammo pool. Ammo drains continuously while firing.
    pub max_ammo: f32,
    /// Ammo consumed per millisecond of sustained fire.
    pub ammo_drain_per_ms: f32,
    /// How long a burning corpse persists before despawning (seconds).
    pub burning_corpse_duration_secs: f32,
    /// Damage dealt to the player per contact tick when touching a burning corpse.
    pub burning_corpse_contact_damage: u32,
    /// Interval between burning-corpse contact damage ticks (milliseconds).
    pub burning_corpse_contact_tick_ms: u64,
    /// Radius around a burning corpse that triggers contact damage (world units).
    pub burning_corpse_contact_radius: f32,
    /// Damage applied to nearby enemies each tick by a burning corpse (crossfire).
    pub burning_corpse_crossfire_damage: u32,
    /// Number of flame sprites placed around a burning corpse perimeter.
    pub burning_flame_count: usize,
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

    #[must_use]
    pub fn fire_death_config(&self) -> crate::fire_death::FireDeathConfig {
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
    pub fn burning_corpse_contact_tick_secs(&self) -> f32 {
        std::time::Duration::from_millis(self.burning_corpse_contact_tick_ms).as_secs_f32()
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
    /// Spidey movement speed during hops (map units/s).
    #[serde(default = "default_spidey_move_speed")]
    pub spidey_move_speed: f32,
    /// Spidey collision radius for wall avoidance.
    #[serde(default = "default_spidey_collision_radius")]
    pub spidey_collision_radius: f32,
    /// Spidey aggro range (map units).
    #[serde(default = "default_spidey_aggro_range")]
    pub spidey_aggro_range: f32,
    /// Minimum seconds between hops.
    #[serde(default = "default_spidey_hop_interval_min")]
    pub spidey_hop_interval_min: f32,
    /// Maximum seconds between hops.
    #[serde(default = "default_spidey_hop_interval_max")]
    pub spidey_hop_interval_max: f32,
    /// Distance covered per hop (map units).
    #[serde(default = "default_spidey_hop_distance")]
    pub spidey_hop_distance: f32,
    /// Duration of a single hop (seconds).
    #[serde(default = "default_spidey_hop_duration")]
    pub spidey_hop_duration: f32,
    /// Peak billboard height during a hop (visual only, map units).
    #[serde(default = "default_spidey_hop_visual_height")]
    pub spidey_hop_visual_height: f32,
    /// Maximum range for lunge melee attack.
    #[serde(default = "default_spidey_lunge_range")]
    pub spidey_lunge_range: f32,
    /// Movement speed during lunge (map units/s).
    #[serde(default = "default_spidey_lunge_speed")]
    pub spidey_lunge_speed: f32,
    /// Damage dealt on lunge arrival.
    #[serde(default = "default_spidey_lunge_melee_damage")]
    pub spidey_lunge_melee_damage: u32,
    /// Crouch duration before lunge (seconds).
    #[serde(default = "default_spidey_lunge_windup_secs")]
    pub spidey_lunge_windup_secs: f32,
    /// Maximum lunge duration before auto-recover (seconds).
    #[serde(default = "default_spidey_lunge_duration_secs")]
    pub spidey_lunge_duration_secs: f32,
    /// Seconds between lunge attacks.
    #[serde(default = "default_spidey_lunge_cooldown")]
    pub spidey_lunge_cooldown: f32,
    /// Maximum range for web ranged attack.
    #[serde(default = "default_spidey_web_range")]
    pub spidey_web_range: f32,
    /// Seconds between web attacks.
    #[serde(default = "default_spidey_web_cooldown")]
    pub spidey_web_cooldown: f32,
    /// Animation lead before web projectile spawns (seconds).
    #[serde(default = "default_spidey_web_cue_secs")]
    pub spidey_web_cue_secs: f32,
    /// Web projectile speed (map units/s).
    #[serde(default = "default_spidey_web_projectile_speed")]
    pub spidey_web_projectile_speed: f32,
    /// Web projectile damage.
    #[serde(default = "default_spidey_web_projectile_damage")]
    pub spidey_web_projectile_damage: u32,
    /// Spidey default health.
    #[serde(default = "default_spidey_health")]
    pub spidey_health: u32,
    /// `WebShot` slow multiplier applied to player movement on hit.
    #[serde(default = "default_spidey_web_slow_multiplier")]
    pub spidey_web_slow_multiplier: f32,
    /// `WebShot` slow duration in seconds.
    #[serde(default = "default_spidey_web_slow_duration")]
    pub spidey_web_slow_duration: f32,
    /// Recovery pause duration after attacks (seconds).
    #[serde(default = "default_spidey_recover_secs")]
    pub spidey_recover_secs: f32,
    /// Death animation duration (seconds).
    #[serde(default = "default_spidey_death_secs")]
    pub spidey_death_secs: f32,
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
    pub ground_fire_flame_count: usize,
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
    pub fn mosquiton_attack_interval(&self) -> f32 {
        self.mosquiton_shoot_cooldown
    }

    /// Build a `MosquitonSimConfig` from the combat config values.
    #[must_use]
    pub fn mosquiton_sim_config(&self) -> crate::mosquiton::MosquitonSimConfig {
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
    pub fn spidey_sim_config(&self) -> crate::spidey::SpideySimConfig {
        crate::spidey::SpideySimConfig {
            move_speed: self.spidey_move_speed,
            collision_radius: self.spidey_collision_radius,
            aggro_range: self.spidey_aggro_range,
            hop_interval_min: self.spidey_hop_interval_min,
            hop_interval_max: self.spidey_hop_interval_max,
            hop_distance: self.spidey_hop_distance,
            hop_duration: self.spidey_hop_duration,
            hop_visual_height: self.spidey_hop_visual_height,
            lunge_range: self.spidey_lunge_range,
            lunge_speed: self.spidey_lunge_speed,
            lunge_melee_damage: self.spidey_lunge_melee_damage,
            lunge_windup_secs: self.spidey_lunge_windup_secs,
            lunge_duration_secs: self.spidey_lunge_duration_secs,
            lunge_cooldown: self.spidey_lunge_cooldown,
            web_range: self.spidey_web_range,
            web_cooldown: self.spidey_web_cooldown,
            web_cue_secs: self.spidey_web_cue_secs,
            web_projectile_speed: self.spidey_web_projectile_speed,
            web_projectile_damage: self.spidey_web_projectile_damage,
            recover_secs: self.spidey_recover_secs,
            death_secs: self.spidey_death_secs,
        }
    }

    /// Build a `GroundFireConfig` from the combat config values.
    #[must_use]
    pub fn ground_fire_config(&self) -> crate::ground_fire::GroundFireConfig {
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
            spidey_move_speed: default_spidey_move_speed(),
            spidey_collision_radius: default_spidey_collision_radius(),
            spidey_aggro_range: default_spidey_aggro_range(),
            spidey_hop_interval_min: default_spidey_hop_interval_min(),
            spidey_hop_interval_max: default_spidey_hop_interval_max(),
            spidey_hop_distance: default_spidey_hop_distance(),
            spidey_hop_duration: default_spidey_hop_duration(),
            spidey_hop_visual_height: default_spidey_hop_visual_height(),
            spidey_lunge_range: default_spidey_lunge_range(),
            spidey_lunge_speed: default_spidey_lunge_speed(),
            spidey_lunge_melee_damage: default_spidey_lunge_melee_damage(),
            spidey_lunge_windup_secs: default_spidey_lunge_windup_secs(),
            spidey_lunge_duration_secs: default_spidey_lunge_duration_secs(),
            spidey_lunge_cooldown: default_spidey_lunge_cooldown(),
            spidey_web_range: default_spidey_web_range(),
            spidey_web_cooldown: default_spidey_web_cooldown(),
            spidey_web_cue_secs: default_spidey_web_cue_secs(),
            spidey_web_projectile_speed: default_spidey_web_projectile_speed(),
            spidey_web_projectile_damage: default_spidey_web_projectile_damage(),
            spidey_health: default_spidey_health(),
            spidey_web_slow_multiplier: default_spidey_web_slow_multiplier(),
            spidey_web_slow_duration: default_spidey_web_slow_duration(),
            spidey_recover_secs: default_spidey_recover_secs(),
            spidey_death_secs: default_spidey_death_secs(),
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
            ground_fire_flame_count: 6,
            ground_fire_max: 32,
        }
    }
}

// -- Spidey serde defaults --

// Serde fallback defaults — must match combat.ron shipped values.
// These only fire when deserializing old RON files missing newer fields.

fn default_spidey_move_speed() -> f32 {
    2.0
}
fn default_spidey_collision_radius() -> f32 {
    0.25
}
fn default_spidey_aggro_range() -> f32 {
    8.0
}
fn default_spidey_hop_interval_min() -> f32 {
    0.4
}
fn default_spidey_hop_interval_max() -> f32 {
    1.0
}
fn default_spidey_hop_distance() -> f32 {
    1.2
}
fn default_spidey_hop_duration() -> f32 {
    0.4
}
fn default_spidey_hop_visual_height() -> f32 {
    0.3
}
fn default_spidey_lunge_range() -> f32 {
    2.0
}
fn default_spidey_lunge_speed() -> f32 {
    7.0
}
fn default_spidey_lunge_melee_damage() -> u32 {
    20
}
fn default_spidey_lunge_windup_secs() -> f32 {
    0.2
}
fn default_spidey_lunge_duration_secs() -> f32 {
    0.7
}
fn default_spidey_lunge_cooldown() -> f32 {
    3.0
}
fn default_spidey_web_range() -> f32 {
    6.0
}
fn default_spidey_web_cooldown() -> f32 {
    3.0
}
fn default_spidey_web_cue_secs() -> f32 {
    1.0
}
fn default_spidey_web_projectile_speed() -> f32 {
    3.0
}
fn default_spidey_web_projectile_damage() -> u32 {
    5
}
fn default_spidey_health() -> u32 {
    100
}
fn default_spidey_web_slow_multiplier() -> f32 {
    0.7
}
fn default_spidey_web_slow_duration() -> f32 {
    3.0
}
fn default_spidey_recover_secs() -> f32 {
    0.5
}
fn default_spidey_death_secs() -> f32 {
    0.6
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

fn default_view_bob_amplitude() -> f32 {
    1.5
}
fn default_view_bob_freq_mult() -> f32 {
    2.0
}
fn default_view_bob_near() -> f32 {
    3.0
}
fn default_view_bob_mid() -> f32 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fps_visual_config_ron_parses() {
        let config = FpsVisualConfig::load();
        assert_eq!(config.damage_flicker_count, 4);
        assert!((config.damage_flicker_regular_secs - 0.1).abs() < f32::EPSILON);
        assert!((config.damage_flicker_invert_secs - 0.075).abs() < f32::EPSILON);
    }
}
