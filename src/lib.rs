mod assets;
pub mod bevy_utils;
mod components;
mod core;
pub mod cutscene;
mod data;
pub mod debug;
mod game;
pub mod globals;
mod input;
mod layer;
pub mod letterbox;
mod main_menu;
mod pixel;
mod plugins;
mod progression;
mod resources;
pub mod stage;
mod systems;
mod transitions;

#[macro_use]
extern crate lazy_static;

use wasm_bindgen::prelude::*;

pub use crate::cutscene::data::CutsceneData;

pub mod asset_meta {
    use bevy::utils::HashMap;

    pub fn ensure_sprite_meta(path: &str, frames: usize) {
        crate::pixel::assets::ensure_sprite_meta(path, frames);
    }

    pub fn ensure_typeface_meta(path: &str, characters: &str, separators: &HashMap<char, u32>) {
        crate::pixel::assets::ensure_typeface_meta(path, characters, separators);
    }
}

#[wasm_bindgen]
extern "C" {}
