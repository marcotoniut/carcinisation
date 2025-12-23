//! Screen and rendering
//!
//! Data flow: gather render-world components by layer, draw into a CPU `PxImage`,
//! then upload to a reusable `R8Uint` texture and present via a fullscreen quad.
//! This is the single compositing path for sprites, text, tilemaps, rects, lines, and filters.

mod draw;
mod node;
mod pipeline;

use std::marker::PhantomData;

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

use crate::{
    palette::{Palette, PaletteHandle},
    position::PxLayer,
    prelude::*,
};

use node::PxRenderNode;
use pipeline::{PxPipeline, PxRenderBuffer, PxUniformBuffer, prepare_uniform};

#[cfg(feature = "headed")]
const SCREEN_SHADER_HANDLE: Handle<Shader> = uuid_handle!("48CE4F2C-8B78-5954-08A8-461F62E10E84");

pub(crate) struct Plug<L: PxLayer> {
    size: ScreenSize,
    _l: PhantomData<L>,
}

impl<L: PxLayer> Plug<L> {
    pub(crate) fn new(size: ScreenSize) -> Self {
        Self {
            size,
            _l: PhantomData,
        }
    }
}

impl<L: PxLayer> Plugin for Plug<L> {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "headed")]
        app.add_plugins(ExtractResourcePlugin::<Screen>::default());

        app.add_systems(Startup, insert_screen(self.size))
            .add_systems(Update, init_screen)
            .add_systems(PostUpdate, update_screen_palette);

        // R-A workaround
        #[cfg(feature = "headed")]
        let _ = Assets::insert(
            &mut app
                .add_systems(PostUpdate, resize_screen)
                .world_mut()
                .resource_mut::<Assets<Shader>>(),
            SCREEN_SHADER_HANDLE.id(),
            Shader::from_wgsl(include_str!("screen.wgsl"), "screen.wgsl"),
        );

        #[cfg(feature = "headed")]
        app.sub_app_mut(RenderApp)
            .add_render_graph_node::<ViewNodeRunner<PxRenderNode<L>>>(Core2d, PxRender)
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::Tonemapping,
                    PxRender,
                    Node2d::EndMainPassPostProcessing,
                ),
            )
            .init_resource::<PxUniformBuffer>()
            .add_systems(Render, prepare_uniform.in_set(RenderSystems::Prepare));
    }

    fn finish(&self, _app: &mut App) {
        #[cfg(feature = "headed")]
        _app.sub_app_mut(RenderApp)
            .init_resource::<PxPipeline>()
            .init_resource::<PxRenderBuffer>();
    }
}

/// Size of the image which `carapace` draws to
#[derive(Clone, Copy, Debug)]
pub enum ScreenSize {
    /// The screen will have the given dimensions, which is scaled up to fit the window, preserving
    /// the given dimensions' aspect ratio
    Fixed(UVec2),
    /// The screen will match the aspect ratio of the window, with an area of at least as many
    /// pixels as given
    MinPixels(u32),
}

impl From<UVec2> for ScreenSize {
    fn from(value: UVec2) -> Self {
        Self::Fixed(value)
    }
}

impl ScreenSize {
    fn compute(self, window_size: Vec2) -> UVec2 {
        use ScreenSize::*;

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

/// Metadata for the image that `carapace` draws to
#[cfg_attr(feature = "headed", derive(ExtractResource))]
#[derive(Resource, Clone, Debug)]
pub struct Screen {
    pub(crate) size: ScreenSize,
    pub(crate) computed_size: UVec2,
    window_aspect_ratio: f32,
    pub(crate) palette: [Vec3; 256],
    // pub(crate) palette_tree: ImmutableKdTree<f32, 3>,
}

impl Screen {
    /// Computed size of the screen
    pub fn size(&self) -> UVec2 {
        self.computed_size
    }
}

pub(crate) fn screen_scale(screen_size: UVec2, window_size: Vec2) -> Vec2 {
    let aspect = screen_size.y as f32 / screen_size.x as f32;

    Vec2::from(match window_size.y > aspect * window_size.x {
        true => (window_size.x, window_size.x * aspect),
        false => (window_size.y / aspect, window_size.y),
    })
}

#[cfg(feature = "headed")]
type Windows<'w, 's> = Query<'w, 's, &'static Window, With<PrimaryWindow>>;
#[cfg(not(feature = "headed"))]
type Windows<'w, 's> = Query<'w, 's, ()>;

fn insert_screen(size: ScreenSize) -> impl Fn(Windows, Commands) -> Result {
    move |_windows, mut commands| {
        #[cfg(feature = "headed")]
        let (computed_size, window_aspect_ratio) = {
            let window = _windows.single()?;
            (
                size.compute(Vec2::new(window.width(), window.height())),
                window.width() / window.height(),
            )
        };

        #[cfg(not(feature = "headed"))]
        let (computed_size, window_aspect_ratio) = (size.compute(Vec2::new(500., 500.)), 1.);

        commands.insert_resource(Screen {
            size,
            computed_size,
            window_aspect_ratio,
            palette: [Vec3::ZERO; 256],
            // palette_tree: ImmutableKdTree::from(&[][..]),
        });

        OK
    }
}

fn init_screen(
    mut initialized: Local<bool>,
    palette: Res<PaletteHandle>,
    palettes: Res<Assets<Palette>>,
    mut screen: ResMut<Screen>,
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
fn resize_screen(mut window_resized: MessageReader<WindowResized>, mut screen: ResMut<Screen>) {
    if let Some(window_resized) = window_resized.read().last() {
        screen.computed_size = screen
            .size
            .compute(Vec2::new(window_resized.width, window_resized.height));
        screen.window_aspect_ratio = window_resized.width / window_resized.height;
    }
}

#[cfg(feature = "headed")]
#[derive(RenderLabel, Hash, Eq, PartialEq, Clone, Debug)]
struct PxRender;

fn update_screen_palette(
    mut waiting_for_load: Local<bool>,
    palette_handle: Res<PaletteHandle>,
    mut screen: ResMut<Screen>,
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
