//! First-person raycaster for Carcinisation.
//!
//! Pure computation crate: map representation, camera, DDA raycasting, and
//! render orchestration. Does NOT depend on the main game crate.

pub mod billboard;
pub mod camera;
pub mod collision;
pub mod data;
pub mod enemy;
pub mod layer;
pub mod map;
pub mod plugin;
pub mod raycast;
pub mod render;
