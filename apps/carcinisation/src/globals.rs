//! Screen-size constants, asset paths, and shared helper functions.

use crate::components::DespawnMark;
use assert_assets_path::assert_assets_path;
use bevy::{ecs::query::QueryFilter, prelude::*};
#[cfg(not(target_arch = "wasm32"))]
use dotenvy::dotenv_override;
#[cfg(not(target_arch = "wasm32"))]
use std::env;
use std::sync::LazyLock;

/// Relative path from the crate manifest to the workspace assets directory.
pub const ASSETS_PATH: &str = "../../assets";

pub const SCREEN_RESOLUTION: UVec2 = UVec2::new(160, 144);

pub const WINDOW_MULTIPLIER_ENV: &str = "CARCINISATION_WINDOW_MULTIPLIER";

#[cfg(debug_assertions)]
const DEFAULT_VIEWPORT_MULTIPLIER: f32 = 4.;
#[cfg(not(debug_assertions))]
const DEFAULT_VIEWPORT_MULTIPLIER: f32 = 3.;

const EXTRA_X: f32 = 1.4;

pub const HUD_HEIGHT: u32 = 14;
pub const FONT_SIZE: u32 = 10;

// TODO is there some way to assert IVec2 at static time?
pub static SCREEN_RESOLUTION_H: LazyLock<IVec2> =
    LazyLock::new(|| (SCREEN_RESOLUTION / 2).as_ivec2());

pub static SCREEN_RESOLUTION_F32: LazyLock<Vec2> = LazyLock::new(|| SCREEN_RESOLUTION.as_vec2());
pub static SCREEN_RESOLUTION_F32_H: LazyLock<Vec2> =
    LazyLock::new(|| SCREEN_RESOLUTION.as_vec2() / 2.0);

static VIEWPORT_MULTIPLIER: LazyLock<f32> = LazyLock::new(load_viewport_multiplier);
static SCALED_SCREEN_RESOLUTION: LazyLock<Vec2> = LazyLock::new(|| {
    Vec2::new(
        SCREEN_RESOLUTION.x as f32 * viewport_multiplier(),
        SCREEN_RESOLUTION.y as f32 * viewport_multiplier(),
    )
});
static VIEWPORT_RESOLUTION: LazyLock<Vec2> = LazyLock::new(|| {
    #[cfg(debug_assertions)]
    {
        Vec2::new(
            scaled_screen_resolution().x * EXTRA_X,
            scaled_screen_resolution().y,
        )
    }
    #[cfg(not(debug_assertions))]
    {
        scaled_screen_resolution()
    }
});
static VIEWPORT_RESOLUTION_OFFSET: LazyLock<Vec2> = LazyLock::new(|| {
    #[cfg(debug_assertions)]
    {
        Vec2::new(
            (viewport_resolution().x - scaled_screen_resolution().x) * 0.5,
            0.0,
        )
    }
    #[cfg(not(debug_assertions))]
    {
        Vec2::ZERO
    }
});

pub const PATH_SPRITES_ENEMIES: &str = assert_assets_path!("sprites/enemies/");
pub const PATH_SPRITES_OBJECTS: &str = assert_assets_path!("sprites/objects/");

const TYPEFACE_INVERTED_PATH: &str =
    assert_assets_path!("typeface/pixeboy-inverted.px_typeface.png");
const TYPEFACE_CHARACTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[{]}\\|;:'\",<.>/?";

/// Loads the standard inverted typeface used by all UI overlays.
#[must_use]
pub fn load_inverted_typeface(
    assets: &crate::pixel::CxAssets<'_, '_, carapace::prelude::CxTypeface>,
) -> Handle<carapace::prelude::CxTypeface> {
    assets.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)])
}

pub const DEFAULT_CROSSHAIR_INDEX: u8 = 1;

pub const DEBUG_STAGESTEP: bool = false;

#[must_use]
pub fn viewport_multiplier() -> f32 {
    *VIEWPORT_MULTIPLIER
}

#[must_use]
pub fn scaled_screen_resolution() -> Vec2 {
    *SCALED_SCREEN_RESOLUTION
}

#[must_use]
pub fn viewport_resolution() -> Vec2 {
    *VIEWPORT_RESOLUTION
}

#[must_use]
pub fn viewport_resolution_offset() -> Vec2 {
    *VIEWPORT_RESOLUTION_OFFSET
}

#[cfg(target_arch = "wasm32")]
fn load_viewport_multiplier() -> f32 {
    DEFAULT_VIEWPORT_MULTIPLIER
}

#[cfg(not(target_arch = "wasm32"))]
fn load_viewport_multiplier() -> f32 {
    let _ = dotenv_override();

    match env::var(WINDOW_MULTIPLIER_ENV) {
        Ok(value) => match value.trim().parse::<f32>() {
            Ok(multiplier) if multiplier.is_finite() && multiplier > 0.0 => multiplier,
            Ok(_) => {
                warn!(
                    "{WINDOW_MULTIPLIER_ENV} must be a finite number greater than 0; using default {DEFAULT_VIEWPORT_MULTIPLIER}"
                );
                DEFAULT_VIEWPORT_MULTIPLIER
            }
            Err(_) => {
                warn!(
                    "{WINDOW_MULTIPLIER_ENV} must parse as a number; using default {DEFAULT_VIEWPORT_MULTIPLIER}"
                );
                DEFAULT_VIEWPORT_MULTIPLIER
            }
        },
        Err(env::VarError::NotPresent) => DEFAULT_VIEWPORT_MULTIPLIER,
        Err(env::VarError::NotUnicode(_)) => {
            warn!(
                "{WINDOW_MULTIPLIER_ENV} must be valid UTF-8; using default {DEFAULT_VIEWPORT_MULTIPLIER}"
            );
            DEFAULT_VIEWPORT_MULTIPLIER
        }
    }
}

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
