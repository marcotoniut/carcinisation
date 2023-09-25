use bevy::prelude::*;

pub const SCREEN_RESOLUTION: UVec2 = UVec2::new(160, 144);

pub const HUD_HEIGHT: u32 = 14;
pub const FONT_SIZE: u32 = 10;

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

pub fn despawn_by_component_query<T: Component>(
    commands: &mut Commands,
    query: &Query<Entity, With<T>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
