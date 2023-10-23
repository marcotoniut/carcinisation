use crate::components::DespawnMark;
use bevy::prelude::*;

pub const SCREEN_RESOLUTION: UVec2 = UVec2::new(160, 144);

#[cfg(debug_assertions)]
pub const VIEWPORT_MULTIPLIER: f32 = 4.;
#[cfg(not(debug_assertions))]
pub const VIEWPORT_MULTIPLIER: f32 = 3.;

pub const REAL_RESOLUTION: Vec2 = Vec2::new(
    SCREEN_RESOLUTION.x as f32 * VIEWPORT_MULTIPLIER,
    SCREEN_RESOLUTION.y as f32 * VIEWPORT_MULTIPLIER,
);

const EXTRA_X: f32 = 1.4;

#[cfg(debug_assertions)]
pub const VIEWPORT_RESOLUTION: Vec2 = Vec2::new(REAL_RESOLUTION.x * EXTRA_X, REAL_RESOLUTION.y);
#[cfg(not(debug_assertions))]
pub const VIEWPORT_RESOLUTION: Vec2 = REAL_RESOLUTION;

pub const VIEWPORT_RESOLUTION_OFFSET: Vec2 = Vec2::new(EXTRA_X / 2., 0.);

pub const HUD_HEIGHT: u32 = 14;
pub const FONT_SIZE: u32 = 10;

lazy_static! {
    pub static ref HALF_SCREEN_RESOLUTION: Vec2 = SCREEN_RESOLUTION.as_vec2() / 2.0;
    pub static ref HUD_OFFSET: UVec2 = UVec2::new(0, HUD_HEIGHT);
    pub static ref CAMERA_RESOLUTION: UVec2 =
        (SCREEN_RESOLUTION.as_vec2() - HUD_OFFSET.as_vec2()).as_uvec2();
    pub static ref HALF_CAMERA_RESOLUTION: Vec2 = CAMERA_RESOLUTION.as_vec2() / 2.0;
    pub static ref CAMERA_CENTER: Vec2 = CAMERA_RESOLUTION.as_vec2() / 2.0 + HUD_OFFSET.as_vec2();
}

pub const PATH_SPRITES_ENEMIES: &str = "sprites/enemies/";
pub const PATH_SPRITES_ATTACKS: &str = "sprites/attacks/";
pub const PATH_SPRITES_OBJECTS: &str = "sprites/objects/";

pub const TYPEFACE_PATH: &str = "typeface/pixeboy.png";
pub const TYPEFACE_INVERTED_PATH: &str = "typeface/pixeboy-inverted.png";
// pub const TYPEFACE_CHARACTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
pub const TYPEFACE_CHARACTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[{]}\\|;:'\",<.>/?";

pub const DEFAULT_CROSSHAIR_INDEX: u8 = 1;

pub const DEFAULT_MASTER_VOLUME: f32 = 0.8;
pub const DEFAULT_SFX_VOLUME: f32 = 0.08;
pub const DEFAULT_MUSIC_VOLUME: f32 = 0.06;

pub const DEBUG_STAGESTEP: bool = false;

pub fn is_inside_area(position: Vec2, bottom_left: Vec2, top_right: Vec2) -> bool {
    position.x >= bottom_left.x
        && position.x <= top_right.x
        && position.y >= bottom_left.y
        && position.y <= top_right.y
}

pub fn mark_for_despawn_by_component_query<T: Component>(
    commands: &mut Commands,
    query: &Query<Entity, With<T>>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(DespawnMark);
    }
}
