#![feature(step_trait)]

mod assets;
mod bevy_utils;
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

#[wasm_bindgen]
extern "C" {}
