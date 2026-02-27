//! Bevy plugin for limited color palette pixel art games. Handles sprites, filters (defined
//! through images; apply to layers or individual entities), simple UI (text, buttons, and sprites
//! locked to the camera), tilemaps, animations (for sprites, filters, tilesets, and text;
//! supports dithering!), custom layers, particles (with pre-simulation!), palette changing,
//! typefaces, an in-game cursor, camera, lines, and more to come!

// TODO Remove `Px` prefix where possible

#![allow(
    // Pixel/graphics math requires pervasive narrowing casts (`as usize`, `as u32`, `as f32`).
    clippy::cast_possible_truncation,
    // Coordinate system conversions between unsigned and signed integers are unavoidable.
    clippy::cast_possible_wrap,
    // Pixel coordinates don't need double precision; `u32 as f32` is fine for rendering.
    clippy::cast_precision_loss,
    // Signed-to-unsigned casts for indexing where values are contextually non-negative.
    clippy::cast_sign_loss,
    // Bevy system signatures expand to 8+ params via `Extract<Query<...>>` tuples.
    clippy::too_many_arguments,
    // Layout and rendering functions are inherently complex.
    clippy::too_many_lines,
    // Bevy ECS queries produce deeply nested generics (`Query<(...), (With<A>, Without<B>)>`).
    clippy::type_complexity,
    // Bevy system params (`Res<T>`, `Query<T>`, `Commands`) must be taken by value.
    clippy::needless_pass_by_value,
    // Tracing span guards use `let _span = ...` to keep the span alive until drop.
    clippy::no_effect_underscore_binding,
)]
#![cfg_attr(not(feature = "headed"), allow(unused_imports))]
#![warn(missing_docs)]

pub mod animation;
pub mod atlas;
pub mod blink;
mod camera;
pub mod cursor;
pub mod filter;
pub mod frame;
mod image;
#[cfg(feature = "line")]
mod line;
mod map;
pub mod math;
pub mod palette;
#[cfg(feature = "particle")]
mod particle;
#[cfg(feature = "headed")]
mod picking;
pub mod position;
pub mod prelude;
mod profiling;
mod rect;
pub mod screen;
pub mod set;
pub mod sprite;
mod text;
mod ui;

use std::{marker::PhantomData, path::PathBuf};

use position::PxLayer;
use prelude::*;

/// Add to your [`App`] to enable `carapace`. The type parameter is your custom layer type
/// used for z-ordering. You can make one using [`px_layer`].
///
/// Add [`animation::PxAnimationPlugin`] if you want the built-in animation systems.
#[derive(Debug)]
pub struct PxPlugin<L: PxLayer> {
    screen_size: ScreenSize,
    palette_path: PathBuf,
    _l: PhantomData<L>,
}

impl<L: PxLayer> PxPlugin<L> {
    /// Create a [`PxPlugin`]. `screen_size` is the size of the screen in pixels.
    /// `palette_path` is the path from `assets/` to your game's palette. This palette will be used
    /// to load assets, even if you change it later.
    pub fn new(screen_size: impl Into<ScreenSize>, palette_path: impl Into<PathBuf>) -> Self {
        Self {
            screen_size: screen_size.into(),
            palette_path: palette_path.into(),
            _l: PhantomData,
        }
    }
}

impl<L: PxLayer> Plugin for PxPlugin<L> {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            (
                blink::plug,
                camera::plug,
                cursor::plug,
                frame::plug,
                palette::plug(self.palette_path.clone()),
                #[cfg(feature = "headed")]
                picking::plug::<L>,
                position::plug::<L>,
                screen::Plug::<L>::new(self.screen_size),
            ),
            (
                #[cfg(feature = "line")]
                line::plug::<L>,
                rect::plug::<L>,
                ui::plug::<L>,
                #[cfg(feature = "particle")]
                (RngPlugin::default(), particle::plug::<L>),
            ),
        ));

        let palette_path = self.palette_path.clone();
        atlas::plug::<L>(app, palette_path.clone());
        filter::plug::<L>(app, palette_path.clone());
        map::plug::<L>(app, palette_path.clone());
        sprite::plug::<L>(app, palette_path.clone());
        text::plug::<L>(app, palette_path);
    }
}
