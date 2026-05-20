//! Shared gameplay vocabulary for Carcinisation.
//!
//! Defines the [`Layer`] rendering enum (composed from per-mode sub-layers)
//! and shared types used across all game modes.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

pub mod direction;
pub mod directional_animation;
pub mod directional_config;
pub mod fire_death;
pub mod game;
pub mod layer;
pub mod layer_order;
