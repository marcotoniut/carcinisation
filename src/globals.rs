use bevy::prelude::*;

pub const SCREEN_RESOLUTION: UVec2 = UVec2::new(160, 144);

pub const HUD_HEIGHT: u32 = 14;
pub const FONT_SIZE: u32 = 10;

pub const TYPEFACE_PATH: &str = "typeface/pixeboy.png";
pub const TYPEFACE_INVERTED_PATH: &str = "typeface/pixeboy-inverted.png";
// pub const TYPEFACE_CHARACTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
pub const TYPEFACE_CHARACTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[{]}\\|;:'\",<.>/?";

pub const CROSSHAIR_SPEED: f32 = 100.0;