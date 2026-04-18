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
    // Pixel coordinates are often compared with == (exact match intended).
    clippy::float_cmp,
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
    // Short pixel-coordinate variable names (e.g. x0, x1) are conventional in graphics code.
    clippy::similar_names,
    // Bevy observers use `_trigger` prefixed params that are read via trait methods.
    clippy::used_underscore_binding,
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
pub mod presentation;
mod profiling;
pub(crate) mod pxi;
mod rect;
#[cfg(feature = "reflect")]
mod reflect;
pub mod screen;
pub mod set;
pub mod sprite;
mod text;
mod ui;

use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
};

#[cfg(all(feature = "brp_extras", feature = "headed"))]
use bevy_brp_extras::BrpExtrasPlugin;
use position::PxLayer;
use prelude::*;

/// Registers only ECS-side types, resources, assets, and systems — without any render pipeline,
/// window, or GPU dependencies.
///
/// Use this for headless integration tests or server builds where game logic needs `carapace`
/// components and assets registered, but no rendering occurs.
///
/// # Usage
///
/// ```ignore
/// // Headless (tests, servers):
/// app.add_plugins(PxHeadlessPlugin::<MyLayer>::new(screen_size, "palette/base.png"));
///
/// // Headed (normal game):
/// app.add_plugins(PxPlugin::<MyLayer>::new(screen_size, "palette/base.png"));
/// ```
#[derive(Debug)]
pub struct PxHeadlessPlugin<L: PxLayer> {
    screen_size: ScreenSize,
    palette_path: PathBuf,
    _l: PhantomData<L>,
}

impl<L: PxLayer> PxHeadlessPlugin<L> {
    /// Create a [`PxHeadlessPlugin`]. `screen_size` is the size of the screen in pixels.
    /// `palette_path` is the path from `assets/` to your game's palette.
    pub fn new(screen_size: impl Into<ScreenSize>, palette_path: impl Into<PathBuf>) -> Self {
        Self {
            screen_size: screen_size.into(),
            palette_path: palette_path.into(),
            _l: PhantomData,
        }
    }
}

impl<L: PxLayer> Plugin for PxHeadlessPlugin<L> {
    fn build(&self, app: &mut App) {
        build_headless::<L>(app, self.screen_size, &self.palette_path);
    }
}

/// Add to your [`App`] to enable `carapace` with full rendering support. The type parameter
/// is your custom layer type used for z-ordering. You can make one using [`px_layer`].
///
/// For headless usage (tests, servers), use [`PxHeadlessPlugin`] instead.
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

    /// Register only ECS-side types without the render pipeline.
    ///
    /// Prefer [`PxHeadlessPlugin`] for new code — it provides the same behavior through
    /// the standard `Plugin` interface:
    ///
    /// ```ignore
    /// app.add_plugins(PxHeadlessPlugin::<MyLayer>::new(screen_size, "palette/base.png"));
    /// ```
    pub fn build_headless(&self, app: &mut App) {
        build_headless::<L>(app, self.screen_size, &self.palette_path);
    }
}

impl<L: PxLayer> Plugin for PxPlugin<L> {
    fn build(&self, app: &mut App) {
        #[cfg(all(feature = "brp_extras", feature = "headed"))]
        register_brp_extras_plugin(app);

        #[cfg(feature = "reflect")]
        reflect::register_types(app);

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

/// Shared headless registration sequence used by both [`PxHeadlessPlugin`] and
/// [`PxPlugin::build_headless`].
fn build_headless<L: PxLayer>(app: &mut App, screen_size: ScreenSize, palette_path: &Path) {
    #[cfg(feature = "reflect")]
    reflect::register_types(app);

    camera::plug_core(app);
    cursor::plug_core(app);
    frame::plug(app);
    app.add_plugins(palette::plug(palette_path.to_path_buf()));
    position::plug_core::<L>(app);

    // Screen: shared systems + headless startup (no window required)
    screen::plug_core(app);
    app.add_systems(Startup, screen::insert_screen_headless(screen_size));

    let palette_path = palette_path.to_path_buf();
    atlas::plug_core(app, palette_path.clone());
    filter::plug_core::<L>(app, palette_path.clone());
    map::plug_core(app, palette_path.clone());
    sprite::plug_core(app, palette_path.clone());
    text::plug_core(app, palette_path);

    // Blink, rect, line: no-op — their plugs only register headed systems
    // UI: layout + input systems (no RenderApp access)
    ui::plug::<L>(app);

    #[cfg(feature = "particle")]
    app.add_plugins((RngPlugin::default(), particle::plug::<L>));
}

#[cfg(all(feature = "brp_extras", feature = "headed"))]
fn register_brp_extras_plugin(app: &mut App) {
    if !app.is_plugin_added::<BrpExtrasPlugin>() {
        app.add_plugins(BrpExtrasPlugin);
    }
}

#[cfg(all(test, feature = "headed", feature = "brp_extras"))]
mod tests {
    use super::*;

    #[test]
    fn px_plugin_registers_brp_extras_when_feature_enabled() {
        let mut app = App::new();
        register_brp_extras_plugin(&mut app);
        assert!(app.is_plugin_added::<BrpExtrasPlugin>());

        // Regression guard: duplicate registration should stay a no-op.
        register_brp_extras_plugin(&mut app);
        assert!(app.is_plugin_added::<BrpExtrasPlugin>());
    }
}
