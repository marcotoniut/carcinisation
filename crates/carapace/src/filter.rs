//! Filtering
#![allow(clippy::items_after_test_module)]

use std::{error::Error, ops::RangeInclusive, path::PathBuf};

use bevy_asset::{AssetLoader, LoadContext, io::Reader, uuid_handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{lifecycle::HookContext, world::DeferredWorld};
use bevy_image::{CompressedImageFormats, ImageLoader, ImageLoaderSettings};
use bevy_math::uvec2;
use bevy_reflect::TypePath;
#[cfg(feature = "headed")]
use bevy_render::{
    Extract, RenderApp,
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin},
    sync_component::SyncComponentPlugin,
    sync_world::RenderEntity,
};

use crate::{
    animation::AnimatedAssetComponent,
    frame::{Frames, draw_frame},
    image::{CxImage, CxImageSliceMut},
    palette::Palette,
    position::CxLayer,
    prelude::*,
};

/// A built-in filter asset that leaves pixels unchanged.
pub const TRANSPARENT_FILTER: Handle<CxFilterAsset> =
    uuid_handle!("798C57A4-A83C-5DD6-8FA6-1426E31A84CA");

pub(crate) fn plug_core<L: CxLayer>(app: &mut App, palette_path: PathBuf) {
    app.init_asset::<CxFilterAsset>()
        .register_asset_loader(CxFilterLoader::new(palette_path));
    app.insert_resource(InsertDefaultCxFilterLayers::new::<L>());

    // R-A workaround
    let _ = Assets::insert(
        &mut app.world_mut().resource_mut::<Assets<CxFilterAsset>>(),
        TRANSPARENT_FILTER.id(),
        CxFilterAsset(CxImage::empty(uvec2(16, 16))),
    );
}

pub(crate) fn plug<L: CxLayer>(app: &mut App, palette_path: PathBuf) {
    #[cfg(feature = "headed")]
    app.add_plugins((
        RenderAssetPlugin::<CxFilterAsset>::default(),
        SyncComponentPlugin::<CxFilterLayers<L>>::default(),
    ));

    plug_core::<L>(app, palette_path);

    #[cfg(feature = "headed")]
    app.sub_app_mut(RenderApp)
        .insert_resource(InsertDefaultCxFilterLayers::new::<L>())
        .add_systems(ExtractSchedule, extract_filters::<L>);
}

#[derive(TypePath)]
struct CxFilterLoader {
    palette_path: PathBuf,
}

impl CxFilterLoader {
    fn new(palette_path: PathBuf) -> Self {
        Self { palette_path }
    }
}

impl AssetLoader for CxFilterLoader {
    type Asset = CxFilterAsset;
    type Settings = ImageLoaderSettings;
    type Error = Box<dyn Error + Send + Sync>;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &ImageLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<CxFilterAsset, Self::Error> {
        let image = ImageLoader::new(CompressedImageFormats::NONE)
            .load(reader, settings, load_context)
            .await?;
        let palette = load_context
            .loader()
            .immediate()
            .load::<Palette>(self.palette_path.clone())
            .await
            .map_err(|err| err.to_string())?;
        let palette = palette.get();
        let indices = CxImage::palette_indices(palette, &image).map_err(|err| err.to_string())?;

        let mut filter = Vec::with_capacity(indices.area());
        let frame_size = palette.size;
        let frame_area = frame_size.x * frame_size.y;
        let filter_width = image.texture_descriptor.size.width;
        let frame_filter_width = filter_width / palette.size.x;

        if frame_filter_width == 0 {
            return Err("filter size is not a multiple of palette size".into());
        }

        let mut frame_visible = true;

        for i in 0..indices.area() {
            let frame_index = i as u32 / frame_area;
            let frame_pos = i as u32 % frame_area;

            if frame_pos == 0 {
                if !frame_visible {
                    for _ in 0..frame_area {
                        filter.pop();
                    }
                    break;
                }

                frame_visible = false;
            }

            let index = indices.pixel(
                (UVec2::new(
                    frame_index % frame_filter_width,
                    frame_index / frame_filter_width,
                ) * frame_size
                    + UVec2::new(frame_pos % frame_size.x, frame_pos / frame_size.x))
                .as_ivec2(),
            );

            if index == 0 {
                frame_visible = true;
            }

            filter.push(index);
        }

        Ok(CxFilterAsset(CxImage::new(filter, frame_area as usize)))
    }

    fn extensions(&self) -> &[&str] {
        &["px_filter.png"]
    }
}

/// Palette-index remapping table (a "palette swap" / "colour remap").
///
/// Each pixel in the source image encodes a mapping: the pixel's position
/// corresponds to an input palette index, and its colour value is the
/// output index.  At render time, every palette index passing through the
/// filter is remapped via this table.
///
/// To apply a filter to a single entity, add [`CxFilter`] (a handle
/// wrapper).  To apply one to entire layers, spawn a [`CxFilterLayers`].
///
/// # Animated filters
///
/// For animated filters, tile multiple remapping frames in the source
/// image from the top-left corner, moving rightwards and wrapping
/// downwards.  See `assets/fade_to_black.png` for an example.
///
/// # Loading
///
/// Create a handle through your asset wrapper. The source image must
/// contain only colours present in the active palette.
#[doc(alias = "palette swap")]
#[doc(alias = "palette remap")]
#[doc(alias = "colour remap")]
#[derive(Asset, Clone, Reflect, Debug)]
pub struct CxFilterAsset(pub(crate) CxImage);

#[cfg(feature = "headed")]
impl RenderAsset for CxFilterAsset {
    type SourceAsset = Self;
    type Param = ();

    fn prepare_asset(
        source_asset: Self,
        _: AssetId<Self>,
        &mut (): &mut (),
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self>> {
        Ok(source_asset)
    }
}

impl Frames for CxFilterAsset {
    type Param = ();

    fn frame_count(&self) -> usize {
        let Self(filter) = self;
        filter.area() / filter.width()
    }

    fn draw(
        &self,
        (): (),
        image: &mut CxImageSliceMut,
        frame: impl Fn(UVec2) -> usize,
        _: impl Fn(u8) -> u8,
    ) {
        let Self(filter) = self;
        let width = image.slice.width() as u32;
        image.for_each_mut(|index, _, pixel| {
            let index = index as u32;
            *pixel = filter.pixel(IVec2::new(
                i32::from(*pixel),
                frame(UVec2::new(index % width, index / width)) as i32,
            ));
        });
    }
}

impl CxFilterAsset {
    pub(crate) fn as_fn(&self) -> impl '_ + Fn(u8) -> u8 {
        let Self(filter) = self;
        |pixel| filter.pixel(IVec2::new(i32::from(pixel), 0))
    }
}

/// Applies a [`CxFilterAsset`] (palette-index remap) to the entity.
#[derive(Component, Deref, DerefMut, Default, Clone, Debug, Reflect)]
pub struct CxFilter(pub Handle<CxFilterAsset>);

impl CxFilter {
    /// Look up the loaded [`CxFilterAsset`] from render assets.
    #[cfg(feature = "headed")]
    #[must_use]
    pub(crate) fn resolve<'a>(
        &self,
        assets: &'a bevy_render::render_asset::RenderAssets<CxFilterAsset>,
    ) -> Option<&'a CxFilterAsset> {
        assets.get(&**self)
    }
}

/// Resolve an optional filter reference against loaded assets.
#[cfg(feature = "headed")]
#[must_use]
pub(crate) fn resolve_filter<'a>(
    filter: Option<&'a CxFilter>,
    assets: &'a bevy_render::render_asset::RenderAssets<CxFilterAsset>,
) -> Option<&'a CxFilterAsset> {
    filter.and_then(|f| f.resolve(assets))
}

impl AnimatedAssetComponent for CxFilter {
    type Asset = CxFilterAsset;

    fn handle(&self) -> &Handle<CxFilterAsset> {
        self
    }

    fn max_frame_count(asset: &CxFilterAsset) -> usize {
        asset.frame_count()
    }
}

/// Determines which layers a filter appies to. Range and Many filters always clip to entity
/// pixels; use `Single` with `clip: false` to filter the composed layer instead.
#[derive(Component, Clone)]
#[require(CxFilter)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub enum CxFilterLayers<L: CxLayer> {
    /// Filter applies to a single layer
    Single {
        /// Layer the filter appies to
        layer: L,
        /// If `true`, the filter will apply only to the entities on that layer,
        /// before it is rendered. If `false`, it will apply to the entire image when the layer
        /// is rendered, including the background color.
        clip: bool,
    },
    /// Filter applies to a range of layers. Uses layer ordering for enum variants with data.
    /// Always clips to entity pixels.
    Range(RangeInclusive<L>),
    /// Filter applies to a set list of layers. Always clips to entity pixels.
    Many(Vec<L>),
}

impl<L: CxLayer> Default for CxFilterLayers<L> {
    fn default() -> Self {
        Self::single_clip(default())
    }
}

impl<L: CxLayer> From<RangeInclusive<L>> for CxFilterLayers<L> {
    fn from(range: RangeInclusive<L>) -> Self {
        Self::Range(range)
    }
}

impl<L: CxLayer> CxFilterLayers<L> {
    /// Creates a [`CxFilterLayers::Single`] with the given layer, with clip enabled
    pub fn single_clip(layer: L) -> Self {
        Self::Single { layer, clip: true }
    }

    /// Creates a [`CxFilterLayers::Single`] with the given layer, with clip disabled
    pub fn single_over(layer: L) -> Self {
        Self::Single { layer, clip: false }
    }

    /// Creates a [`CxFilterLayers::Range`] that clips to entity pixels.
    pub fn range_clip(range: RangeInclusive<L>) -> Self {
        Self::Range(range)
    }

    /// Creates a [`CxFilterLayers::Many`] that clips to entity pixels.
    pub fn many_clip(layers: impl Into<Vec<L>>) -> Self {
        Self::Many(layers.into())
    }
}

#[derive(Resource, Deref)]
struct InsertDefaultCxFilterLayers(Box<dyn Fn(bool, &mut EntityWorldMut) + Send + Sync>);

impl InsertDefaultCxFilterLayers {
    fn new<L: CxLayer>() -> Self {
        Self(Box::new(|clip, entity| {
            entity.insert_if_new(CxFilterLayers::Single {
                layer: L::default(),
                clip,
            });
        }))
    }
}

fn insert_default_px_filter_layers(mut world: DeferredWorld, ctx: HookContext) {
    world.commands().queue(move |world: &mut World| {
        let insert_default_px_filter_layers = world
            .remove_resource::<InsertDefaultCxFilterLayers>()
            .unwrap();
        if let Ok(mut entity) = world.get_entity_mut(ctx.entity)
            && let Some(default) = entity.get::<DefaultCxFilterLayers>()
        {
            insert_default_px_filter_layers(default.clip, entity.remove::<DefaultCxFilterLayers>());
        }
        world.insert_resource(insert_default_px_filter_layers);
    });
}

#[derive(Component)]
#[component(on_add = insert_default_px_filter_layers)]
pub(crate) struct DefaultCxFilterLayers {
    pub(crate) clip: bool,
}

impl Default for DefaultCxFilterLayers {
    fn default() -> Self {
        Self { clip: true }
    }
}

/// Marks that a filter should apply outside a shape rather than inside it.
#[derive(Component, Default, Reflect)]
pub struct CxInvertMask;

pub(crate) type FilterComponents<L> = (
    &'static CxFilter,
    &'static CxFilterLayers<L>,
    Option<&'static CxFrameView>,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{frame::draw_frame, image::CxImage};

    fn pixels(image: &CxImage) -> Vec<u8> {
        let size = image.size();
        let mut out = Vec::with_capacity((size.x * size.y) as usize);
        for y in 0..size.y as i32 {
            for x in 0..size.x as i32 {
                out.push(image.pixel(IVec2::new(x, y)));
            }
        }
        out
    }

    #[test]
    fn filter_maps_palette_indices() {
        let filter = CxFilterAsset(CxImage::new(vec![0, 2, 3, 1], 4));
        let mut image = CxImage::new(vec![1, 2, 1, 2], 2);
        let mut slice = image.slice_all_mut();

        draw_frame(&filter, (), &mut slice, None, []);

        let expected = vec![2, 3, 2, 3];
        assert_eq!(pixels(&image), expected);
    }
}

#[cfg(feature = "headed")]
fn extract_filters<L: CxLayer>(
    filters: Extract<
        Query<(FilterComponents<L>, &InheritedVisibility, RenderEntity), Without<CxRenderSpace>>,
    >,
    mut cmd: Commands,
) {
    for ((filter, layers, frame), visibility, id) in &filters {
        let mut entity = cmd.entity(id);

        if !visibility.get() {
            entity.remove::<CxFilterLayers<L>>();
            continue;
        }

        entity.insert((filter.clone(), layers.clone()));

        if let Some(frame) = frame {
            entity.insert(*frame);
        } else {
            entity.remove::<CxFrameView>();
        }
    }
}

pub(crate) fn draw_filter(
    filter: &CxFilterAsset,
    frame: Option<CxFrameView>,
    image: &mut CxImageSliceMut,
) {
    draw_frame(filter, (), image, frame, []);
}
