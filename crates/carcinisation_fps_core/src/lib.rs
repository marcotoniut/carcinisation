// carcinisation_fps_core: Headless-only shared FP simulation (NO carapace dep)

pub mod camera;
pub mod collision;
pub mod enemy;
pub mod fire_death;
pub mod layer;
pub mod map;
pub mod raycast;

// Re-export key types
pub use camera::Camera;
pub use collision::try_move;
pub use enemy::{
    Enemy, EnemyState, HitscanResult, Projectile, ProjectileImpact, hitscan, tick_enemies,
    tick_projectiles, tick_single_enemy,
};
pub use fire_death::{
    DamageKind, FireDeathConfig, PerimeterFlame, corpse_seed, perimeter_flames_from_mask,
};
pub use layer::FpsLayer;
pub use map::Map;
pub use raycast::{HitSide, RayHit, cast_ray};
