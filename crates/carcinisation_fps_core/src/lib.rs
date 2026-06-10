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
pub mod enemy_collision;
pub mod fire_death;
pub mod ground_fire;
pub mod hash_util;
pub mod hitscan;
pub mod map;
pub mod mosquiton;
pub mod movement;
pub mod occupancy;
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
pub use collision::{
    AnimationKey, BillboardFacing8, Capsule, Circle, Collider, CollisionFrameKey, HitDetail,
    HitResult, MaterialId, Obb, PartCollider2d, PartId, PartMetadata, TargetCollisionFrame,
    TargetCollisionSet, TargetQueryPose2d, nearest_ray_hit, nearest_ray_hit_tagged,
    nearest_segment_hit, nearest_segment_hit_tagged, ray_vs_capsule, ray_vs_circle,
    ray_vs_collider, ray_vs_obb, segment_vs_capsule, segment_vs_circle, segment_vs_collider,
    segment_vs_obb, swept_circle_vs_capsule, swept_circle_vs_circle, swept_circle_vs_collider,
    swept_circle_vs_obb, try_move,
};
pub use combat::{
    FirePose2d, flame_hits_position, flame_hits_position_configured,
    flame_hits_position_configured_from_pose, flame_hits_position_from_pose,
    flame_visual_max_distance, wall_obstruction_distance, wall_obstruction_distance_for_pose,
};
pub use config::{
    CombatControlMode, FpsCombatConfig, FpsMovementConfig, FpsVisualConfig, OccupancyConfig,
    PlayerFlamethrowerConfig, ScreenParticleConfig, SizeTierConfig, SpideyCombatConfig,
};
pub use enemy::{
    DamageOutcome, Enemy, EnemyAiDisposition, EnemyAiOutput, EnemyPlayerTarget, EnemySim,
    EnemyState, FpsEnemyAiState, FpsEnemyKind, HitscanResult, MosquitonAiConfig, Projectile,
    ProjectileImpact, ProjectileKind, ProjectileSlowEffect, apply_damage, facing_yaw_toward,
    hitscan, hitscan_from_pose, hitscan_generic, hitscan_generic_from_pose,
    hitscan_projectiles_from_pose, is_showing_damage_invert, segment_circle_hit_distance,
    tick_enemies, tick_enemy_ai, tick_projectiles, tick_single_enemy,
};
pub use enemy_collision::collision_set;
pub use fire_death::{
    DamageKind, FireDeathConfig, PerimeterFlame, centered_flames_from_mask, corpse_seed,
    perimeter_flames_from_mask,
};
pub use ground_fire::{
    GroundFire, GroundFireConfig, GroundFireContactResult, GroundFireContactState,
    ground_fire_contact_damage, ground_fire_flame_layout, tick_ground_fires, try_spawn_ground_fire,
};
pub use hitscan::{
    FlamePartHit, NEUTRAL_DAMAGE_SCALE, PartHitscanResult, PartHitscanTarget,
    flame_hits_target_parts, flame_hits_target_parts_configured, hitscan_parts_from_pose,
    scaled_damage,
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
