//! Sprites

use std::{error::Error, path::PathBuf};

use bevy_asset::{AssetEvent, AssetId, AssetLoader, LoadContext, io::Reader};
use bevy_derive::{Deref, DerefMut};
#[cfg(feature = "gpu_palette")]
use bevy_ecs::system::lifetimeless::SRes;
use bevy_image::{CompressedImageFormats, ImageLoader, ImageLoaderSettings};
use bevy_math::{ivec2, uvec2};
use bevy_reflect::TypePath;
#[cfg(feature = "headed")]
use bevy_render::{
    Extract, RenderApp,
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin},
    sync_component::SyncComponentPlugin,
    sync_world::RenderEntity,
};
#[cfg(feature = "gpu_palette")]
use bevy_render::{
    render_resource::{
        Extent3d, TexelCopyBufferLayout, Texture, TextureDescriptor, TextureDimension,
        TextureFormat, TextureUsages,
    },
    renderer::{RenderDevice, RenderQueue},
};
use serde::{Deserialize, Serialize};

use crate::{
    animation::AnimatedAssetComponent,
    atlas::{AtlasRegion, AtlasRegionId, PxSpriteAtlasAsset},
    filter::PxFilterAsset,
    frame::{Frames, PxFrameBinding, PxFrameCount},
    image::{PxImage, PxImageSliceMut},
    palette::Palette,
    position::{DefaultLayer, PxLayer, Spatial},
    prelude::*,
    set::PxSet,
};

pub(crate) fn plug_core(app: &mut App, palette_path: PathBuf) {
    app.init_asset::<PxSpriteAsset>()
        .register_asset_loader(PxSpriteLoader::new(palette_path));

    app.add_systems(
        PostUpdate,
        (
            update_composite_metrics_on_change,
            update_composite_metrics_on_assets,
            sync_composite_frame_count_on_animation_added,
        )
            .before(PxSet::FinishAnimations),
    );
}

pub(crate) fn plug<L: PxLayer>(app: &mut App, palette_path: PathBuf) {
    #[cfg(feature = "headed")]
    app.add_plugins((
        RenderAssetPlugin::<PxSpriteAsset>::default(),
        SyncComponentPlugin::<PxSprite>::default(),
        SyncComponentPlugin::<PxCompositeSprite>::default(),
    ));

    #[cfg(all(feature = "headed", feature = "gpu_palette"))]
    app.add_plugins((
        RenderAssetPlugin::<PxSpriteGpu>::default(),
        SyncComponentPlugin::<PxGpuSprite>::default(),
        SyncComponentPlugin::<PxGpuComposite>::default(),
    ));

    plug_core(app, palette_path);

    #[cfg(feature = "headed")]
    app.sub_app_mut(RenderApp).add_systems(
        ExtractSchedule,
        (
            extract_sprites::<L>,
            extract_composite_sprites::<L>,
            // extract_image_to_sprites::<L>
        ),
    );
}

#[derive(Serialize, Deserialize)]
struct PxSpriteLoaderSettings {
    frame_count: usize,
    image_loader_settings: ImageLoaderSettings,
}

impl Default for PxSpriteLoaderSettings {
    fn default() -> Self {
        Self {
            frame_count: 1,
            image_loader_settings: default(),
        }
    }
}

#[derive(TypePath)]
struct PxSpriteLoader {
    palette_path: PathBuf,
}

impl PxSpriteLoader {
    fn new(palette_path: PathBuf) -> Self {
        Self { palette_path }
    }
}

impl AssetLoader for PxSpriteLoader {
    type Asset = PxSpriteAsset;
    type Settings = PxSpriteLoaderSettings;
    type Error = Box<dyn Error + Send + Sync>;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &PxSpriteLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<PxSpriteAsset, Self::Error> {
        let image = ImageLoader::new(CompressedImageFormats::NONE)
            .load(reader, &settings.image_loader_settings, load_context)
            .await?;
        let palette = load_context
            .loader()
            .immediate()
            .load::<Palette>(self.palette_path.clone())
            .await
            .map_err(|err| err.to_string())?;
        let data =
            PxImage::palette_indices(palette.get(), &image).map_err(|err| err.to_string())?;

        Ok(PxSpriteAsset {
            frame_size: data.area() / settings.frame_count,
            data,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["px_sprite.png"]
    }
}

/// A sprite. Create a [`Handle<PxSpriteAsset>`] with a [`PxAssets<PxSprite>`] and an image.
/// If the sprite is animated, the frames should be laid out from top to bottom.
/// See `assets/sprite/runner.png` for an example of an animated sprite.
#[derive(Asset, Serialize, Deserialize, Clone, Reflect, Debug)]
pub struct PxSpriteAsset {
    pub(crate) data: PxImage,
    pub(crate) frame_size: usize,
}

#[cfg(feature = "gpu_palette")]
#[derive(Clone)]
pub(crate) struct PxSpriteGpu {
    pub(crate) size: UVec2,
    pub(crate) frame_size: usize,
    pub(crate) texture: Texture,
}

#[cfg(feature = "gpu_palette")]
impl RenderAsset for PxSpriteGpu {
    type SourceAsset = PxSpriteAsset;
    type Param = (SRes<RenderDevice>, SRes<RenderQueue>);

    fn prepare_asset(
        source_asset: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (device, queue): &mut bevy_ecs::system::SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let size = source_asset.data.size();
        let descriptor = TextureDescriptor {
            label: Some("px_sprite_texture"),
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Uint,
            sample_count: 1,
            mip_level_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        };

        let texture = device.create_texture(&descriptor);
        queue.write_texture(
            texture.as_image_copy(),
            source_asset.data.data(),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.x),
                rows_per_image: None,
            },
            descriptor.size,
        );

        Ok(Self {
            size,
            frame_size: source_asset.frame_size,
            texture,
        })
    }
}

#[cfg(feature = "headed")]
impl RenderAsset for PxSpriteAsset {
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

impl Frames for PxSpriteAsset {
    type Param = ();

    fn frame_count(&self) -> usize {
        self.data.area() / self.frame_size
    }

    fn draw(
        &self,
        (): (),
        image: &mut PxImageSliceMut,
        frame: impl Fn(UVec2) -> usize,
        filter: impl Fn(u8) -> u8,
    ) {
        let width = self.data.width();
        let image_width = image.image_width();
        image.for_each_mut(|slice_i, image_i, pixel| {
            if let Some(value) = self.data.get_pixel(ivec2(
                (slice_i % width) as i32,
                ((frame(uvec2(
                    (image_i % image_width) as u32,
                    (image_i / image_width) as u32,
                )) * self.frame_size
                    + slice_i)
                    / width) as i32,
            )) && value != 0
            {
                *pixel = filter(value);
            }
        });
    }
}

impl Spatial for PxSpriteAsset {
    fn frame_size(&self) -> UVec2 {
        UVec2::new(
            self.data.width() as u32,
            (self.frame_size / self.data.width()) as u32,
        )
    }
}

impl Frames for PxResolvedCompositePart<'_> {
    type Param = ();

    fn frame_count(&self) -> usize {
        match self {
            Self::Sprite(sprite) => sprite.frame_count(),
            Self::AtlasRegion { region, .. } => region.frame_count(),
        }
    }

    fn draw(
        &self,
        (): (),
        image: &mut PxImageSliceMut,
        frame: impl Fn(UVec2) -> usize,
        filter: impl Fn(u8) -> u8,
    ) {
        match self {
            Self::Sprite(sprite) => sprite.draw((), image, frame, filter),
            Self::AtlasRegion { atlas, region } => (*atlas, *region).draw((), image, frame, filter),
        }
    }
}

impl Spatial for PxResolvedCompositePart<'_> {
    fn frame_size(&self) -> UVec2 {
        match self {
            Self::Sprite(sprite) => sprite.frame_size(),
            Self::AtlasRegion { region, .. } => region.frame_size,
        }
    }
}

/// A sprite
#[derive(Component, Deref, DerefMut, Default, Clone, Debug, Reflect)]
#[require(PxPosition, PxAnchor, DefaultLayer, PxCanvas)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct PxSprite(pub Handle<PxSpriteAsset>);

impl From<Handle<PxSpriteAsset>> for PxSprite {
    fn from(value: Handle<PxSpriteAsset>) -> Self {
        Self(value)
    }
}

/// A sprite composed of multiple sprite or atlas-backed parts.
///
/// The CPU renderer supports all composite part sources and per-part flips. The optional
/// [`PxGpuComposite`] path is a narrower optimization subset: it currently supports only
/// sprite-backed parts with no per-part filter and no flips.
#[derive(Component, Default, Clone, Debug)]
#[require(PxPosition, PxAnchor, DefaultLayer, PxCanvas)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct PxCompositeSprite {
    /// Parts that make up the composite sprite.
    pub parts: Vec<PxCompositePart>,
    /// Cached composite size (computed from parts).
    pub size: UVec2,
    /// Cached origin shift when parts have negative offsets.
    pub origin: IVec2,
    /// Cached frame count for the master animation.
    pub frame_count: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PxCompositeMetrics {
    pub size: UVec2,
    pub origin: IVec2,
    pub frame_count: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PxCompositePartMetrics {
    pub size: UVec2,
    pub frame_count: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum PxResolvedCompositePart<'a> {
    Sprite(&'a PxSpriteAsset),
    AtlasRegion {
        atlas: &'a PxSpriteAtlasAsset,
        region: &'a AtlasRegion,
    },
}

#[derive(Clone, Debug)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum PxCompositePartResolveError {
    MissingSpriteAsset(Handle<PxSpriteAsset>),
    MissingAtlasAsset(Handle<PxSpriteAtlasAsset>),
    MissingAtlasRegion {
        atlas: Handle<PxSpriteAtlasAsset>,
        region: AtlasRegionId,
    },
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PxCompositePartDrawable<'a> {
    pub resolved: PxResolvedCompositePart<'a>,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl PxResolvedCompositePart<'_> {
    pub(crate) fn metrics(&self) -> PxCompositePartMetrics {
        PxCompositePartMetrics {
            size: self.frame_size(),
            frame_count: self.frame_count(),
        }
    }

    pub(crate) fn pixel(&self, frame_index: usize, local_pos: UVec2) -> Option<u8> {
        let size = self.frame_size();
        if local_pos.x >= size.x || local_pos.y >= size.y {
            return None;
        }

        match self {
            Self::Sprite(sprite) => {
                let pixel_y = frame_index as i32 * size.y as i32 + local_pos.y as i32;
                sprite
                    .data
                    .get_pixel(IVec2::new(local_pos.x as i32, pixel_y))
            }
            Self::AtlasRegion { atlas, region } => {
                let rect = region.frame(frame_index)?;
                atlas.data.get_pixel(ivec2(
                    rect.x as i32 + local_pos.x as i32,
                    rect.y as i32 + local_pos.y as i32,
                ))
            }
        }
    }

    pub(crate) fn flipped_pixel(
        &self,
        frame_index: usize,
        local_pos: UVec2,
        flip_x: bool,
        flip_y: bool,
    ) -> Option<u8> {
        let size = self.frame_size();
        let mapped = uvec2(
            if flip_x {
                size.x.checked_sub(local_pos.x + 1)?
            } else {
                local_pos.x
            },
            if flip_y {
                size.y.checked_sub(local_pos.y + 1)?
            } else {
                local_pos.y
            },
        );

        self.pixel(frame_index, mapped)
    }
}

impl Frames for PxCompositePartDrawable<'_> {
    type Param = ();

    fn frame_count(&self) -> usize {
        self.resolved.frame_count()
    }

    fn draw(
        &self,
        (): (),
        image: &mut PxImageSliceMut,
        frame: impl Fn(UVec2) -> usize,
        filter: impl Fn(u8) -> u8,
    ) {
        let size = self.frame_size();
        if size.x == 0 || size.y == 0 {
            return;
        }
        let frame_width = size.x as usize;
        let image_width = image.image_width();

        image.for_each_mut(|slice_i, image_i, pixel| {
            let local_pos = uvec2(
                (slice_i % frame_width) as u32,
                (slice_i / frame_width) as u32,
            );
            let frame_index = frame(uvec2(
                (image_i % image_width) as u32,
                (image_i / image_width) as u32,
            ));

            if let Some(value) =
                self.resolved
                    .flipped_pixel(frame_index, local_pos, self.flip_x, self.flip_y)
                && value != 0
            {
                *pixel = filter(value);
            }
        });
    }
}

impl Spatial for PxCompositePartDrawable<'_> {
    fn frame_size(&self) -> UVec2 {
        self.resolved.frame_size()
    }
}

impl PxCompositeSprite {
    /// Create a composite sprite from parts.
    #[must_use]
    pub fn new(parts: Vec<PxCompositePart>) -> Self {
        Self {
            parts,
            size: UVec2::ZERO,
            origin: IVec2::ZERO,
            frame_count: 0,
        }
    }

    /// Recompute cached size/origin/frame count from current parts.
    pub(crate) fn metrics_with<F>(&self, mut get: F) -> Option<PxCompositeMetrics>
    where
        F: FnMut(&PxCompositePartSource) -> Option<PxCompositePartMetrics>,
    {
        let mut any = false;
        let mut min = IVec2::ZERO;
        let mut max = IVec2::ZERO;
        let mut frame_count = 0usize;

        for part in &self.parts {
            let Some(metrics) = get(&part.source) else {
                continue;
            };

            let size = metrics.size.as_ivec2();
            let part_min = part.offset;
            let part_max = part.offset + size;

            if any {
                min = min.min(part_min);
                max = max.max(part_max);
            } else {
                min = part_min;
                max = part_max;
                any = true;
            }

            frame_count = frame_count.max(metrics.frame_count);
        }

        if !any {
            return None;
        }

        let size = max - min;
        Some(PxCompositeMetrics {
            origin: min,
            size: UVec2::new(size.x.max(0) as u32, size.y.max(0) as u32),
            frame_count,
        })
    }

    /// Recompute cached size/origin/frame count from current parts using sprite assets only.
    ///
    /// Atlas-backed parts are ignored by this compatibility helper. Use
    /// [`PxCompositeSprite::recompute_metrics_with_atlases`] when a composite may contain atlas
    /// regions.
    pub fn recompute_metrics(&mut self, sprites: &Assets<PxSpriteAsset>) {
        if self
            .parts
            .iter()
            .any(|part| matches!(part.source, PxCompositePartSource::AtlasRegion { .. }))
        {
            warn!(
                "PxCompositeSprite::recompute_metrics() ignores atlas-backed parts; \
                 use recompute_metrics_with_atlases() for atlas-backed composites"
            );
        }
        let atlases = Assets::<PxSpriteAtlasAsset>::default();
        self.recompute_metrics_with_atlases(sprites, &atlases);
    }

    /// Recompute cached size/origin/frame count from current parts using sprite and atlas assets.
    pub fn recompute_metrics_with_atlases(
        &mut self,
        sprites: &Assets<PxSpriteAsset>,
        atlases: &Assets<PxSpriteAtlasAsset>,
    ) {
        let metrics = self.metrics_with(|source| {
            source
                .resolve(|handle| sprites.get(handle), |handle| atlases.get(handle))
                .ok()
                .map(|resolved| resolved.metrics())
        });
        if let Some(metrics) = metrics {
            self.size = metrics.size;
            self.origin = metrics.origin;
            self.frame_count = metrics.frame_count;
        } else {
            self.size = UVec2::ZERO;
            self.origin = IVec2::ZERO;
            self.frame_count = 0;
        }
    }
}

/// Source for a composite part.
///
/// Composite parts stay source-agnostic at the engine layer: a part can draw either a standalone
/// [`PxSpriteAsset`] or a region within a [`PxSpriteAtlasAsset`], referenced by atlas handle and
/// [`AtlasRegionId`].
///
/// For most call sites, prefer [`PxCompositePart::new`] or [`PxCompositePart::atlas_region`] and
/// then configure the part with builder-style helpers. Use this enum directly when you need to
/// construct or store the source separately from the part.
#[derive(Clone, Debug)]
pub enum PxCompositePartSource {
    /// Draw from a standalone sprite asset.
    Sprite(Handle<PxSpriteAsset>),
    /// Draw from a named region within a sprite atlas asset.
    AtlasRegion {
        /// Atlas asset handle.
        atlas: Handle<PxSpriteAtlasAsset>,
        /// Region identifier within the atlas.
        region: AtlasRegionId,
    },
}

impl PxCompositePartSource {
    /// Create a composite part source from a standalone sprite.
    #[must_use]
    pub fn sprite(sprite: Handle<PxSpriteAsset>) -> Self {
        Self::Sprite(sprite)
    }

    /// Create a composite part source from an atlas region.
    #[must_use]
    pub fn atlas_region(atlas: Handle<PxSpriteAtlasAsset>, region: AtlasRegionId) -> Self {
        Self::AtlasRegion { atlas, region }
    }

    pub(crate) fn resolve<'a, FS, FA>(
        &self,
        mut get_sprite: FS,
        mut get_atlas: FA,
    ) -> Result<PxResolvedCompositePart<'a>, PxCompositePartResolveError>
    where
        FS: FnMut(&Handle<PxSpriteAsset>) -> Option<&'a PxSpriteAsset>,
        FA: FnMut(&Handle<PxSpriteAtlasAsset>) -> Option<&'a PxSpriteAtlasAsset>,
    {
        match self {
            Self::Sprite(sprite) => get_sprite(sprite)
                .map(PxResolvedCompositePart::Sprite)
                .ok_or_else(|| PxCompositePartResolveError::MissingSpriteAsset(sprite.clone())),
            Self::AtlasRegion { atlas, region } => {
                let atlas_asset = get_atlas(atlas)
                    .ok_or_else(|| PxCompositePartResolveError::MissingAtlasAsset(atlas.clone()))?;
                let region_asset = atlas_asset.region(*region).ok_or_else(|| {
                    PxCompositePartResolveError::MissingAtlasRegion {
                        atlas: atlas.clone(),
                        region: *region,
                    }
                })?;
                Ok(PxResolvedCompositePart::AtlasRegion {
                    atlas: atlas_asset,
                    region: region_asset,
                })
            }
        }
    }
}

/// A single part of a composite sprite.
#[derive(Clone, Debug)]
pub struct PxCompositePart {
    /// Source asset used for this part.
    pub source: PxCompositePartSource,
    /// Offset in composite-local pixel space from the part's bottom-left corner.
    ///
    /// This is an engine-space, bottom-left-oriented offset. If your asset pipeline or runtime
    /// manifest uses top-left image coordinates, convert into this space before constructing the
    /// part.
    pub offset: IVec2,
    /// Frame binding to the composite's master frame.
    pub frame: PxFrameBinding,
    /// Optional filter applied before the composite's filter.
    pub filter: Option<Handle<PxFilterAsset>>,
    /// Mirror the part horizontally at draw time.
    pub flip_x: bool,
    /// Mirror the part vertically at draw time.
    pub flip_y: bool,
}

impl PxCompositePart {
    /// Create a composite part with default binding and zero offset.
    #[must_use]
    pub fn new(sprite: Handle<PxSpriteAsset>) -> Self {
        Self::from_source(PxCompositePartSource::Sprite(sprite))
    }

    /// Create a composite part from any supported source.
    #[must_use]
    pub fn from_source(source: PxCompositePartSource) -> Self {
        Self {
            source,
            offset: IVec2::ZERO,
            frame: PxFrameBinding::default(),
            filter: None,
            flip_x: false,
            flip_y: false,
        }
    }

    /// Create a composite part that draws from an atlas region.
    #[must_use]
    pub fn atlas_region(atlas: Handle<PxSpriteAtlasAsset>, region: AtlasRegionId) -> Self {
        Self::from_source(PxCompositePartSource::AtlasRegion { atlas, region })
    }

    /// Set the part offset relative to the composite origin.
    #[must_use]
    pub fn with_offset(mut self, offset: IVec2) -> Self {
        self.offset = offset;
        self
    }

    /// Set how this part binds to the composite master frame.
    #[must_use]
    pub fn with_frame_binding(mut self, frame: PxFrameBinding) -> Self {
        self.frame = frame;
        self
    }

    /// Set an optional per-part filter.
    #[must_use]
    pub fn with_filter(mut self, filter: Option<Handle<PxFilterAsset>>) -> Self {
        self.filter = filter;
        self
    }

    /// Set horizontal and vertical flip flags for draw-time mirroring.
    #[must_use]
    pub fn with_flip(mut self, flip_x: bool, flip_y: bool) -> Self {
        self.flip_x = flip_x;
        self.flip_y = flip_y;
        self
    }
}

pub(crate) fn log_composite_part_resolve_error(
    part_index: usize,
    error: &PxCompositePartResolveError,
) {
    match error {
        PxCompositePartResolveError::MissingSpriteAsset(handle) => {
            error!("skipping composite part {part_index}: missing sprite asset {handle:?}");
        }
        PxCompositePartResolveError::MissingAtlasAsset(handle) => {
            error!("skipping composite part {part_index}: missing atlas asset {handle:?}");
        }
        PxCompositePartResolveError::MissingAtlasRegion { atlas, region } => {
            error!(
                "skipping composite part {part_index}: missing atlas region {:?} in atlas {atlas:?}",
                region
            );
        }
    }
}

/// Marker to render a sprite via the experimental GPU palette path.
#[cfg(feature = "gpu_palette")]
#[derive(Component, Default, Clone, Copy, Debug)]
#[require(PxSprite)]
pub struct PxGpuSprite;

/// Marker to render a composite sprite via the experimental GPU palette path.
///
/// This is an optimization subset of [`PxCompositeSprite`], not a separate composite feature set.
/// A composite is GPU-eligible only when every part:
///
/// - uses [`PxCompositePartSource::Sprite`]
/// - has no per-part filter
/// - has `flip_x == false`
/// - has `flip_y == false`
///
/// Composites outside that subset fall back to the CPU renderer.
#[cfg(feature = "gpu_palette")]
#[derive(Component, Default, Clone, Copy, Debug)]
#[require(PxCompositeSprite)]
pub struct PxGpuComposite;

impl AnimatedAssetComponent for PxSprite {
    type Asset = PxSpriteAsset;

    fn handle(&self) -> &Handle<Self::Asset> {
        self
    }

    fn max_frame_count(sprite: &PxSpriteAsset) -> usize {
        sprite.frame_count()
    }
}

impl Spatial for PxCompositeSprite {
    fn frame_size(&self) -> UVec2 {
        self.size
    }
}

fn sync_composite_metrics(
    composite: &mut PxCompositeSprite,
    sprites: &Assets<PxSpriteAsset>,
    atlases: &Assets<PxSpriteAtlasAsset>,
    mut count: Option<Mut<PxFrameCount>>,
) {
    composite.recompute_metrics_with_atlases(sprites, atlases);
    if let Some(count) = count.as_mut() {
        count.0 = composite.frame_count;
    }
}

fn update_composite_metrics_on_change(
    sprites: Res<Assets<PxSpriteAsset>>,
    atlases: Res<Assets<PxSpriteAtlasAsset>>,
    mut composites: Query<
        (&mut PxCompositeSprite, Option<&mut PxFrameCount>),
        Changed<PxCompositeSprite>,
    >,
) {
    for (mut composite, count) in &mut composites {
        sync_composite_metrics(&mut composite, &sprites, &atlases, count);
    }
}

fn update_composite_metrics_on_assets(
    sprites: Res<Assets<PxSpriteAsset>>,
    atlases: Res<Assets<PxSpriteAtlasAsset>>,
    mut sprite_events: MessageReader<AssetEvent<PxSpriteAsset>>,
    mut atlas_events: MessageReader<AssetEvent<PxSpriteAtlasAsset>>,
    mut composites: Query<(&mut PxCompositeSprite, Option<&mut PxFrameCount>)>,
) {
    if sprite_events.read().next().is_none() && atlas_events.read().next().is_none() {
        return;
    }

    for (mut composite, count) in &mut composites {
        sync_composite_metrics(&mut composite, &sprites, &atlases, count);
    }
}

fn sync_composite_frame_count_on_animation_added(
    sprites: Res<Assets<PxSpriteAsset>>,
    atlases: Res<Assets<PxSpriteAtlasAsset>>,
    mut composites: Query<(&mut PxCompositeSprite, &mut PxFrameCount), Added<PxAnimation>>,
) {
    for (mut composite, count) in &mut composites {
        sync_composite_metrics(&mut composite, &sprites, &atlases, Some(count));
    }
}

pub(crate) type SpriteComponents<L> = (
    &'static PxSprite,
    &'static PxPosition,
    &'static PxAnchor,
    &'static L,
    &'static PxCanvas,
    Option<&'static PxFrame>,
    Option<&'static PxFilter>,
    Option<&'static crate::presentation::PxPresentationTransform>,
);

pub(crate) type CompositeSpriteComponents<L> = (
    &'static PxCompositeSprite,
    &'static PxPosition,
    &'static PxAnchor,
    &'static L,
    &'static PxCanvas,
    Option<&'static PxFrame>,
    Option<&'static PxFilter>,
    Option<&'static crate::presentation::PxPresentationTransform>,
);

#[cfg(feature = "headed")]
fn extract_sprites<L: PxLayer>(
    // TODO Maybe calculate `ViewVisibility`
    sprites: Extract<Query<(SpriteComponents<L>, &InheritedVisibility, RenderEntity)>>,
    mut cmd: Commands,
) {
    for (
        (sprite, &position, &anchor, layer, &canvas, frame, filter, presentation),
        visibility,
        id,
    ) in &sprites
    {
        let mut entity = cmd.entity(id);

        if !visibility.get() {
            // TODO Need to a better way to prevent entities from rendering
            entity.remove::<L>();
            continue;
        }

        entity.insert((sprite.clone(), position, anchor, layer.clone(), canvas));

        if let Some(frame) = frame {
            entity.insert(*frame);
        } else {
            entity.remove::<PxFrame>();
        }

        if let Some(filter) = filter {
            entity.insert(filter.clone());
        } else {
            entity.remove::<PxFilter>();
        }

        if let Some(&presentation) = presentation {
            entity.insert(presentation);
        } else {
            entity.remove::<crate::presentation::PxPresentationTransform>();
        }
    }
}

#[cfg(feature = "headed")]
fn extract_composite_sprites<L: PxLayer>(
    // TODO Maybe calculate `ViewVisibility`
    sprites: Extract<
        Query<(
            CompositeSpriteComponents<L>,
            &InheritedVisibility,
            RenderEntity,
        )>,
    >,
    mut cmd: Commands,
) {
    for (
        (sprite, &position, &anchor, layer, &canvas, frame, filter, presentation),
        visibility,
        id,
    ) in &sprites
    {
        let mut entity = cmd.entity(id);

        if !visibility.get() {
            // TODO Need to a better way to prevent entities from rendering
            entity.remove::<L>();
            continue;
        }

        entity.insert((sprite.clone(), position, anchor, layer.clone(), canvas));

        if let Some(frame) = frame {
            entity.insert(*frame);
        } else {
            entity.remove::<PxFrame>();
        }

        if let Some(filter) = filter {
            entity.insert(filter.clone());
        } else {
            entity.remove::<PxFilter>();
        }

        if let Some(&presentation) = presentation {
            entity.insert(presentation);
        } else {
            entity.remove::<crate::presentation::PxPresentationTransform>();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;
    use crate::{
        atlas::{AtlasRect, PxSpriteAtlasAsset},
        camera::PxCamera,
        frame::{PxFrameSelector, PxFrameView, draw_spatial, resolve_frame_binding},
        image::PxImage,
    };
    use bevy_asset::Assets;
    use bevy_platform::collections::HashMap;
    use insta::assert_snapshot;

    fn pixels(image: &PxImage) -> Vec<u8> {
        let size = image.size();
        let mut out = Vec::with_capacity((size.x * size.y) as usize);
        for y in 0..size.y as i32 {
            for x in 0..size.x as i32 {
                out.push(image.pixel(IVec2::new(x, y)));
            }
        }
        out
    }

    fn image_grid(image: &PxImage) -> String {
        let size = image.size();
        let mut out = String::new();
        for y in 0..size.y as i32 {
            for x in 0..size.x as i32 {
                let value = image.pixel(IVec2::new(x, y));
                if x > 0 {
                    out.push(' ');
                }
                let _ = write!(&mut out, "{value:02}");
            }
            if y + 1 < size.y as i32 {
                out.push('\n');
            }
        }
        out
    }

    fn draw_composite(
        image: &mut PxImage,
        composite: &PxCompositeSprite,
        master: Option<PxFrameView>,
        sprites: &Assets<PxSpriteAsset>,
        atlases: &Assets<PxSpriteAtlasAsset>,
    ) {
        let mut slice = image.slice_all_mut();
        let base_pos = IVec2::ZERO - PxAnchor::BottomLeft.pos(composite.size).as_ivec2();

        for part in &composite.parts {
            let resolved = part
                .source
                .resolve(|handle| sprites.get(handle), |handle| atlases.get(handle))
                .unwrap();
            let part_frame = resolve_frame_binding(
                master,
                composite.frame_count,
                resolved.frame_count(),
                &part.frame,
            );
            let part_pos = base_pos + (part.offset - composite.origin);
            let drawable = PxCompositePartDrawable {
                resolved,
                flip_x: part.flip_x,
                flip_y: part.flip_y,
            };
            draw_spatial(
                &drawable,
                (),
                &mut slice,
                part_pos.into(),
                PxAnchor::BottomLeft,
                PxCanvas::Camera,
                part_frame,
                [],
                PxCamera::default(),
            );
        }
    }

    #[test]
    fn sprite_draws_nonzero_pixels() {
        let sprite = PxSpriteAsset {
            data: PxImage::new(vec![0, 2, 3, 0], 2),
            frame_size: 4,
        };
        let mut image = PxImage::new(vec![1; 4], 2);
        let mut slice = image.slice_all_mut();

        draw_spatial(
            &sprite,
            (),
            &mut slice,
            PxPosition(IVec2::ZERO),
            PxAnchor::BottomLeft,
            PxCanvas::Camera,
            None,
            [],
            PxCamera::default(),
        );

        let expected = vec![1, 2, 3, 1];
        assert_eq!(pixels(&image), expected);
    }

    #[test]
    fn composite_sprite_snapshot() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let sprite_a = sprites.add(PxSpriteAsset {
            data: PxImage::new(vec![1, 2, 3, 4], 2),
            frame_size: 4,
        });
        let sprite_b = sprites.add(PxSpriteAsset {
            data: PxImage::new(vec![5, 6, 7, 8], 2),
            frame_size: 4,
        });

        let mut composite = PxCompositeSprite::new(vec![
            PxCompositePart {
                source: PxCompositePartSource::Sprite(sprite_a),
                offset: IVec2::ZERO,
                frame: PxFrameBinding::default(),
                filter: None,
                flip_x: false,
                flip_y: false,
            },
            PxCompositePart {
                source: PxCompositePartSource::Sprite(sprite_b),
                offset: IVec2::new(2, 0),
                frame: PxFrameBinding::default(),
                filter: None,
                flip_x: false,
                flip_y: false,
            },
        ]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);

        let mut image = PxImage::new(vec![0; 8], 4);
        draw_composite(&mut image, &composite, None, &sprites, &atlases);

        assert_snapshot!(
            image_grid(&image),
            @r###"
01 02 05 06
03 04 07 08
"###
        );
    }

    #[test]
    fn composite_animation_snapshot() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let sprite_a = sprites.add(PxSpriteAsset {
            data: PxImage::new(vec![1, 2, 3, 4, 9, 10, 11, 12], 2),
            frame_size: 4,
        });
        let sprite_b = sprites.add(PxSpriteAsset {
            data: PxImage::new(vec![5, 6, 7, 8, 13, 14, 15, 16], 2),
            frame_size: 4,
        });

        let mut composite = PxCompositeSprite::new(vec![
            PxCompositePart {
                source: PxCompositePartSource::Sprite(sprite_a),
                offset: IVec2::ZERO,
                frame: PxFrameBinding::default(),
                filter: None,
                flip_x: false,
                flip_y: false,
            },
            PxCompositePart {
                source: PxCompositePartSource::Sprite(sprite_b),
                offset: IVec2::new(2, 0),
                frame: PxFrameBinding::Offset(1),
                filter: None,
                flip_x: false,
                flip_y: false,
            },
        ]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);

        let mut image_frame_0 = PxImage::new(vec![0; 8], 4);
        draw_composite(
            &mut image_frame_0,
            &composite,
            Some(PxFrameView::from(PxFrameSelector::Index(0.))),
            &sprites,
            &atlases,
        );

        let mut image_frame_1 = PxImage::new(vec![0; 8], 4);
        draw_composite(
            &mut image_frame_1,
            &composite,
            Some(PxFrameView::from(PxFrameSelector::Index(1.))),
            &sprites,
            &atlases,
        );

        let snapshot = format!(
            "frame 0\n{}\n\nframe 1\n{}",
            image_grid(&image_frame_0),
            image_grid(&image_frame_1)
        );
        assert_snapshot!(
            snapshot,
            @r###"
frame 0
01 02 13 14
03 04 15 16

frame 1
09 10 05 06
11 12 07 08
"###
        );
    }

    fn two_frame_atlas() -> PxSpriteAtlasAsset {
        PxSpriteAtlasAsset {
            size: UVec2::new(4, 2),
            data: PxImage::new(vec![1, 2, 5, 6, 3, 4, 7, 8], 4),
            regions: vec![AtlasRegion {
                frame_size: UVec2::new(2, 2),
                frames: vec![
                    AtlasRect {
                        x: 0,
                        y: 0,
                        w: 2,
                        h: 2,
                    },
                    AtlasRect {
                        x: 2,
                        y: 0,
                        w: 2,
                        h: 2,
                    },
                ],
            }],
            names: HashMap::default(),
        }
    }

    #[test]
    fn composite_metrics_support_mixed_sources() {
        let mut sprites = Assets::default();
        let mut atlases = Assets::default();
        let sprite = sprites.add(PxSpriteAsset {
            data: PxImage::new(vec![1, 2], 2),
            frame_size: 2,
        });
        let atlas = atlases.add(two_frame_atlas());

        let mut composite = PxCompositeSprite::new(vec![
            PxCompositePart {
                source: PxCompositePartSource::Sprite(sprite),
                offset: IVec2::new(-1, 1),
                frame: PxFrameBinding::default(),
                filter: None,
                flip_x: false,
                flip_y: false,
            },
            PxCompositePart {
                source: PxCompositePartSource::AtlasRegion {
                    atlas,
                    region: AtlasRegionId(0),
                },
                offset: IVec2::new(1, -1),
                frame: PxFrameBinding::default(),
                filter: None,
                flip_x: false,
                flip_y: false,
            },
        ]);

        composite.recompute_metrics_with_atlases(&sprites, &atlases);

        assert_eq!(composite.origin, IVec2::new(-1, -1));
        assert_eq!(composite.size, UVec2::new(4, 3));
        assert_eq!(composite.frame_count, 2);
    }

    #[test]
    fn composite_atlas_part_uses_frame_binding() {
        let sprites = Assets::default();
        let mut atlases = Assets::default();
        let atlas = atlases.add(two_frame_atlas());

        let mut composite = PxCompositeSprite::new(vec![PxCompositePart {
            source: PxCompositePartSource::AtlasRegion {
                atlas,
                region: AtlasRegionId(0),
            },
            offset: IVec2::ZERO,
            frame: PxFrameBinding::Offset(1),
            filter: None,
            flip_x: false,
            flip_y: false,
        }]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);

        let mut image = PxImage::new(vec![0; 4], 2);
        draw_composite(
            &mut image,
            &composite,
            Some(PxFrameView::from(PxFrameSelector::Index(0.))),
            &sprites,
            &atlases,
        );

        assert_eq!(pixels(&image), vec![5, 6, 7, 8]);
    }

    #[test]
    fn composite_part_flip_semantics() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let sprite = sprites.add(PxSpriteAsset {
            data: PxImage::new(vec![1, 2, 3, 4], 2),
            frame_size: 4,
        });

        let mut composite = PxCompositeSprite::new(vec![
            PxCompositePart {
                source: PxCompositePartSource::Sprite(sprite.clone()),
                offset: IVec2::ZERO,
                frame: PxFrameBinding::default(),
                filter: None,
                flip_x: true,
                flip_y: false,
            },
            PxCompositePart {
                source: PxCompositePartSource::Sprite(sprite),
                offset: IVec2::new(2, 0),
                frame: PxFrameBinding::default(),
                filter: None,
                flip_x: false,
                flip_y: true,
            },
        ]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);

        let mut image = PxImage::new(vec![0; 8], 4);
        draw_composite(&mut image, &composite, None, &sprites, &atlases);

        assert_eq!(pixels(&image), vec![2, 1, 3, 4, 4, 3, 1, 2]);
    }

    #[test]
    fn composite_part_source_reports_missing_atlas_asset() {
        let result = PxCompositePartSource::atlas_region(Handle::default(), AtlasRegionId(3))
            .resolve(
                |_: &Handle<PxSpriteAsset>| None,
                |_: &Handle<PxSpriteAtlasAsset>| None,
            );

        assert!(matches!(
            result,
            Err(PxCompositePartResolveError::MissingAtlasAsset(_))
        ));
    }

    #[test]
    fn composite_part_source_reports_missing_atlas_region() {
        let atlas = two_frame_atlas();
        let result = PxCompositePartSource::atlas_region(Handle::default(), AtlasRegionId(3))
            .resolve(
                |_: &Handle<PxSpriteAsset>| None,
                |_: &Handle<PxSpriteAtlasAsset>| Some(&atlas),
            );

        assert!(matches!(
            result,
            Err(PxCompositePartResolveError::MissingAtlasRegion {
                region: AtlasRegionId(3),
                ..
            })
        ));
    }
}

// /// Size of threshold map to use for dithering. The image is tiled with dithering according to this
// /// map, so smaller sizes will have more visible repetition and worse color approximation, but
// /// larger sizes are much, much slower with pattern dithering.
// #[derive(Clone, Copy, Debug)]
// pub enum ThresholdMap {
//     /// 2x2
//     X2_2,
//     /// 4x4
//     X4_4,
//     /// 8x8
//     X8_8,
// }
//
// /// Dithering algorithm. Perf measurements are for 10,000 pixels with a 4x4 threshold map on a
// /// pretty old machine.
// #[derive(Clone, Copy, Debug)]
// pub enum DitherAlgorithm {
//     /// Almost as fast as undithered. 16.0 ms in debug mode and 1.23 ms in release mode. Doesn't
//     /// make very good use of the color palette.
//     Ordered,
//     /// Slow, but mixes colors very well. 219 ms in debug mode and 6.81 ms in release mode. Consider
//     /// only using this algorithm with some optimizations enabled.
//     Pattern,
// }
//
// /// Info needed to dither an image
// #[derive(Clone, Debug)]
// pub struct Dither {
//     /// Dithering algorithm
//     pub algorithm: DitherAlgorithm,
//     /// How much to dither. Lower values leave solid color areas. Should range from 0 to 1.
//     pub threshold: f32,
//     /// Threshold map size
//     pub threshold_map: ThresholdMap,
// }

// // TODO Example
// /// Renders the contents of an image to a sprite every tick. The image is interpreted as
// /// `Rgba8UnormSrgb`.
// #[derive(Component, Clone, Default, Debug)]
// pub struct ImageToSprite {
//     /// Image to render
//     pub image: Handle<Image>,
//     /// Dithering
//     pub dither: Option<Dither>,
// }

// /// Spawns a sprite generated from an [`Image`]
// #[derive(Bundle, Debug, Default)]
// pub struct ImageToSpriteBundle<L: PxLayer> {
//     /// A [`Handle<PxSprite>`] component
//     pub image: ImageToSprite,
//     /// A [`PxPosition`] component
//     pub position: PxPosition,
//     /// A [`PxAnchor`] component
//     pub anchor: PxAnchor,
//     /// A layer component
//     pub layer: L,
//     /// A [`PxCanvas`] component
//     pub canvas: PxCanvas,
//     /// A [`Visibility`] component
//     pub visibility: Visibility,
//     /// An [`InheritedVisibility`] component
//     pub inherited_visibility: InheritedVisibility,
// }

// pub(crate) trait MapSize<const SIZE: usize> {
//     const WIDTH: usize;
//     const MAP: [usize; SIZE];
// }
//
// impl MapSize<1> for () {
//     const WIDTH: usize = 1;
//     const MAP: [usize; 1] = [0];
// }
//
// impl MapSize<4> for () {
//     const WIDTH: usize = 2;
//     #[rustfmt::skip]
//     const MAP: [usize; 4] = [
//         0, 2,
//         3, 1,
//     ];
// }
//
// impl MapSize<16> for () {
//     const WIDTH: usize = 4;
//     #[rustfmt::skip]
//     const MAP: [usize; 16] = [
//         0, 8, 2, 10,
//         12, 4, 14, 6,
//         3, 11, 1, 9,
//         15, 7, 13, 5,
//     ];
// }
//
// impl MapSize<64> for () {
//     const WIDTH: usize = 8;
//     #[rustfmt::skip]
//     const MAP: [usize; 64] = [
//         0, 48, 12, 60, 3, 51, 15, 63,
//         32, 16, 44, 28, 35, 19, 47, 31,
//         8, 56, 4, 52, 11, 59, 7, 55,
//         40, 24, 36, 20, 43, 27, 39, 23,
//         2, 50, 14, 62, 1, 49, 13, 61,
//         34, 18, 46, 30, 33, 17, 45, 29,
//         10, 58, 6, 54, 9, 57, 5, 53,
//         42, 26, 38, 22, 41, 25, 37, 21,
//     ];
// }
//
// pub(crate) trait Algorithm<const MAP_SIZE: usize> {
//     fn compute(
//         color: Vec3,
//         threshold: Vec3,
//         threshold_index: usize,
//         candidates: &mut [usize; MAP_SIZE],
//         palette_tree: &ImmutableKdTree<f32, 3>,
//         palette: &[Vec3],
//     ) -> u8;
// }
//
// pub(crate) enum ClosestAlg {}
//
// impl<const MAP_SIZE: usize> Algorithm<MAP_SIZE> for ClosestAlg {
//     fn compute(
//         color: Vec3,
//         _: Vec3,
//         _: usize,
//         _: &mut [usize; MAP_SIZE],
//         palette_tree: &ImmutableKdTree<f32, 3>,
//         _: &[Vec3],
//     ) -> u8 {
//         palette_tree
//             .approx_nearest_one::<SquaredEuclidean>(&color.into())
//             .item as usize as u8
//     }
// }
//
// pub(crate) enum OrderedAlg {}
//
// impl<const MAP_SIZE: usize> Algorithm<MAP_SIZE> for OrderedAlg {
//     fn compute(
//         color: Vec3,
//         threshold: Vec3,
//         threshold_index: usize,
//         _: &mut [usize; MAP_SIZE],
//         palette_tree: &ImmutableKdTree<f32, 3>,
//         _: &[Vec3],
//     ) -> u8 {
//         palette_tree
//             .approx_nearest_one::<SquaredEuclidean>(
//                 &(color + threshold * (threshold_index as f32 / MAP_SIZE as f32 - 0.5)).into(),
//             )
//             .item as u8
//     }
// }
//
// pub(crate) enum PatternAlg {}
//
// impl<const MAP_SIZE: usize> Algorithm<MAP_SIZE> for PatternAlg {
//     fn compute(
//         color: Vec3,
//         threshold: Vec3,
//         threshold_index: usize,
//         candidates: &mut [usize; MAP_SIZE],
//         palette_tree: &ImmutableKdTree<f32, 3>,
//         palette: &[Vec3],
//     ) -> u8 {
//         let mut error = Vec3::ZERO;
//         for candidate_ref in &mut *candidates {
//             let sample = color + error * threshold;
//             let candidate = palette_tree
//                 .approx_nearest_one::<SquaredEuclidean>(&sample.into())
//                 .item as usize;
//
//             *candidate_ref = candidate;
//             error += color - palette[candidate];
//         }
//
//         candidates.sort_unstable_by(|&candidate_1, &candidate_2| {
//             palette[candidate_1][0].total_cmp(&palette[candidate_2][0])
//         });
//
//         candidates[threshold_index] as u8
//     }
// }
//
// pub(crate) fn dither_slice<A: Algorithm<MAP_SIZE>, const MAP_SIZE: usize>(
//     pixels: &mut [(usize, (&[u8], &mut Option<u8>))],
//     threshold: f32,
//     size: UVec2,
//     palette_tree: &ImmutableKdTree<f32, 3>,
//     palette: &[Vec3],
// ) where
//     (): MapSize<MAP_SIZE>,
// {
//     let mut candidates = [0; MAP_SIZE];
//
//     for &mut (i, (color, ref mut pixel)) in pixels {
//         let i = i as u32;
//         let pos = UVec2::new(i % size.x, i / size.x);
//
//         if color[3] == 0 {
//             **pixel = None;
//             continue;
//         }
//
//         **pixel = Some(A::compute(
//             Oklaba::from(Srgba::rgb_u8(color[0], color[1], color[2])).to_vec3(),
//             Vec3::splat(threshold),
//             <() as MapSize<MAP_SIZE>>::MAP[pos.x as usize % <() as MapSize<MAP_SIZE>>::WIDTH
//                 * <() as MapSize<MAP_SIZE>>::WIDTH
//                 + pos.y as usize % <() as MapSize<MAP_SIZE>>::WIDTH],
//             &mut candidates,
//             palette_tree,
//             palette,
//         ));
//     }
// }

// pub(crate) type ImageToSpriteComponents<L> = (
//     &'static ImageToSprite,
//     &'static PxPosition,
//     &'static PxAnchor,
//     &'static L,
//     &'static PxCanvas,
//     Option<&'static Handle<PxFilter>>,
// );
//
// fn extract_image_to_sprites<L: PxLayer>(
//     image_to_sprites: Extract<Query<(ImageToSpriteComponents<L>, &InheritedVisibility)>>,
//     mut cmd: Commands,
// ) {
//     for ((image_to_sprite, &position, &anchor, layer, &canvas, filter), visibility) in
//         &image_to_sprites
//     {
//         if !visibility.get() {
//             continue;
//         }
//
//         let mut image_to_sprite = cmd.spawn((
//             image_to_sprite.clone(),
//             position,
//             anchor,
//             layer.clone(),
//             canvas,
//         ));
//
//         if let Some(filter) = filter {
//             image_to_sprite.insert(filter.clone());
//         }
//     }
// }
