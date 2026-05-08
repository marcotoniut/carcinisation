//! Shared FPS gameplay constants.
//!
//! Single source of truth for all gameplay tuning values used by both
//! singleplayer and multiplayer. Server and client import from here.
//!
//! When adding a new weapon, enemy type, or mechanic — add its constants here
//! so both SP and MP automatically use the same values.

// -- Movement --

/// Default movement speed in map-units per second.
pub const MOVE_SPEED: f32 = 2.0;
/// Default turn speed in radians per second.
pub const TURN_SPEED: f32 = 2.0;
/// Collision margin in map-units (distance kept from walls).
pub const COLLISION_MARGIN: f32 = 0.2;

// -- Snap / Quick Turn --

/// Duration of a 180° quick turn in seconds. 90° turns complete in half this time.
pub const QUICK_TURN_DURATION_SECS: f32 = 0.4;

// -- Pistol --

/// Damage per hitscan shot.
pub const HITSCAN_DAMAGE: f32 = 37.0;
/// Minimum seconds between pistol shots.
pub const FIRE_COOLDOWN_SECS: f32 = 0.33;

// -- Flamethrower --

/// Flamethrower damage per second (continuous while held).
pub const FLAME_DPS: f32 = 580.0;
/// Flamethrower max range in world units.
pub const FLAME_RANGE: f32 = 5.0;
/// Flamethrower half-angle in radians (~30 degrees).
pub const FLAME_HALF_ANGLE: f32 = 0.52;

// -- Enemy Projectiles --

/// Enemy projectile speed in world-units per second.
pub const PROJECTILE_SPEED: f32 = 4.0;
/// Enemy projectile collision radius.
pub const PROJECTILE_HIT_RADIUS: f32 = 0.3;
/// Enemy projectile lifetime in seconds before auto-despawn.
pub const PROJECTILE_LIFETIME: f32 = 3.0;

// -- Mosquiton Enemy --

/// Seconds between Mosquiton ranged attacks.
pub const MOSQUITON_ATTACK_INTERVAL: f32 = 2.0;
/// Damage per Mosquiton ranged projectile.
pub const MOSQUITON_PROJECTILE_DAMAGE: f32 = 10.0;
/// Damage per Mosquiton melee hit.
pub const MOSQUITON_MELEE_DAMAGE: f32 = 15.0;
/// Melee range in world units.
pub const MOSQUITON_MELEE_RANGE: f32 = 0.8;
/// Delay from shoot animation start to blood_shot projectile spawn.
/// Derived from composed atlas `shoot_fly` cue frame.
/// The fps crate test `shoot_cue_elapsed_from_composed_atlas` validates this.
pub const MOSQUITON_SHOOT_CUE_SECS: f32 = 1.0;

// -- Burning Corpse Contact Damage --
// Values match SP RON config: assets/config/attacks/player_flamethrower_fps.ron

/// Radius for burning corpse contact damage.
pub const BURN_CONTACT_RADIUS: f32 = 1.1;
/// Damage per burn contact tick.
pub const BURN_CONTACT_DAMAGE: f32 = 5.0;
/// Seconds between burn contact damage ticks.
pub const BURN_CONTACT_TICK_SECS: f32 = 0.5;

// -- Death / Respawn --

/// Duration of enemy death animation before transitioning to Dead state.
pub const ENEMY_DEATH_ANIM_SECS: f32 = 0.5;
/// Seconds before a dead enemy entity is despawned.
pub const ENEMY_DESPAWN_DELAY: f32 = 5.0;
/// Seconds before a dead player respawns.
pub const PLAYER_RESPAWN_DELAY_SECS: f32 = 3.0;

// -- Damage Flicker --

/// Number of invert cycles in a damage flicker effect.
pub const DAMAGE_FLICKER_COUNT: u8 = 4;
/// Duration of the regular (non-inverted) phase in seconds.
pub const DAMAGE_FLICKER_REGULAR_SECS: f32 = 0.1;
/// Duration of the inverted phase in seconds.
pub const DAMAGE_FLICKER_INVERT_SECS: f32 = 0.075;
