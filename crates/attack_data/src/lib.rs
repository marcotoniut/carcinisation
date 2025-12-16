//! A crate for managing data-driven attack configurations.
//!
//! Provides a full asset pipeline for attack data, from raw RON files to a
//! packed binary format for release builds.
//!
//! # Features
//! - `attack_hot_reload`: Enables loading raw `.ron` files and hot-reloading them
//!   at runtime. When disabled (default), the game loads a single pre-compiled
//!   `.bin` file.

pub mod asset;
pub mod compiler;
pub mod config;
pub mod packed;
pub mod plugin;
pub mod runtime;
