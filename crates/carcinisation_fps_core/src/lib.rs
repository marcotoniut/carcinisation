// carcinisation_fps_core: Headless-only shared FP simulation (NO carapace dep)
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

pub mod camera;
pub mod collision;
pub mod config;
pub mod enemy;
pub mod fire_death;
pub mod map;
pub mod movement;
pub mod raycast;

// Re-export key types
pub use camera::Camera;
pub use collision::try_move;
pub use config::{
    BURN_CONTACT_DAMAGE, BURN_CONTACT_RADIUS, BURN_CONTACT_TICK_SECS, COLLISION_MARGIN,
    DAMAGE_FLICKER_COUNT, DAMAGE_FLICKER_INVERT_SECS, DAMAGE_FLICKER_REGULAR_SECS,
    ENEMY_DEATH_ANIM_SECS, ENEMY_DESPAWN_DELAY, FIRE_COOLDOWN_SECS, FLAME_DPS, FLAME_HALF_ANGLE,
    FLAME_RANGE, HITSCAN_DAMAGE, MOSQUITON_ATTACK_INTERVAL, MOSQUITON_MELEE_DAMAGE,
    MOSQUITON_MELEE_RANGE, MOSQUITON_PROJECTILE_DAMAGE, MOSQUITON_SHOOT_CUE_SECS, MOVE_SPEED,
    PLAYER_RESPAWN_DELAY_SECS, PROJECTILE_HIT_RADIUS, PROJECTILE_LIFETIME, PROJECTILE_SPEED,
    QUICK_TURN_DURATION_SECS, TURN_SPEED,
};
pub use enemy::{
    Enemy, EnemyAiDisposition, EnemyAiOutput, EnemyPlayerTarget, EnemySim, EnemyState,
    FpsEnemyAiState, FpsEnemyKind, HitscanResult, MosquitonAiConfig, Projectile, ProjectileImpact,
    hitscan, segment_circle_hit_distance, tick_enemies, tick_enemy_ai, tick_projectiles,
    tick_single_enemy,
};
pub use fire_death::{
    DamageKind, FireDeathConfig, PerimeterFlame, corpse_seed, perimeter_flames_from_mask,
};
pub use map::{Map, MapError};
pub use raycast::{HitSide, RayHit, cast_ray};
