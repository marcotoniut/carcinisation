//! Carcinisation game library — exposes data types and plugins for the runtime, editor, and tooling.
#![allow(
    dead_code,
    // Bevy system params (`Res<T>`, `Query<T>`, `Commands`) must be taken by value.
    clippy::needless_pass_by_value,
    // Bevy system signatures expand to 8+ params via query/resource tuples.
    clippy::too_many_arguments,
    // Bevy ECS queries produce deeply nested generics.
    clippy::type_complexity,
    // Numeric casts pervasive in pixel-coordinate/game math.
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    // Module paths like `enemy::components::Enemy` are clearer with repetition.
    clippy::module_name_repetitions,
    // Derive macros (Reflect, Serialize, Deserialize) generate underscore-prefixed
    // bindings internally that trigger false positives.
    clippy::used_underscore_binding,
)]

pub mod assets {
    pub use carcinisation_ors::assets::*;
}
pub mod components {
    pub use carcinisation_core::components::*;
}
pub mod core {
    pub use carcinisation_core::core::*;
}
pub mod app;
pub mod cutscene;
pub mod data {
    pub use carcinisation_ors::data::*;
}
pub mod debug;
#[cfg(feature = "gallery")]
pub mod gallery;
pub mod game;
pub mod globals;
mod layer;
pub mod letterbox;
mod main_menu;
mod progression;
pub mod resources;
pub mod splash;
pub mod stage {
    pub use carcinisation_ors::stage::*;
}
mod systems;
mod transitions;

#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

pub use crate::cutscene::data::CutsceneData;

pub mod asset_meta {
    use std::collections::HashMap;

    pub fn ensure_sprite_meta(path: &str, frames: usize) {
        crate::assets::ensure_sprite_meta(path, frames);
    }

    pub fn ensure_typeface_meta<S: std::hash::BuildHasher>(
        path: &str,
        characters: &str,
        separators: &HashMap<char, u32, S>,
    ) {
        crate::assets::ensure_typeface_meta(path, characters, separators);
    }
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
extern "C" {}
