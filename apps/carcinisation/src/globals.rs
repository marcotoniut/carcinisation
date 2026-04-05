//! Screen-size constants, asset paths, and shared helper functions.

use crate::components::DespawnMark;
use assert_assets_path::assert_assets_path;
use bevy::{ecs::query::QueryFilter, prelude::*};

/// Relative path from the crate manifest to the workspace assets directory.
pub const ASSETS_PATH: &str = "../../assets";

pub const SCREEN_RESOLUTION: UVec2 = UVec2::new(160, 144);

#[cfg(debug_assertions)]
pub const VIEWPORT_MULTIPLIER: f32 = 4.;
#[cfg(not(debug_assertions))]
pub const VIEWPORT_MULTIPLIER: f32 = 3.;

const REAL_RESOLUTION: Vec2 = Vec2::new(
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

// TODO is there some way to assert IVec2 at static time?
pub static SCREEN_RESOLUTION_H: std::sync::LazyLock<IVec2> =
    std::sync::LazyLock::new(|| (SCREEN_RESOLUTION / 2).as_ivec2());

pub static SCREEN_RESOLUTION_F32: std::sync::LazyLock<Vec2> =
    std::sync::LazyLock::new(|| SCREEN_RESOLUTION.as_vec2());
pub static SCREEN_RESOLUTION_F32_H: std::sync::LazyLock<Vec2> =
    std::sync::LazyLock::new(|| SCREEN_RESOLUTION.as_vec2() / 2.0);

pub const PATH_SPRITES_ENEMIES: &str = assert_assets_path!("sprites/enemies/");
pub const PATH_SPRITES_ATTACKS: &str = assert_assets_path!("sprites/attacks/");
pub const PATH_SPRITES_OBJECTS: &str = assert_assets_path!("sprites/objects/");

const TYPEFACE_INVERTED_PATH: &str =
    assert_assets_path!("typeface/pixeboy-inverted.px_typeface.png");
const TYPEFACE_CHARACTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[{]}\\|;:'\",<.>/?";

/// Loads the standard inverted typeface used by all UI overlays.
pub fn load_inverted_typeface(
    assets: &crate::pixel::PxAssets<'_, '_, carapace::prelude::PxTypeface>,
) -> Handle<carapace::prelude::PxTypeface> {
    assets.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)])
}

pub const DEFAULT_CROSSHAIR_INDEX: u8 = 1;

pub const DEBUG_STAGESTEP: bool = false;

#[must_use]
pub fn is_inside_area(position: Vec2, bottom_left: Vec2, top_right: Vec2) -> bool {
    position.x >= bottom_left.x
        && position.x <= top_right.x
        && position.y >= bottom_left.y
        && position.y <= top_right.y
}

// TODO could replace with a generic trigger/observe?
pub fn mark_for_despawn_by_query<F: QueryFilter>(
    commands: &mut Commands,
    query: &Query<'_, '_, Entity, F>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(DespawnMark);
    }
}
