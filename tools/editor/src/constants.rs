use bevy::prelude::Color;
use std::path::PathBuf;

pub const ASSETS_PATH: &str = "../../assets/";
// TODO assert_assets_path! with right base path to assets
pub const FONT_PATH: &str = "fonts/FiraSans-Bold.ttf";

pub const CAMERA_MOVE_SENSITIVITY: f32 = 1.0;
pub const CAMERA_ZOOM_MIN: f32 = 0.5;
pub const CAMERA_ZOOM_MAX: f32 = 2.0;
pub const CAMERA_MOVE_BOUNDARY: f32 = 2000.0;

pub trait EditorColor {
    const CYAN: Self;
}

impl EditorColor for Color {
    const CYAN: Self = Color::srgb(0.0, 1.0, 1.0);
}

pub fn assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(ASSETS_PATH)
}
