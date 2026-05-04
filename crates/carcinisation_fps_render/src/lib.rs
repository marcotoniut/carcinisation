// carcinisation_fps_render: Client-only rendering (depends on carcinisation_fps_core + carapace)

// Re-export key types from core
pub use carcinisation_fps_core::{
    camera::Camera,
    collision::try_move,
    enemy::{
        Enemy, EnemyState, HitscanResult, Projectile, ProjectileImpact, hitscan, tick_enemies,
        tick_projectiles, tick_single_enemy,
    },
    layer::FpsLayer,
    map::Map,
    raycast::{HitSide, RayHit, cast_ray},
};

// Import carapace for rendering
#[allow(unused_imports)]
use carapace::prelude::*;

// Declare modules (will implement later)
// pub mod billboard;
// pub mod data_render;
// pub mod mosquito;
// pub mod player_attack;
// pub mod plugin;
// pub mod render;
// pub mod sky;
