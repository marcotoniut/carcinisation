// carcinisation_fps_core: Headless-only shared FP simulation (NO carapace dep)
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

pub mod burning;
pub mod camera;
pub mod collision;
pub mod combat;
pub mod config;
pub mod enemy;
pub mod fire_death;
pub mod ground_fire;
pub mod hash_util;
pub mod map;
pub mod mosquiton;
pub mod movement;
pub mod pickup;
pub mod presentation;
pub mod raycast;
pub mod spidey;

// Re-export pickup types
pub use pickup::{PickupKind, PickupRules};

// Re-export key types
pub use burning::{
    BurnConfig, BurnState, BurnTickResult, apply_exposure, burn_flame_count, burn_flame_scale,
    tick_burning,
};
pub use camera::Camera;
pub use collision::try_move;
pub use combat::{flame_hits_position, flame_hits_position_configured};
pub use config::{
    FpsCombatConfig, FpsMovementConfig, FpsVisualConfig, PlayerFlamethrowerConfig,
    ScreenParticleConfig, SizeTierConfig, SpideyCombatConfig,
};
pub use enemy::{
    DamageOutcome, Enemy, EnemyAiDisposition, EnemyAiOutput, EnemyPlayerTarget, EnemySim,
    EnemyState, FpsEnemyAiState, FpsEnemyKind, HitscanResult, MosquitonAiConfig, Projectile,
    ProjectileImpact, ProjectileKind, ProjectileSlowEffect, apply_damage, hitscan, hitscan_generic,
    is_showing_damage_invert, segment_circle_hit_distance, tick_enemies, tick_enemy_ai,
    tick_projectiles, tick_single_enemy,
};
pub use fire_death::{
    DamageKind, FireDeathConfig, PerimeterFlame, centered_flames_from_mask, corpse_seed,
    perimeter_flames_from_mask,
};
pub use ground_fire::{
    GroundFire, GroundFireConfig, GroundFireContactResult, GroundFireContactState,
    ground_fire_contact_damage, ground_fire_flame_layout, tick_ground_fires, try_spawn_ground_fire,
};
pub use map::{
    EntitySpawnData, EntitySpawnKind, Map, MapError, MapLoadData, PlayerStartData, test_map,
};
pub use mosquiton::{
    MosquitonSim, MosquitonSimConfig, MosquitonSimOutput, MosquitonSimState, tick_mosquiton_sim,
};
pub use movement::{
    SnapTurnKind, SnapTurnParams, SpeedModifier, angular_velocity_clamped, apply_movement,
    apply_movement_with_modifier, local_to_world, snap_turn_params, tick_snap_turn,
};
pub use presentation::{AttackPresentationKind, EnemyPresentationState};
pub use raycast::{HitSide, RayHit, cast_ray};
pub use raycast::{WallSurfaceId, has_line_of_sight};
pub use spidey::{SpideySim, SpideySimConfig, SpideySimOutput, SpideySimState, tick_spidey_sim};
