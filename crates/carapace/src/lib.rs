//! Bevy plugin for limited-colour-palette pixel-art games.
//!
//! Provides sprites, palette-indexed filters, tilemaps, animations (with dithered
//! frame transitions), composites, particles, text, a simple UI system, an
//! in-game cursor, camera, line drawing, and more.
//!
//! # Position pipeline
//!
//! Every positioned entity flows through three stages:
//!
//! ```text
//! WorldPos          (Vec2  — authoritative world-space gameplay position)
//!   → CxPosition      (IVec2 — derived integer cache, rounded each frame)
//!   → CxPresentationTransform  (visual & collision offsets, layered separately)
//!       ├─ visual_offset      — rendering reads this
//!       └─ collision_offset   — hit-detection reads this
//! ```
//!
//! Simulation systems (movement, AI, spawn placement) read and write
//! [`WorldPos`].  [`CxPosition`] is a read-only cache for the renderer —
//! never write it directly.  [`CxPresentationTransform`] carries per-entity
//! visual displacement (parallax, knockback) without altering the gameplay
//! position.
//!
//! # Coordinate spaces
//!
//! | Space | Origin | Unit | Who reads it |
//! |-------|--------|------|-------------|
//! | **World** | Stage origin (bottom-left of level) | Pixel (f32 or i32) | Simulation, physics, AI |
//! | **Visual** | World position + presentation offsets | Pixel | Rendering, collision hit-detection |
//! | **Screen** | Bottom-left of the rendered canvas | Pixel (u32) | Cursor, UI layout, picking |
//!
//! Types document which space they belong to.  When in doubt, check
//! [`CxRenderSpace`]: `World` = drawn relative to world origin;
//! `Camera` = drawn at a fixed screen position.
//!
//! # Anchor origin convention
//!
//! [`CxAnchor`] uses **bottom-left origin** (Y-up):
//! `(0, 0)` = bottom-left, `(1, 1)` = top-right.  This matches Bevy's and
//! most 2D engines' world-space convention.
//!
//! [`PartTransform`]`.pivot` uses **top-left origin**
//! (Y-down): `(0, 0)` = top-left, `(1, 1)` = bottom-right.  This matches
//! image/texture convention where row 0 is the top of the raster buffer.
//!
//! The difference is intentional — anchors position entities in world space
//! (Y-up), while pivots address pixels within a raster part (Y-down).  An
//! internal `PartTransform::anchor()` method converts between the two
//! conventions.

//! # The `Cx` prefix
//!
//! `Cx` stays on collision-prone or generic public types. Descriptive
//! low-risk names may remain unprefixed when they still read cleanly in
//! `use carapace::prelude::*;`. `Cx` is retired as the public namespace
//! policy. Read `Cx` as "carapace" rather than "pixel".

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
/// Convenience bundles for spawning common entity configurations.
pub mod bundles;
mod camera;
pub mod cursor;
#[cfg(feature = "headed")]
pub mod debug_draw;
pub mod filter;
pub mod frame;
/// Palette-indexed raster image buffer.
pub mod image;
#[cfg(feature = "line")]
mod line;
pub mod math;
pub mod palette;
#[cfg(feature = "particle")]
mod particle;
#[cfg(feature = "headed")]
mod picking;
pub mod position;
pub mod prelude;
pub mod presentation;
pub mod primitive;
mod profiling;
pub(crate) mod pxi;
/// Generic raycaster rendering helpers.
pub mod raycaster;
mod rect;
#[cfg(feature = "reflect")]
mod reflect;
pub mod screen;
pub mod set;
pub mod sprite;
mod text;
mod tilemap;
mod ui;

use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
};

#[cfg(all(feature = "brp_extras", feature = "headed"))]
use bevy_brp_extras::BrpExtrasPlugin;
use position::CxLayer;
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
/// app.add_plugins(CxHeadlessPlugin::<MyLayer>::new(screen_size, "palette/base.png"));
///
/// // Headed (normal game):
/// app.add_plugins(CxPlugin::<MyLayer>::new(screen_size, "palette/base.png"));
/// ```
#[derive(Debug)]
pub struct CxHeadlessPlugin<L: CxLayer> {
    screen_size: CxScreenSize,
    palette_path: PathBuf,
    _l: PhantomData<L>,
}

impl<L: CxLayer> CxHeadlessPlugin<L> {
    /// Create a [`CxHeadlessPlugin`]. `screen_size` is the size of the screen in pixels.
    /// `palette_path` is the path from `assets/` to your game's palette.
    pub fn new(screen_size: impl Into<CxScreenSize>, palette_path: impl Into<PathBuf>) -> Self {
        Self {
            screen_size: screen_size.into(),
            palette_path: palette_path.into(),
            _l: PhantomData,
        }
    }
}

impl<L: CxLayer> Plugin for CxHeadlessPlugin<L> {
    fn build(&self, app: &mut App) {
        build_headless::<L>(app, self.screen_size, &self.palette_path);
    }
}

/// Add to your [`App`] to enable `carapace` with full rendering support. The type parameter
/// is your custom layer type used for z-ordering. You can make one using [`px_layer`].
///
/// For headless usage (tests, servers), use [`CxHeadlessPlugin`] instead.
///
/// Add [`animation::CxAnimationPlugin`] if you want the built-in animation systems.
#[derive(Debug)]
pub struct CxPlugin<L: CxLayer> {
    screen_size: CxScreenSize,
    palette_path: PathBuf,
    _l: PhantomData<L>,
}

impl<L: CxLayer> CxPlugin<L> {
    /// Create a [`CxPlugin`]. `screen_size` is the size of the screen in pixels.
    /// `palette_path` is the path from `assets/` to your game's palette. This palette will be used
    /// to load assets, even if you change it later.
    pub fn new(screen_size: impl Into<CxScreenSize>, palette_path: impl Into<PathBuf>) -> Self {
        Self {
            screen_size: screen_size.into(),
            palette_path: palette_path.into(),
            _l: PhantomData,
        }
    }

    /// Register only ECS-side types without the render pipeline.
    ///
    /// Prefer [`CxHeadlessPlugin`] for new code — it provides the same behavior through
    /// the standard `Plugin` interface:
    ///
    /// ```ignore
    /// app.add_plugins(CxHeadlessPlugin::<MyLayer>::new(screen_size, "palette/base.png"));
    /// ```
    pub fn build_headless(&self, app: &mut App) {
        build_headless::<L>(app, self.screen_size, &self.palette_path);
    }
}

impl<L: CxLayer> Plugin for CxPlugin<L> {
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
                primitive::plug::<L>,
                ui::plug::<L>,
                #[cfg(feature = "particle")]
                (RngPlugin::default(), particle::plug::<L>),
            ),
        ));

        let palette_path = self.palette_path.clone();
        atlas::plug::<L>(app, palette_path.clone());
        filter::plug::<L>(app, palette_path.clone());
        tilemap::plug::<L>(app, palette_path.clone());
        sprite::plug::<L>(app, palette_path.clone());
        text::plug::<L>(app, palette_path);
    }
}

/// Shared headless registration sequence used by both [`CxHeadlessPlugin`] and
/// [`CxPlugin::build_headless`].
fn build_headless<L: CxLayer>(app: &mut App, screen_size: CxScreenSize, palette_path: &Path) {
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
    tilemap::plug_core(app, palette_path.clone());
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
