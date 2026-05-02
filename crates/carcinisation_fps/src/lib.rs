//! First-person raycaster for Carcinisation.
//!
//! Pure computation crate: map representation, camera, DDA raycasting, and
//! render orchestration. Does NOT depend on the main game crate.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::trivially_copy_pass_by_ref,
    clippy::struct_excessive_bools,
    clippy::fn_params_excessive_bools,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::needless_pass_by_value
)]

pub mod billboard;
pub mod camera;
pub mod collision;
pub mod data;
pub mod enemy;
pub mod layer;
pub mod map;
pub mod mosquiton;
pub mod player_attack;
pub mod plugin;
pub mod raycast;
pub mod render;
