//! Screen and rendering
//!
//! Data flow: gather render-world components by layer, draw into a CPU `CxImage`,
//! then upload to a reusable `R8Uint` texture and present via a fullscreen quad.
//! This is the single compositing path for sprites, text, tilemaps, rects, lines, and filters.

mod draw;
#[cfg(feature = "gpu_palette")]
mod gpu_sprite;
#[cfg(feature = "headed")]
mod node;
#[cfg(feature = "headed")]
mod pipeline;

use std::marker::PhantomData;
#[cfg(feature = "gpu_palette")]
use std::sync::RwLock;

use bevy_asset::uuid_handle;
#[cfg(feature = "headed")]
use bevy_core_pipeline::core_2d::graph::{Core2d, Node2d};
use bevy_math::UVec2;
#[cfg(feature = "headed")]
use bevy_render::{
    Render, RenderApp, RenderSystems,
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_graph::{RenderGraphExt, RenderLabel, ViewNodeRunner},
};
#[cfg(feature = "headed")]
use bevy_window::{PrimaryWindow, WindowResized};

#[cfg(feature = "gpu_palette")]
use crate::filter::CxFilter;

use crate::{
    palette::{Palette, PaletteHandle},
    position::CxLayer,
    prelude::*,
};

/// Marker component for cameras that should **not** run the `CxPlugin` render
/// pass. Attach this to a `Camera2d` to let Bevy gizmos render on top of
/// pixel-art output without being overwritten by the fullscreen quad.
///
/// # Example
///
/// ```ignore
/// commands.spawn((
///     Camera2d,
///     Camera { order: 1, clear_color: ClearColorConfig::None, ..default() },
///     CxOverlayCamera,
/// ));
/// ```
#[cfg(feature = "headed")]
#[derive(
    Component,
    Clone,
    Copy,
    Debug,
    Default,
    Reflect,
    bevy_render::extract_component::ExtractComponent,
)]
pub struct CxOverlayCamera;

#[cfg(feature = "gpu_palette")]
use gpu_sprite::{CxGpuSpriteBuffer, CxGpuSpriteNode, CxGpuSpritePipeline};
#[cfg(feature = "headed")]
use node::CxRenderNode;
#[cfg(feature = "headed")]
use pipeline::{CxPipeline, CxRenderBuffer, CxUniformBuffer, prepare_uniform};

#[cfg(feature = "headed")]
const SCREEN_SHADER_HANDLE: Handle<Shader> = uuid_handle!("48CE4F2C-8B78-5954-08A8-461F62E10E84");
#[cfg(feature = "gpu_palette")]
const GPU_SPRITE_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("1845F452-7396-4858-A665-9FC8B796AF31");

#[cfg(feature = "gpu_palette")]
#[derive(Resource, Default)]
pub(crate) struct CxLayerOrder<L: CxLayer> {
    inner: RwLock<Vec<L>>,
}

#[cfg(feature = "gpu_palette")]
impl<L: CxLayer> CxLayerOrder<L> {
    pub(crate) fn set(&self, layers: Vec<L>) {
        let mut inner = self.inner.write().unwrap();
        if *inner != layers {
            *inner = layers;
        }
    }

    pub(crate) fn read(&self) -> std::sync::RwLockReadGuard<'_, Vec<L>> {
        self.inner.read().unwrap()
    }
}

pub(crate) struct Plug<L: CxLayer> {
    size: CxScreenSize,
    _l: PhantomData<L>,
}

impl<L: CxLayer> Plug<L> {
    pub(crate) fn new(size: CxScreenSize) -> Self {
        Self {
            size,
            _l: PhantomData,
        }
    }
}

/// Register ECS-side screen systems (palette init + palette update).
/// The startup system that creates the [`CxScreen`] resource is **not** included —
/// callers pick [`insert_screen`] (headed) or [`insert_screen_headless`] (headless).
pub(crate) fn plug_core(app: &mut App) {
    app.add_systems(Update, init_screen)
        .add_systems(PostUpdate, update_screen_palette);
}

impl<L: CxLayer> Plugin for Plug<L> {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "headed")]
        {
            use bevy_render::extract_component::ExtractComponentPlugin;
            app.add_plugins(ExtractResourcePlugin::<CxScreen>::default());
            app.add_plugins(ExtractComponentPlugin::<CxOverlayCamera>::default());
        }

        plug_core(app);
        app.add_systems(Startup, insert_screen(self.size));

        // R-A workaround
        #[cfg(feature = "headed")]
        {
            app.add_systems(PostUpdate, resize_screen);
            let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
            let _ = Assets::insert(
                &mut shaders,
                SCREEN_SHADER_HANDLE.id(),
                Shader::from_wgsl(include_str!("screen.wgsl"), "screen.wgsl"),
            );
            #[cfg(feature = "gpu_palette")]
            let _ = Assets::insert(
                &mut shaders,
                GPU_SPRITE_SHADER_HANDLE.id(),
                Shader::from_wgsl(include_str!("gpu_sprite.wgsl"), "gpu_sprite.wgsl"),
            );
        }

        #[cfg(feature = "headed")]
        {
            let render_app = app.sub_app_mut(RenderApp);
            render_app.add_render_graph_node::<ViewNodeRunner<CxRenderNode<L>>>(Core2d, CxRender);
            #[cfg(feature = "gpu_palette")]
            render_app.add_render_graph_node::<ViewNodeRunner<CxGpuSpriteNode<L>>>(
                Core2d,
                CxGpuSpriteRender,
            );
            #[cfg(feature = "gpu_palette")]
            render_app.add_render_graph_edges(
                Core2d,
                (
                    Node2d::Tonemapping,
                    CxRender,
                    CxGpuSpriteRender,
                    Node2d::EndMainPassPostProcessing,
                ),
            );
            #[cfg(not(feature = "gpu_palette"))]
            render_app.add_render_graph_edges(
                Core2d,
                (
                    Node2d::Tonemapping,
                    CxRender,
                    Node2d::EndMainPassPostProcessing,
                ),
            );

            render_app
                .init_resource::<CxUniformBuffer>()
                .add_systems(Render, prepare_uniform.in_set(RenderSystems::Prepare));
        }
    }

    fn finish(&self, app: &mut App) {
        #[cfg(feature = "headed")]
        app.sub_app_mut(RenderApp)
            .init_resource::<CxPipeline>()
            .init_resource::<CxRenderBuffer>();
        #[cfg(feature = "gpu_palette")]
        app.sub_app_mut(RenderApp)
            .init_resource::<CxGpuSpritePipeline>()
            .init_resource::<CxGpuSpriteBuffer>()
            .init_resource::<CxLayerOrder<L>>();
    }
}

/// Render-target size strategy for the pixel canvas.
#[derive(Clone, Copy, Debug)]
pub enum CxScreenSize {
    /// The screen will have the given dimensions, which is scaled up to fit the window, preserving
    /// the given dimensions' aspect ratio
    Fixed(UVec2),
    /// The screen will match the aspect ratio of the window, with an area of at least as many
    /// pixels as given
    MinPixels(u32),
}

impl From<UVec2> for CxScreenSize {
    fn from(value: UVec2) -> Self {
        Self::Fixed(value)
    }
}

impl CxScreenSize {
    fn compute(self, window_size: Vec2) -> UVec2 {
        use CxScreenSize::{Fixed, MinPixels};

        match self {
            Fixed(size) => size,
            MinPixels(pixels) => {
                let pixels = pixels as f32;
                let width = (window_size.x * pixels / window_size.y).sqrt();
                let height = pixels / width;

                UVec2::new(width as u32, height as u32)
            }
        }
    }
}

/// Resource holding render-target metadata: size, computed dimensions, and palette cache.
#[cfg_attr(feature = "headed", derive(ExtractResource))]
#[derive(Resource, Clone, Debug)]
pub struct CxScreen {
    pub(crate) size: CxScreenSize,
    pub(crate) computed_size: UVec2,
    window_aspect_ratio: f32,
    pub(crate) palette: [Vec3; 256],
    // pub(crate) palette_tree: ImmutableKdTree<f32, 3>,
}

impl CxScreen {
    /// Computed size of the screen
    #[must_use]
    pub fn size(&self) -> UVec2 {
        self.computed_size
    }

    #[cfg(test)]
    pub(crate) fn test_resource(computed_size: UVec2) -> Self {
        Self {
            size: CxScreenSize::Fixed(computed_size),
            computed_size,
            window_aspect_ratio: 1.0,
            palette: [Vec3::ZERO; 256],
        }
    }
}

#[cfg(feature = "gpu_palette")]
pub(crate) fn gpu_sprite_supported(frame: Option<CxFrameView>, filter: Option<&CxFilter>) -> bool {
    if filter.is_some() {
        return false;
    }

    match frame {
        Some(frame) => !matches!(frame.transition, CxFrameTransition::Dither),
        None => true,
    }
}

#[cfg(feature = "gpu_palette")]
pub(crate) fn gpu_composite_supported(
    composite: &CxCompositeSprite,
    frame: Option<CxFrameView>,
    filter: Option<&CxFilter>,
) -> bool {
    if !gpu_sprite_supported(frame, filter) {
        return false;
    }

    composite.parts.iter().all(|part| {
        part.filter.is_none()
            && !part.flip_x
            && !part.flip_y
            && matches!(part.source, crate::sprite::CxCompositePartSource::Sprite(_))
    })
}

pub(crate) fn screen_scale(screen_size: UVec2, window_size: Vec2) -> Vec2 {
    let aspect = screen_size.y as f32 / screen_size.x as f32;

    Vec2::from(if window_size.y > aspect * window_size.x {
        (window_size.x, window_size.x * aspect)
    } else {
        (window_size.y / aspect, window_size.y)
    })
}

/// Canonical transform from Carapace screen pixel coordinates into Bevy overlay
/// world coordinates for a [`CxOverlayCamera`].
///
/// This matches the same fit-to-window model used by Carapace's fullscreen
/// presentation quad: the logical screen is scaled uniformly to the largest
/// centred rectangle that fits inside the current window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CxOverlayViewportTransform {
    screen_size: Vec2,
    viewport_size: Vec2,
}

impl CxOverlayViewportTransform {
    /// Build the transform for a logical Carapace screen inside the current
    /// window/backbuffer size.
    #[must_use]
    pub fn new(screen_size: UVec2, window_size: Vec2) -> Self {
        Self {
            screen_size: screen_size.as_vec2(),
            viewport_size: screen_scale(screen_size, window_size),
        }
    }

    /// Build the transform from the active [`CxScreen`] resource.
    #[must_use]
    pub fn from_screen(screen: &CxScreen, window_size: Vec2) -> Self {
        Self::new(screen.size(), window_size)
    }

    /// Size of the onscreen viewport rectangle occupied by the Carapace output.
    #[must_use]
    pub fn viewport_size(self) -> Vec2 {
        self.viewport_size
    }

    /// Uniform screen-to-overlay scale on each axis.
    #[must_use]
    pub fn scale(self) -> Vec2 {
        self.viewport_size / self.screen_size
    }

    /// Centred overlay-space rectangle occupied by the Carapace output.
    #[must_use]
    pub fn viewport_rect(self) -> Rect {
        Rect::from_center_size(Vec2::ZERO, self.viewport_size)
    }

    /// Convert a Carapace screen pixel coordinate into Bevy overlay world space.
    #[must_use]
    pub fn point(self, point: Vec2) -> Vec2 {
        (point - self.screen_size * 0.5) * self.scale()
    }

    /// Convert an X coordinate from Carapace screen pixels into overlay space.
    #[must_use]
    pub fn point_x(self, x: f32) -> f32 {
        self.point(Vec2::new(x, 0.0)).x
    }

    /// Convert a Y coordinate from Carapace screen pixels into overlay space.
    #[must_use]
    pub fn point_y(self, y: f32) -> f32 {
        self.point(Vec2::new(0.0, y)).y
    }

    /// Convert a size / delta from Carapace pixel units into overlay-space units.
    #[must_use]
    pub fn delta(self, delta: Vec2) -> Vec2 {
        delta * self.scale()
    }

    /// Convert an X delta from Carapace pixels into overlay-space units.
    #[must_use]
    pub fn delta_x(self, x: f32) -> f32 {
        self.delta(Vec2::new(x, 0.0)).x
    }

    /// Convert a Y delta from Carapace pixels into overlay-space units.
    #[must_use]
    pub fn delta_y(self, y: f32) -> f32 {
        self.delta(Vec2::new(0.0, y)).y
    }
}

#[cfg(feature = "headed")]
type Windows<'w, 's> = Query<'w, 's, &'static Window, With<PrimaryWindow>>;
#[cfg(not(feature = "headed"))]
type Windows<'w, 's> = Query<'w, 's, ()>;

fn insert_screen(size: CxScreenSize) -> impl Fn(Windows, Commands) -> Result {
    move |windows, mut commands| {
        #[cfg(feature = "headed")]
        let (computed_size, window_aspect_ratio) = {
            let window = windows.single()?;
            (
                size.compute(Vec2::new(window.width(), window.height())),
                window.width() / window.height(),
            )
        };

        #[cfg(not(feature = "headed"))]
        let (computed_size, window_aspect_ratio) = (size.compute(Vec2::new(500., 500.)), 1.);

        commands.insert_resource(CxScreen {
            size,
            computed_size,
            window_aspect_ratio,
            palette: [Vec3::ZERO; 256],
            // palette_tree: ImmutableKdTree::from(&[][..]),
        });

        OK
    }
}

/// Headless variant of [`insert_screen`] that uses a default window size (no actual window
/// required). For use with [`CxPlugin::build_headless`](crate::CxPlugin::build_headless).
pub(crate) fn insert_screen_headless(size: CxScreenSize) -> impl Fn(Commands) {
    move |mut commands| {
        let (computed_size, window_aspect_ratio) = (size.compute(Vec2::new(500., 500.)), 1.);
        commands.insert_resource(CxScreen {
            size,
            computed_size,
            window_aspect_ratio,
            palette: [Vec3::ZERO; 256],
        });
    }
}

fn init_screen(
    mut initialized: Local<bool>,
    palette: Res<PaletteHandle>,
    palettes: Res<Assets<Palette>>,
    mut screen: ResMut<CxScreen>,
) {
    if *initialized {
        return;
    }

    let Some(palette) = palettes.get(&**palette) else {
        return;
    };

    let mut screen_palette = [Vec3::ZERO; 256];

    for (i, [r, g, b]) in palette.colors.iter().enumerate() {
        screen_palette[i] = Color::srgb_u8(*r, *g, *b).to_linear().to_vec3();
    }

    screen.palette = screen_palette;

    *initialized = true;
}

#[cfg(feature = "headed")]
fn resize_screen(mut window_resized: MessageReader<WindowResized>, mut screen: ResMut<CxScreen>) {
    if let Some(window_resized) = window_resized.read().last() {
        screen.computed_size = screen
            .size
            .compute(Vec2::new(window_resized.width, window_resized.height));
        screen.window_aspect_ratio = window_resized.width / window_resized.height;
    }
}

#[cfg(feature = "headed")]
#[derive(RenderLabel, Hash, Eq, PartialEq, Clone, Debug)]
struct CxRender;

#[cfg(feature = "gpu_palette")]
#[derive(RenderLabel, Hash, Eq, PartialEq, Clone, Debug)]
struct CxGpuSpriteRender;

fn update_screen_palette(
    mut waiting_for_load: Local<bool>,
    palette_handle: Res<PaletteHandle>,
    mut screen: ResMut<CxScreen>,
    palette: Res<PaletteHandle>,
    palettes: Res<Assets<Palette>>,
) {
    if !palette_handle.is_changed() && !*waiting_for_load {
        return;
    }

    let Some(palette) = palettes.get(&**palette) else {
        *waiting_for_load = true;
        return;
    };

    let mut screen_palette = [Vec3::ZERO; 256];

    for (i, [r, g, b]) in palette.colors.iter().enumerate() {
        screen_palette[i] = Color::srgb_u8(*r, *g, *b).to_linear().to_vec3();
    }

    screen.palette = screen_palette;

    *waiting_for_load = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_transform_matches_centered_uniform_fit() {
        let transform =
            CxOverlayViewportTransform::new(UVec2::new(160, 144), Vec2::new(640.0, 576.0));

        assert_eq!(transform.scale(), Vec2::splat(4.0));
        assert_eq!(
            transform.viewport_rect(),
            Rect::from_center_size(Vec2::ZERO, Vec2::new(640.0, 576.0))
        );
        assert_eq!(transform.point(Vec2::new(80.0, 72.0)), Vec2::ZERO);
        assert_eq!(
            transform.point(Vec2::new(160.0, 144.0)),
            Vec2::new(320.0, 288.0)
        );
    }

    #[test]
    fn overlay_transform_preserves_letterboxed_viewport_width() {
        let transform =
            CxOverlayViewportTransform::new(UVec2::new(160, 144), Vec2::new(896.0, 576.0));

        assert_eq!(transform.scale(), Vec2::splat(4.0));
        assert_eq!(transform.viewport_size(), Vec2::new(640.0, 576.0));
        assert_eq!(
            transform.viewport_rect(),
            Rect::from_center_size(Vec2::ZERO, Vec2::new(640.0, 576.0))
        );
    }
}
