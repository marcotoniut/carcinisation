//! Sprites

use std::{error::Error, path::PathBuf, time::Instant};

use bevy_asset::{AssetEvent, AssetId, AssetLoader, LoadContext, io::Reader};
use bevy_derive::{Deref, DerefMut};
use bevy_diagnostic::{
    DEFAULT_MAX_HISTORY_LENGTH, Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic,
};
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
    atlas::{AtlasRegion, AtlasRegionId, CxSpriteAtlasAsset},
    filter::CxFilterAsset,
    frame::{CxFrameBinding, CxFrameCount, Frames},
    image::{CxImage, CxImageSliceMut},
    palette::Palette,
    position::{CxLayer, DefaultLayer, Spatial},
    prelude::*,
    set::CxSet,
};

const COMPOSITE_METRICS_ON_CHANGE_COUNT: DiagnosticPath =
    DiagnosticPath::const_new("carapace/composite_metrics_on_change_count");
const COMPOSITE_METRICS_ON_CHANGE_PARTS: DiagnosticPath =
    DiagnosticPath::const_new("carapace/composite_metrics_on_change_parts");
const COMPOSITE_METRICS_ON_CHANGE_TIME: DiagnosticPath =
    DiagnosticPath::const_new("carapace/composite_metrics_on_change_time");

pub(crate) fn plug_core(app: &mut App, palette_path: PathBuf) {
    app.init_asset::<CxSpriteAsset>()
        .register_asset_loader(CxSpriteLoader::new(palette_path))
        .register_diagnostic(
            Diagnostic::new(COMPOSITE_METRICS_ON_CHANGE_COUNT)
                .with_suffix(" composites")
                .with_max_history_length(DEFAULT_MAX_HISTORY_LENGTH),
        )
        .register_diagnostic(
            Diagnostic::new(COMPOSITE_METRICS_ON_CHANGE_PARTS)
                .with_suffix(" parts")
                .with_max_history_length(DEFAULT_MAX_HISTORY_LENGTH),
        )
        .register_diagnostic(
            Diagnostic::new(COMPOSITE_METRICS_ON_CHANGE_TIME)
                .with_suffix(" ms")
                .with_max_history_length(DEFAULT_MAX_HISTORY_LENGTH),
        );

    app.add_systems(
        PostUpdate,
        (
            update_composite_metrics_on_change,
            update_composite_metrics_on_assets,
            sync_composite_frame_count_on_animation_added,
        )
            .after(CxSet::CompositePresentationWrites)
            .before(CxSet::FinishAnimations),
    );
}

pub(crate) fn plug<L: CxLayer>(app: &mut App, palette_path: PathBuf) {
    #[cfg(feature = "headed")]
    app.add_plugins((
        RenderAssetPlugin::<CxSpriteAsset>::default(),
        SyncComponentPlugin::<CxSprite>::default(),
        SyncComponentPlugin::<CxCompositeSprite>::default(),
    ));

    #[cfg(all(feature = "headed", feature = "gpu_palette"))]
    app.add_plugins((
        RenderAssetPlugin::<CxSpriteGpu>::default(),
        SyncComponentPlugin::<CxGpuSprite>::default(),
        SyncComponentPlugin::<CxGpuComposite>::default(),
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
struct CxSpriteLoaderSettings {
    frame_count: usize,
    image_loader_settings: ImageLoaderSettings,
}

impl Default for CxSpriteLoaderSettings {
    fn default() -> Self {
        Self {
            frame_count: 1,
            image_loader_settings: default(),
        }
    }
}

#[derive(TypePath)]
struct CxSpriteLoader {
    palette_path: PathBuf,
}

impl CxSpriteLoader {
    fn new(palette_path: PathBuf) -> Self {
        Self { palette_path }
    }
}

impl AssetLoader for CxSpriteLoader {
    type Asset = CxSpriteAsset;
    type Settings = CxSpriteLoaderSettings;
    type Error = Box<dyn Error + Send + Sync>;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &CxSpriteLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<CxSpriteAsset, Self::Error> {
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
            CxImage::palette_indices(palette.get(), &image).map_err(|err| err.to_string())?;

        Ok(CxSpriteAsset {
            frame_size: data.area() / settings.frame_count,
            data,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["px_sprite.png"]
    }
}

/// A sprite. Use your asset wrapper to create a [`Handle<CxSpriteAsset>`] and supply an image.
/// If the sprite is animated, the frames should be laid out from top to bottom.
/// See `assets/sprite/runner.png` for an example of an animated sprite.
#[derive(Asset, Serialize, Deserialize, Clone, Reflect, Debug)]
pub struct CxSpriteAsset {
    pub(crate) data: CxImage,
    pub(crate) frame_size: usize,
}

#[cfg(feature = "gpu_palette")]
#[derive(Clone)]
pub(crate) struct CxSpriteGpu {
    pub(crate) size: UVec2,
    pub(crate) frame_size: usize,
    pub(crate) texture: Texture,
}

#[cfg(feature = "gpu_palette")]
impl RenderAsset for CxSpriteGpu {
    type SourceAsset = CxSpriteAsset;
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
impl RenderAsset for CxSpriteAsset {
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

impl Frames for CxSpriteAsset {
    type Param = ();

    fn frame_count(&self) -> usize {
        self.data.area() / self.frame_size
    }

    fn draw(
        &self,
        (): (),
        image: &mut CxImageSliceMut,
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

impl Spatial for CxSpriteAsset {
    fn frame_size(&self) -> UVec2 {
        UVec2::new(
            self.data.width() as u32,
            (self.frame_size / self.data.width()) as u32,
        )
    }
}

impl Frames for CxResolvedCompositePart<'_> {
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
        image: &mut CxImageSliceMut,
        frame: impl Fn(UVec2) -> usize,
        filter: impl Fn(u8) -> u8,
    ) {
        match self {
            Self::Sprite(sprite) => sprite.draw((), image, frame, filter),
            Self::AtlasRegion { atlas, region } => (*atlas, *region).draw((), image, frame, filter),
        }
    }
}

impl Spatial for CxResolvedCompositePart<'_> {
    fn frame_size(&self) -> UVec2 {
        match self {
            Self::Sprite(sprite) => sprite.frame_size(),
            Self::AtlasRegion { region, .. } => region.frame_size,
        }
    }
}

/// A sprite
#[derive(Component, Deref, DerefMut, Default, Clone, Debug, Reflect)]
#[require(CxPosition, CxAnchor, DefaultLayer, CxRenderSpace)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct CxSprite(pub Handle<CxSpriteAsset>);

impl From<Handle<CxSpriteAsset>> for CxSprite {
    fn from(value: Handle<CxSpriteAsset>) -> Self {
        Self(value)
    }
}

/// A sprite composed of multiple sprite or atlas-backed parts.
///
/// The CPU renderer supports all composite part sources and per-part flips. The optional
/// [`CxGpuComposite`] path is a narrower optimization subset: it currently supports only
/// sprite-backed parts with no per-part filter and no flips.
#[derive(Component, Default, Clone, Debug)]
#[require(CxPosition, CxAnchor, DefaultLayer, CxRenderSpace)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct CxCompositeSprite {
    /// Parts that make up the composite sprite.
    pub parts: Vec<CxCompositePart>,
    /// Cached base placement size (from native part bounds).
    pub size: UVec2,
    /// Cached base placement origin (min corner of native part bounds).
    pub origin: IVec2,
    /// Cached render envelope size (may exceed `size` for transformed parts).
    pub render_size: UVec2,
    /// Cached render envelope origin.
    pub render_origin: IVec2,
    /// Cached frame count for the master animation.
    pub frame_count: usize,
}

/// Opt-in marker for composites whose writer keeps cached metrics in sync.
///
/// When present, change-driven metric sync trusts the cached `origin`, `size`,
/// `render_origin`, `render_size`, and `frame_count` already stored on
/// [`CxCompositeSprite`] instead of rescanning `parts`. Asset-driven sync still
/// recomputes metrics so hot-reloaded sprite or atlas size changes remain
/// correct.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct CxAuthoritativeCompositeMetrics;

#[derive(Clone, Copy, Debug)]
pub(crate) struct CxCompositeMetrics {
    /// Base placement size (from untransformed / native part bounds).
    /// Used for anchor computation and entity-level placement.
    pub size: UVec2,
    /// Base placement origin (min corner of native part bounds).
    pub origin: IVec2,
    /// Render envelope size (may be larger than `size` due to worst-case
    /// envelopes for transformed parts). Used for scratch buffer allocation.
    pub render_size: UVec2,
    /// Render envelope origin (min corner of worst-case bounds).
    /// Parts are placed in the scratch at `part.offset - render_origin`.
    pub render_origin: IVec2,
    pub frame_count: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CxCompositePartMetrics {
    pub size: UVec2,
    pub frame_count: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum CxResolvedCompositePart<'a> {
    Sprite(&'a CxSpriteAsset),
    AtlasRegion {
        atlas: &'a CxSpriteAtlasAsset,
        region: &'a AtlasRegion,
    },
}

#[derive(Clone, Debug)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum CxCompositePartResolveError {
    MissingSpriteAsset(Handle<CxSpriteAsset>),
    MissingAtlasAsset(Handle<CxSpriteAtlasAsset>),
    MissingAtlasRegion {
        atlas: Handle<CxSpriteAtlasAsset>,
        region: AtlasRegionId,
    },
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CxCompositePartDrawable<'a> {
    pub resolved: CxResolvedCompositePart<'a>,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl CxResolvedCompositePart<'_> {
    pub(crate) fn metrics(&self) -> CxCompositePartMetrics {
        CxCompositePartMetrics {
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

impl Frames for CxCompositePartDrawable<'_> {
    type Param = ();

    fn frame_count(&self) -> usize {
        self.resolved.frame_count()
    }

    fn draw(
        &self,
        (): (),
        image: &mut CxImageSliceMut,
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

impl Spatial for CxCompositePartDrawable<'_> {
    fn frame_size(&self) -> UVec2 {
        self.resolved.frame_size()
    }
}

impl CxCompositeSprite {
    /// Create a composite sprite from parts.
    #[must_use]
    pub fn new(parts: Vec<CxCompositePart>) -> Self {
        Self {
            parts,
            size: UVec2::ZERO,
            origin: IVec2::ZERO,
            render_size: UVec2::ZERO,
            render_origin: IVec2::ZERO,
            frame_count: 0,
        }
    }

    /// Set cached metrics for a composite whose parts use only native bounds.
    ///
    /// This is correct when no part transform expands the render envelope,
    /// which means the render bounds match the base bounds exactly.
    pub fn set_native_metrics(&mut self, origin: IVec2, size: UVec2, frame_count: usize) {
        self.origin = origin;
        self.size = size;
        self.render_origin = origin;
        self.render_size = size;
        self.frame_count = frame_count;
    }

    /// Recompute cached metrics from current parts.
    ///
    /// Computes two sets of bounds:
    /// - **Base bounds** (`origin`, `size`): from native (untransformed) part extents.
    ///   Used for anchor computation and entity-level placement.
    /// - **Render bounds** (`render_origin`, `render_size`): union of base bounds and
    ///   worst-case envelopes for transformed parts. Used for scratch buffer allocation.
    ///
    /// This separation ensures that expanding the render envelope for animated parts
    /// does not shift the composite's anchor or world placement.
    pub(crate) fn metrics_with<F>(&self, mut get: F) -> Option<CxCompositeMetrics>
    where
        F: FnMut(&CxCompositePartSource) -> Option<CxCompositePartMetrics>,
    {
        let mut any = false;
        // Base bounds: native part extents only.
        let mut base_min = IVec2::ZERO;
        let mut base_max = IVec2::ZERO;
        // Render bounds: union of base bounds + worst-case envelopes.
        let mut render_min = IVec2::ZERO;
        let mut render_max = IVec2::ZERO;
        let mut frame_count = 0usize;

        for part in &self.parts {
            let Some(metrics) = get(&part.source) else {
                continue;
            };

            // Native / base bounds for this part (always computed).
            let native_size = metrics.size.as_ivec2();
            let native_min = part.offset;
            let native_max = part.offset + native_size;

            // Render envelope: use worst-case bounds for transformed parts,
            // native bounds otherwise. worst_case_bounds is rotation-independent
            // so the result is stable during runtime articulation.
            let (env_min, env_max) = match &part.transform {
                Some(t) if !t.is_identity() => t.worst_case_bounds(part.offset, metrics.size),
                _ => (native_min, native_max),
            };

            if any {
                base_min = base_min.min(native_min);
                base_max = base_max.max(native_max);
                render_min = render_min.min(env_min);
                render_max = render_max.max(env_max);
            } else {
                base_min = native_min;
                base_max = native_max;
                render_min = env_min;
                render_max = env_max;
                any = true;
            }

            frame_count = frame_count.max(metrics.frame_count);
        }

        if !any {
            return None;
        }

        // Render bounds must be at least as large as base bounds.
        render_min = render_min.min(base_min);
        render_max = render_max.max(base_max);

        let base_size = base_max - base_min;
        let render_size = render_max - render_min;
        Some(CxCompositeMetrics {
            origin: base_min,
            size: UVec2::new(base_size.x.max(0) as u32, base_size.y.max(0) as u32),
            render_origin: render_min,
            render_size: UVec2::new(render_size.x.max(0) as u32, render_size.y.max(0) as u32),
            frame_count,
        })
    }

    /// Recompute cached size/origin/frame count from current parts using sprite assets only.
    ///
    /// Atlas-backed parts are ignored by this compatibility helper. Use
    /// [`CxCompositeSprite::recompute_metrics_with_atlases`] when a composite may contain atlas
    /// regions.
    pub fn recompute_metrics(&mut self, sprites: &Assets<CxSpriteAsset>) {
        if self
            .parts
            .iter()
            .any(|part| matches!(part.source, CxCompositePartSource::AtlasRegion { .. }))
        {
            warn!(
                "CxCompositeSprite::recompute_metrics() ignores atlas-backed parts; \
                 use recompute_metrics_with_atlases() for atlas-backed composites"
            );
        }
        let atlases = Assets::<CxSpriteAtlasAsset>::default();
        self.recompute_metrics_with_atlases(sprites, &atlases);
    }

    /// Recompute cached size/origin/frame count from current parts using sprite and atlas assets.
    pub fn recompute_metrics_with_atlases(
        &mut self,
        sprites: &Assets<CxSpriteAsset>,
        atlases: &Assets<CxSpriteAtlasAsset>,
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
            self.render_size = metrics.render_size;
            self.render_origin = metrics.render_origin;
            self.frame_count = metrics.frame_count;
        } else {
            self.size = UVec2::ZERO;
            self.origin = IVec2::ZERO;
            self.render_size = UVec2::ZERO;
            self.render_origin = IVec2::ZERO;
            self.frame_count = 0;
        }
    }
}

/// Source for a composite part.
///
/// Composite parts stay source-agnostic at the engine layer: a part can draw either a standalone
/// [`CxSpriteAsset`] or a region within a [`CxSpriteAtlasAsset`], referenced by atlas handle and
/// [`AtlasRegionId`].
///
/// For most call sites, prefer [`CxCompositePart::new`] or [`CxCompositePart::atlas_region`] and
/// then configure the part with builder-style helpers. Use this enum directly when you need to
/// construct or store the source separately from the part.
#[derive(Clone, Debug)]
pub enum CxCompositePartSource {
    /// Draw from a standalone sprite asset.
    Sprite(Handle<CxSpriteAsset>),
    /// Draw from a named region within a sprite atlas asset.
    AtlasRegion {
        /// Atlas asset handle.
        atlas: Handle<CxSpriteAtlasAsset>,
        /// Region identifier within the atlas.
        region: AtlasRegionId,
    },
}

impl CxCompositePartSource {
    /// Create a composite part source from a standalone sprite.
    #[must_use]
    pub fn sprite(sprite: Handle<CxSpriteAsset>) -> Self {
        Self::Sprite(sprite)
    }

    /// Create a composite part source from an atlas region.
    #[must_use]
    pub fn atlas_region(atlas: Handle<CxSpriteAtlasAsset>, region: AtlasRegionId) -> Self {
        Self::AtlasRegion { atlas, region }
    }

    pub(crate) fn resolve<'a, FS, FA>(
        &self,
        mut get_sprite: FS,
        mut get_atlas: FA,
    ) -> Result<CxResolvedCompositePart<'a>, CxCompositePartResolveError>
    where
        FS: FnMut(&Handle<CxSpriteAsset>) -> Option<&'a CxSpriteAsset>,
        FA: FnMut(&Handle<CxSpriteAtlasAsset>) -> Option<&'a CxSpriteAtlasAsset>,
    {
        match self {
            Self::Sprite(sprite) => get_sprite(sprite)
                .map(CxResolvedCompositePart::Sprite)
                .ok_or_else(|| CxCompositePartResolveError::MissingSpriteAsset(sprite.clone())),
            Self::AtlasRegion { atlas, region } => {
                let atlas_asset = get_atlas(atlas)
                    .ok_or_else(|| CxCompositePartResolveError::MissingAtlasAsset(atlas.clone()))?;
                let region_asset = atlas_asset.region(*region).ok_or_else(|| {
                    CxCompositePartResolveError::MissingAtlasRegion {
                        atlas: atlas.clone(),
                        region: *region,
                    }
                })?;
                Ok(CxResolvedCompositePart::AtlasRegion {
                    atlas: atlas_asset,
                    region: region_asset,
                })
            }
        }
    }
}

/// Per-part render-only transform applied during composite composition.
///
/// This is distinct from entity-level [`CxPresentationTransform`](crate::presentation::CxPresentationTransform):
/// - `PartTransform` transforms an individual part **during** composition into the
///   composite scratch buffer (articulation / procedural motion inside the composite).
/// - `CxPresentationTransform` transforms the **composed result** when blitting to the
///   final layer (whole-entity visual effects).
///
/// There is **no transform inheritance** between parts — each part's transform is
/// self-contained and pivots around a point within its own bounds.
///
/// # Pivot
///
/// `pivot` is in normalised part-local coordinates with **top-left origin**
/// (Y-down), matching image/raster convention where row 0 is the top:
/// - `(0.0, 0.0)` = top-left of the part
/// - `(0.5, 0.5)` = centre (default)
/// - `(1.0, 1.0)` = bottom-right
///
/// The pivot controls the origin for scale and rotation.
///
/// This differs from [`CxAnchor`], which uses **bottom-left origin** (Y-up)
/// for world-space positioning.  See the
/// [crate-level docs](crate#anchor-origin-convention) for the rationale.
/// An internal `anchor()` method converts between the two.
///
/// # Scale
///
/// Negative scale values produce mirroring, matching the entity-level
/// signed-scale semantics. Magnitude is clamped to `[MIN_SCALE, ∞)`.
///
/// # Multi-strip parts
///
/// Per-part transforms work best on **visually independent** parts (wings, limbs,
/// antennae). Multi-strip regions that were split across several atlas regions
/// (e.g., a head composed of left-half, centre-strip, right-half) cannot be
/// transformed as a unit — each strip scales/rotates around its own pivot,
/// creating gaps or overlaps.
///
/// For regions that must transform together, merge them into a **single atlas
/// region** at the asset level. A future group-transform feature (sub-scratch
/// per group of parts) may lift this limitation if content pressure demands it.
///
/// # Stable envelope
///
/// Composite metrics (`size`, `origin`) use a **worst-case full-rotation envelope**
/// for transformed parts, not current-frame bounds. This means:
/// - Runtime rotation changes do **not** shift the composite anchor or origin.
/// - The scratch buffer may reserve more space than a single frame strictly needs.
/// - This is intentional: stable anchoring is preferred over tight dynamic bounds.
///
/// The conservative envelope is a good fit at Carapace's pixel-art scale. Tighter
/// range-based envelopes (e.g., declared max rotation) can be added later if
/// the over-allocation ever matters.
///
/// # Invariant
///
/// `part.offset` defines **where** the part is placed in composite space.
/// `part.transform` defines **how** the part looks around its local pivot.
/// The transform does not implicitly orbit or remap the part's placement.
#[derive(Clone, Copy, Debug)]
pub struct PartTransform {
    /// Non-uniform scale factor. `Vec2::ONE` = native size. Negative = flip.
    pub scale: Vec2,
    /// Rotation in radians, counter-clockwise around the pivot.
    pub rotation: f32,
    /// Normalised pivot within part bounds (top-left origin).
    /// Default: `(0.5, 0.5)` = centre.
    pub pivot: Vec2,
}

impl Default for PartTransform {
    fn default() -> Self {
        Self {
            scale: Vec2::ONE,
            rotation: 0.0,
            pivot: Vec2::splat(0.5),
        }
    }
}

impl PartTransform {
    /// Returns true if this transform would have no visual effect.
    #[must_use]
    pub fn is_identity(&self) -> bool {
        !self.has_scale() && !self.has_rotation()
    }

    /// Returns true if scale differs meaningfully from `Vec2::ONE`.
    #[must_use]
    pub fn has_scale(&self) -> bool {
        (self.scale - Vec2::ONE).length_squared() >= f32::EPSILON
    }

    /// Returns true if rotation differs meaningfully from zero.
    #[must_use]
    pub fn has_rotation(&self) -> bool {
        self.rotation.abs() >= f32::EPSILON
    }

    /// Returns the scale with magnitude clamped per axis, preserving sign.
    #[must_use]
    pub(crate) fn clamped_scale(&self) -> Vec2 {
        Vec2::new(
            crate::presentation::clamp_scale_axis(self.scale.x),
            crate::presentation::clamp_scale_axis(self.scale.y),
        )
    }

    /// Returns the rotation, with NaN treated as 0.0.
    #[must_use]
    pub(crate) fn sanitised_rotation(&self) -> f32 {
        crate::presentation::sanitise_rotation(self.rotation)
    }

    /// Converts the top-left-origin pivot to a [`CxAnchor::Custom`] (bottom-left origin).
    #[must_use]
    pub(crate) fn anchor(&self) -> CxAnchor {
        CxAnchor::Custom(Vec2::new(self.pivot.x, 1.0 - self.pivot.y))
    }

    /// Computes the bounding box of this part for the **current** transform values.
    ///
    /// Returns `(min, max)` corners in composite engine-space.
    ///
    /// **Not suitable for composite metrics** — use [`worst_case_bounds`](Self::worst_case_bounds)
    /// instead, which is rotation-independent and produces stable bounds for animated parts.
    /// This method is retained for debug/editor visualisation of the current-frame footprint.
    #[allow(dead_code)]
    pub(crate) fn transformed_bounds(
        &self,
        part_offset: IVec2,
        part_size: UVec2,
    ) -> (IVec2, IVec2) {
        let w = part_size.x as f32;
        let h = part_size.y as f32;
        let scale = self.clamped_scale();
        let rotation = self.sanitised_rotation();

        // Pivot in top-left image space.
        let pivot_img = Vec2::new(self.pivot.x * w, self.pivot.y * h);

        let (sin_r, cos_r) = rotation.sin_cos();

        // Source corners relative to pivot (image space, top-left origin).
        let corners = [
            Vec2::new(0.0, 0.0) - pivot_img,
            Vec2::new(w, 0.0) - pivot_img,
            Vec2::new(w, h) - pivot_img,
            Vec2::new(0.0, h) - pivot_img,
        ];

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        for corner in &corners {
            let scaled = Vec2::new(corner.x * scale.x, corner.y * scale.y);
            let rotated = Vec2::new(
                scaled.x * cos_r - scaled.y * sin_r,
                scaled.x * sin_r + scaled.y * cos_r,
            );
            min_x = min_x.min(rotated.x);
            max_x = max_x.max(rotated.x);
            min_y = min_y.min(rotated.y);
            max_y = max_y.max(rotated.y);
        }

        // Pivot position in composite engine-space (bottom-left origin).
        // part_offset is the bottom-left of the part in engine space.
        let pivot_engine =
            part_offset.as_vec2() + Vec2::new(self.pivot.x * w, (1.0 - self.pivot.y) * h);

        // Bounding box in engine space: pivot ± extents.
        // Image-space Y is inverted relative to engine-space, so min_y (top in image)
        // maps to max in engine, and vice versa.
        let eng_min = Vec2::new(pivot_engine.x + min_x, pivot_engine.y - max_y);
        let eng_max = Vec2::new(pivot_engine.x + max_x, pivot_engine.y - min_y);

        (
            IVec2::new(eng_min.x.floor() as i32, eng_min.y.floor() as i32),
            IVec2::new(eng_max.x.ceil() as i32, eng_max.y.ceil() as i32),
        )
    }

    /// Computes a **stable worst-case bounding box** for this part over all
    /// possible rotation angles, in composite engine-space.
    ///
    /// Returns `(min, max)` corners. Used by composite metrics to size the
    /// scratch buffer so that the composite `origin` and `size` remain stable
    /// regardless of the current runtime rotation value.
    ///
    /// The envelope is conservative: for each corner of the part (relative to
    /// the pivot, after scaling), the maximum distance from the pivot is used
    /// as a uniform half-extent. This means the envelope may reserve more space
    /// than a single frame strictly needs — but it guarantees that the composite
    /// anchor never drifts during animation.
    ///
    /// At Carapace's pixel-art scale this over-allocation is negligible (a few KB
    /// of transient scratch per composite). Tighter range-based envelopes can be
    /// considered later if needed.
    ///
    /// # Stability
    ///
    /// The result depends only on `(part_offset, part_size, scale magnitude, pivot)`.
    /// It is intentionally **independent of the current rotation angle**, so
    /// calling this with different `self.rotation` values yields the same result.
    pub(crate) fn worst_case_bounds(&self, part_offset: IVec2, part_size: UVec2) -> (IVec2, IVec2) {
        let w = part_size.x as f32;
        let h = part_size.y as f32;
        let scale = self.clamped_scale();

        // Pivot in top-left image space.
        let pivot_img = Vec2::new(self.pivot.x * w, self.pivot.y * h);

        // Corner vectors relative to pivot, scaled by magnitude (sign irrelevant
        // for bounding — a flipped corner has the same distance from pivot).
        let abs_sx = scale.x.abs();
        let abs_sy = scale.y.abs();

        let corners = [
            Vec2::new((0.0 - pivot_img.x) * abs_sx, (0.0 - pivot_img.y) * abs_sy),
            Vec2::new((w - pivot_img.x) * abs_sx, (0.0 - pivot_img.y) * abs_sy),
            Vec2::new((w - pivot_img.x) * abs_sx, (h - pivot_img.y) * abs_sy),
            Vec2::new((0.0 - pivot_img.x) * abs_sx, (h - pivot_img.y) * abs_sy),
        ];

        // For each corner, its maximum reach over all rotation angles is its
        // distance from the pivot (the corner traces a circle of that radius).
        let max_radius = corners.iter().map(|c| c.length()).fold(0.0_f32, f32::max);

        let half = max_radius.ceil() as i32;

        // Pivot position in composite engine-space (bottom-left origin).
        let pivot_engine =
            part_offset.as_vec2() + Vec2::new(self.pivot.x * w, (1.0 - self.pivot.y) * h);

        let pivot_i = IVec2::new(pivot_engine.x.round() as i32, pivot_engine.y.round() as i32);

        (
            IVec2::new(pivot_i.x - half, pivot_i.y - half),
            IVec2::new(pivot_i.x + half, pivot_i.y + half),
        )
    }
}

/// A single part of a composite sprite.
#[derive(Clone, Debug)]
pub struct CxCompositePart {
    /// Source asset used for this part.
    pub source: CxCompositePartSource,
    /// Offset in composite-local pixel space from the part's bottom-left corner.
    ///
    /// This is an engine-space, bottom-left-oriented offset. If your asset pipeline or runtime
    /// manifest uses top-left image coordinates, convert into this space before constructing the
    /// part.
    pub offset: IVec2,
    /// Frame binding to the composite's master frame.
    pub frame: CxFrameBinding,
    /// Optional filter applied before the composite's filter.
    pub filter: Option<Handle<CxFilterAsset>>,
    /// Mirror the part horizontally at draw time.
    pub flip_x: bool,
    /// Mirror the part vertically at draw time.
    pub flip_y: bool,
    /// Optional per-part render-time transform (scale/rotation around pivot).
    ///
    /// When present and non-identity, the part is rendered into a mini scratch buffer
    /// and blitted with the transform during composition. `None` or identity uses the
    /// direct fast path.
    pub transform: Option<PartTransform>,
}

impl CxCompositePart {
    /// Create a composite part with default binding and zero offset.
    #[must_use]
    pub fn new(sprite: Handle<CxSpriteAsset>) -> Self {
        Self::from_source(CxCompositePartSource::Sprite(sprite))
    }

    /// Create a composite part from any supported source.
    #[must_use]
    pub fn from_source(source: CxCompositePartSource) -> Self {
        Self {
            source,
            offset: IVec2::ZERO,
            frame: CxFrameBinding::default(),
            filter: None,
            flip_x: false,
            flip_y: false,
            transform: None,
        }
    }

    /// Create a composite part that draws from an atlas region.
    #[must_use]
    pub fn atlas_region(atlas: Handle<CxSpriteAtlasAsset>, region: AtlasRegionId) -> Self {
        Self::from_source(CxCompositePartSource::AtlasRegion { atlas, region })
    }

    /// Set the part offset relative to the composite origin.
    #[must_use]
    pub fn with_offset(mut self, offset: IVec2) -> Self {
        self.offset = offset;
        self
    }

    /// Set how this part binds to the composite master frame.
    #[must_use]
    pub fn with_frame_binding(mut self, frame: CxFrameBinding) -> Self {
        self.frame = frame;
        self
    }

    /// Set an optional per-part filter.
    #[must_use]
    pub fn with_filter(mut self, filter: Option<Handle<CxFilterAsset>>) -> Self {
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

    /// Set a per-part render-time transform (scale/rotation around pivot).
    ///
    /// When set and non-identity, the part is rendered into a mini scratch buffer
    /// and blitted with the transform during composition.
    #[must_use]
    pub fn with_transform(mut self, transform: PartTransform) -> Self {
        self.transform = Some(transform);
        self
    }
}

pub(crate) fn log_composite_part_resolve_error(
    part_index: usize,
    error: &CxCompositePartResolveError,
) {
    match error {
        CxCompositePartResolveError::MissingSpriteAsset(handle) => {
            error!("skipping composite part {part_index}: missing sprite asset {handle:?}");
        }
        CxCompositePartResolveError::MissingAtlasAsset(handle) => {
            error!("skipping composite part {part_index}: missing atlas asset {handle:?}");
        }
        CxCompositePartResolveError::MissingAtlasRegion { atlas, region } => {
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
#[require(CxSprite)]
pub struct CxGpuSprite;

/// Marker to render a composite sprite via the experimental GPU palette path.
///
/// This is an optimization subset of [`CxCompositeSprite`], not a separate composite feature set.
/// A composite is GPU-eligible only when every part:
///
/// - uses [`CxCompositePartSource::Sprite`]
/// - has no per-part filter
/// - has `flip_x == false`
/// - has `flip_y == false`
///
/// Composites outside that subset fall back to the CPU renderer.
#[cfg(feature = "gpu_palette")]
#[derive(Component, Default, Clone, Copy, Debug)]
#[require(CxCompositeSprite)]
pub struct CxGpuComposite;

impl AnimatedAssetComponent for CxSprite {
    type Asset = CxSpriteAsset;

    fn handle(&self) -> &Handle<Self::Asset> {
        self
    }

    fn max_frame_count(sprite: &CxSpriteAsset) -> usize {
        sprite.frame_count()
    }
}

impl Spatial for CxCompositeSprite {
    fn frame_size(&self) -> UVec2 {
        self.size
    }
}

fn sync_composite_metrics(
    composite: &mut CxCompositeSprite,
    sprites: &Assets<CxSpriteAsset>,
    atlases: &Assets<CxSpriteAtlasAsset>,
    mut count: Option<Mut<CxFrameCount>>,
) {
    composite.recompute_metrics_with_atlases(sprites, atlases);
    if let Some(count) = count.as_mut() {
        count.0 = composite.frame_count;
    }
}

fn update_composite_metrics_on_change(
    sprites: Res<Assets<CxSpriteAsset>>,
    atlases: Res<Assets<CxSpriteAtlasAsset>>,
    mut diagnostics: Diagnostics,
    mut composites: Query<
        (
            &mut CxCompositeSprite,
            Option<&mut CxFrameCount>,
            Has<CxAuthoritativeCompositeMetrics>,
        ),
        Changed<CxCompositeSprite>,
    >,
) {
    let started = Instant::now();
    let mut composite_count = 0usize;
    let mut part_count = 0usize;

    for (mut composite, mut count, authoritative_metrics) in &mut composites {
        if authoritative_metrics {
            if let Some(count) = count.as_mut() {
                count.0 = composite.frame_count;
            }
            continue;
        }

        composite_count += 1;
        part_count += composite.parts.len();
        sync_composite_metrics(&mut composite, &sprites, &atlases, count);
    }

    diagnostics.add_measurement(&COMPOSITE_METRICS_ON_CHANGE_COUNT, || {
        composite_count as f64
    });
    diagnostics.add_measurement(&COMPOSITE_METRICS_ON_CHANGE_PARTS, || part_count as f64);
    diagnostics.add_measurement(&COMPOSITE_METRICS_ON_CHANGE_TIME, || {
        started.elapsed().as_secs_f64() * 1000.0
    });
}

fn update_composite_metrics_on_assets(
    sprites: Res<Assets<CxSpriteAsset>>,
    atlases: Res<Assets<CxSpriteAtlasAsset>>,
    mut sprite_events: MessageReader<AssetEvent<CxSpriteAsset>>,
    mut atlas_events: MessageReader<AssetEvent<CxSpriteAtlasAsset>>,
    mut composites: Query<(&mut CxCompositeSprite, Option<&mut CxFrameCount>)>,
) {
    if sprite_events.read().next().is_none() && atlas_events.read().next().is_none() {
        return;
    }

    for (mut composite, count) in &mut composites {
        sync_composite_metrics(&mut composite, &sprites, &atlases, count);
    }
}

fn sync_composite_frame_count_on_animation_added(
    sprites: Res<Assets<CxSpriteAsset>>,
    atlases: Res<Assets<CxSpriteAtlasAsset>>,
    mut composites: Query<(&mut CxCompositeSprite, &mut CxFrameCount), Added<CxAnimation>>,
) {
    for (mut composite, count) in &mut composites {
        sync_composite_metrics(&mut composite, &sprites, &atlases, Some(count));
    }
}

pub(crate) type SpriteComponents<L> = (
    &'static CxSprite,
    &'static CxPosition,
    &'static CxAnchor,
    &'static L,
    &'static CxRenderSpace,
    Option<&'static CxFrameView>,
    Option<&'static CxFilter>,
    Option<&'static crate::presentation::CxPresentationTransform>,
);

pub(crate) type CompositeSpriteComponents<L> = (
    &'static CxCompositeSprite,
    &'static CxPosition,
    &'static CxAnchor,
    &'static L,
    &'static CxRenderSpace,
    Option<&'static CxFrameView>,
    Option<&'static CxFilter>,
    Option<&'static crate::presentation::CxPresentationTransform>,
);

#[cfg(feature = "headed")]
fn extract_sprites<L: CxLayer>(
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
            entity.remove::<CxFrameView>();
        }

        if let Some(filter) = filter {
            entity.insert(filter.clone());
        } else {
            entity.remove::<CxFilter>();
        }

        if let Some(&presentation) = presentation {
            entity.insert(presentation);
        } else {
            entity.remove::<crate::presentation::CxPresentationTransform>();
        }
    }
}

#[cfg(feature = "headed")]
fn extract_composite_sprites<L: CxLayer>(
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
            entity.remove::<CxFrameView>();
        }

        if let Some(filter) = filter {
            entity.insert(filter.clone());
        } else {
            entity.remove::<CxFilter>();
        }

        if let Some(&presentation) = presentation {
            entity.insert(presentation);
        } else {
            entity.remove::<crate::presentation::CxPresentationTransform>();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;
    use crate::{
        atlas::{AtlasRect, CxSpriteAtlasAsset},
        camera::CxCamera,
        frame::{
            CxFrameSelector, CxFrameView, blit_transformed, draw_frame, draw_spatial,
            resolve_frame_binding,
        },
        image::CxImage,
    };
    use bevy_app::{App, Update};
    use bevy_asset::Assets;
    use bevy_diagnostic::DiagnosticsPlugin;
    use bevy_platform::collections::HashMap;
    #[cfg(feature = "headed")]
    use bevy_render::extract_component::ExtractComponent;
    use bevy_time::Time;
    use insta::assert_snapshot;

    #[cfg_attr(feature = "headed", derive(ExtractComponent))]
    #[derive(Component, next::Next, Ord, PartialOrd, Eq, PartialEq, Clone, Default, Debug)]
    enum TestLayer {
        #[default]
        Base,
    }

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

    fn image_grid(image: &CxImage) -> String {
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

    #[test]
    fn authoritative_metrics_skip_change_rescan() {
        let mut app = App::new();
        app.add_plugins(DiagnosticsPlugin)
            .insert_resource(Assets::<CxSpriteAsset>::default())
            .insert_resource(Assets::<CxSpriteAtlasAsset>::default())
            .init_resource::<Time>();
        crate::position::plug_core::<TestLayer>(&mut app);
        app.add_systems(Update, update_composite_metrics_on_change);

        let sprite = app
            .world_mut()
            .resource_mut::<Assets<CxSpriteAsset>>()
            .add(CxSpriteAsset {
                data: CxImage::new(vec![1; 4], 2),
                frame_size: 4,
            });

        let entity = app
            .world_mut()
            .spawn((
                CxAuthoritativeCompositeMetrics,
                CxCompositeSprite {
                    parts: vec![CxCompositePart::new(sprite)],
                    ..Default::default()
                },
                CxFrameCount(0),
            ))
            .id();

        {
            let mut entity_mut = app.world_mut().entity_mut(entity);
            let mut composite = entity_mut.get_mut::<CxCompositeSprite>().unwrap();
            composite.set_native_metrics(IVec2::new(7, -6), UVec2::new(9, 7), 1);
        }

        app.update();

        let entity_ref = app.world().entity(entity);
        let composite = entity_ref.get::<CxCompositeSprite>().unwrap();
        let frame_count = entity_ref.get::<CxFrameCount>().unwrap();

        assert_eq!(composite.origin, IVec2::new(7, -6));
        assert_eq!(composite.size, UVec2::new(9, 7));
        assert_eq!(composite.render_origin, IVec2::new(7, -6));
        assert_eq!(composite.render_size, UVec2::new(9, 7));
        assert_eq!(composite.frame_count, 1);
        assert_eq!(frame_count.0, 1);
    }

    /// Draw a composite into an image at base (non-render-envelope) size.
    /// Parts with a per-part transform are rendered via a mini scratch +
    /// `blit_transformed`, matching the real renderer's logic.
    fn draw_composite(
        image: &mut CxImage,
        composite: &CxCompositeSprite,
        master: Option<CxFrameView>,
        sprites: &Assets<CxSpriteAsset>,
        atlases: &Assets<CxSpriteAtlasAsset>,
    ) {
        draw_composite_into(image, composite, master, sprites, atlases, false);
    }

    /// Draw a composite into an image using the render-envelope (`render_origin`
    /// / `render_size`) for placement, as the real renderer does when per-part
    /// transforms are present.
    fn draw_composite_render(
        image: &mut CxImage,
        composite: &CxCompositeSprite,
        master: Option<CxFrameView>,
        sprites: &Assets<CxSpriteAsset>,
        atlases: &Assets<CxSpriteAtlasAsset>,
    ) {
        draw_composite_into(image, composite, master, sprites, atlases, true);
    }

    fn draw_composite_into(
        image: &mut CxImage,
        composite: &CxCompositeSprite,
        master: Option<CxFrameView>,
        sprites: &Assets<CxSpriteAsset>,
        atlases: &Assets<CxSpriteAtlasAsset>,
        use_render_envelope: bool,
    ) {
        let mut slice = image.slice_all_mut();
        let (origin, size) = if use_render_envelope {
            (composite.render_origin, composite.render_size)
        } else {
            (composite.origin, composite.size)
        };
        let base_pos = IVec2::ZERO - CxAnchor::BottomLeft.pos(size).as_ivec2();

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
            let drawable = CxCompositePartDrawable {
                resolved,
                flip_x: part.flip_x,
                flip_y: part.flip_y,
            };

            let needs_part_transform = part.transform.as_ref().is_some_and(|t| !t.is_identity());

            if needs_part_transform {
                let t = part.transform.as_ref().unwrap();
                let part_size = drawable.frame_size();
                if part_size.x == 0 || part_size.y == 0 {
                    continue;
                }
                // Render part into a mini scratch at native size.
                let mut mini = CxImage::empty(part_size);
                let mut mini_slice = mini.slice_all_mut();
                draw_frame(&drawable, (), &mut mini_slice, part_frame, []);

                // Pivot position in engine-space relative to composite origin.
                let part_bl = (part.offset - origin).as_vec2();
                let ps = part_size.as_vec2();
                let pivot_pos = part_bl + Vec2::new(t.pivot.x * ps.x, (1.0 - t.pivot.y) * ps.y);

                blit_transformed(
                    &mini,
                    part_size,
                    &mut slice,
                    crate::position::CxPosition(pivot_pos.round().as_ivec2()),
                    t.anchor(),
                    CxRenderSpace::Camera,
                    CxCamera(IVec2::ZERO),
                    t.clamped_scale(),
                    t.sanitised_rotation(),
                    Vec2::ZERO,
                );
            } else {
                let part_pos = base_pos + (part.offset - origin);
                draw_spatial(
                    &drawable,
                    (),
                    &mut slice,
                    part_pos.into(),
                    CxAnchor::BottomLeft,
                    CxRenderSpace::Camera,
                    part_frame,
                    [],
                    CxCamera::default(),
                );
            }
        }
    }

    #[test]
    fn sprite_draws_nonzero_pixels() {
        let sprite = CxSpriteAsset {
            data: CxImage::new(vec![0, 2, 3, 0], 2),
            frame_size: 4,
        };
        let mut image = CxImage::new(vec![1; 4], 2);
        let mut slice = image.slice_all_mut();

        draw_spatial(
            &sprite,
            (),
            &mut slice,
            CxPosition(IVec2::ZERO),
            CxAnchor::BottomLeft,
            CxRenderSpace::Camera,
            None,
            [],
            CxCamera::default(),
        );

        let expected = vec![1, 2, 3, 1];
        assert_eq!(pixels(&image), expected);
    }

    #[test]
    fn composite_sprite_snapshot() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let sprite_a = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![1, 2, 3, 4], 2),
            frame_size: 4,
        });
        let sprite_b = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![5, 6, 7, 8], 2),
            frame_size: 4,
        });

        let mut composite = CxCompositeSprite::new(vec![
            CxCompositePart {
                source: CxCompositePartSource::Sprite(sprite_a),
                offset: IVec2::ZERO,
                frame: CxFrameBinding::default(),
                filter: None,
                flip_x: false,
                flip_y: false,
                transform: None,
            },
            CxCompositePart {
                source: CxCompositePartSource::Sprite(sprite_b),
                offset: IVec2::new(2, 0),
                frame: CxFrameBinding::default(),
                filter: None,
                flip_x: false,
                flip_y: false,
                transform: None,
            },
        ]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);

        let mut image = CxImage::new(vec![0; 8], 4);
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
        let sprite_a = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![1, 2, 3, 4, 9, 10, 11, 12], 2),
            frame_size: 4,
        });
        let sprite_b = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![5, 6, 7, 8, 13, 14, 15, 16], 2),
            frame_size: 4,
        });

        let mut composite = CxCompositeSprite::new(vec![
            CxCompositePart {
                source: CxCompositePartSource::Sprite(sprite_a),
                offset: IVec2::ZERO,
                frame: CxFrameBinding::default(),
                filter: None,
                flip_x: false,
                flip_y: false,
                transform: None,
            },
            CxCompositePart {
                source: CxCompositePartSource::Sprite(sprite_b),
                offset: IVec2::new(2, 0),
                frame: CxFrameBinding::Offset(1),
                filter: None,
                flip_x: false,
                flip_y: false,
                transform: None,
            },
        ]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);

        let mut image_frame_0 = CxImage::new(vec![0; 8], 4);
        draw_composite(
            &mut image_frame_0,
            &composite,
            Some(CxFrameView::from(CxFrameSelector::Index(0.))),
            &sprites,
            &atlases,
        );

        let mut image_frame_1 = CxImage::new(vec![0; 8], 4);
        draw_composite(
            &mut image_frame_1,
            &composite,
            Some(CxFrameView::from(CxFrameSelector::Index(1.))),
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

    fn two_frame_atlas() -> CxSpriteAtlasAsset {
        CxSpriteAtlasAsset {
            size: UVec2::new(4, 2),
            data: CxImage::new(vec![1, 2, 5, 6, 3, 4, 7, 8], 4),
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
            animations: HashMap::default(),
        }
    }

    #[test]
    fn composite_metrics_support_mixed_sources() {
        let mut sprites = Assets::default();
        let mut atlases = Assets::default();
        let sprite = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![1, 2], 2),
            frame_size: 2,
        });
        let atlas = atlases.add(two_frame_atlas());

        let mut composite = CxCompositeSprite::new(vec![
            CxCompositePart {
                source: CxCompositePartSource::Sprite(sprite),
                offset: IVec2::new(-1, 1),
                frame: CxFrameBinding::default(),
                filter: None,
                flip_x: false,
                flip_y: false,
                transform: None,
            },
            CxCompositePart {
                source: CxCompositePartSource::AtlasRegion {
                    atlas,
                    region: AtlasRegionId(0),
                },
                offset: IVec2::new(1, -1),
                frame: CxFrameBinding::default(),
                filter: None,
                flip_x: false,
                flip_y: false,
                transform: None,
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

        let mut composite = CxCompositeSprite::new(vec![CxCompositePart {
            source: CxCompositePartSource::AtlasRegion {
                atlas,
                region: AtlasRegionId(0),
            },
            offset: IVec2::ZERO,
            frame: CxFrameBinding::Offset(1),
            filter: None,
            flip_x: false,
            flip_y: false,
            transform: None,
        }]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);

        let mut image = CxImage::new(vec![0; 4], 2);
        draw_composite(
            &mut image,
            &composite,
            Some(CxFrameView::from(CxFrameSelector::Index(0.))),
            &sprites,
            &atlases,
        );

        assert_eq!(pixels(&image), vec![5, 6, 7, 8]);
    }

    #[test]
    fn composite_part_flip_semantics() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let sprite = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![1, 2, 3, 4], 2),
            frame_size: 4,
        });

        let mut composite = CxCompositeSprite::new(vec![
            CxCompositePart {
                source: CxCompositePartSource::Sprite(sprite.clone()),
                offset: IVec2::ZERO,
                frame: CxFrameBinding::default(),
                filter: None,
                flip_x: true,
                flip_y: false,
                transform: None,
            },
            CxCompositePart {
                source: CxCompositePartSource::Sprite(sprite),
                offset: IVec2::new(2, 0),
                frame: CxFrameBinding::default(),
                filter: None,
                flip_x: false,
                flip_y: true,
                transform: None,
            },
        ]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);

        let mut image = CxImage::new(vec![0; 8], 4);
        draw_composite(&mut image, &composite, None, &sprites, &atlases);

        // Left part: h-flipped [2,1 / 4,3]. Right part: v-flipped [3,4 / 1,2].
        assert_eq!(image_grid(&image), "02 01 03 04\n04 03 01 02");
    }

    #[test]
    fn composite_part_source_reports_missing_atlas_asset() {
        let result = CxCompositePartSource::atlas_region(Handle::default(), AtlasRegionId(3))
            .resolve(
                |_: &Handle<CxSpriteAsset>| None,
                |_: &Handle<CxSpriteAtlasAsset>| None,
            );

        assert!(matches!(
            result,
            Err(CxCompositePartResolveError::MissingAtlasAsset(_))
        ));
    }

    #[test]
    fn composite_part_source_reports_missing_atlas_region() {
        let atlas = two_frame_atlas();
        let result = CxCompositePartSource::atlas_region(Handle::default(), AtlasRegionId(3))
            .resolve(
                |_: &Handle<CxSpriteAsset>| None,
                |_: &Handle<CxSpriteAtlasAsset>| Some(&atlas),
            );

        assert!(matches!(
            result,
            Err(CxCompositePartResolveError::MissingAtlasRegion {
                region: AtlasRegionId(3),
                ..
            })
        ));
    }

    // --- PartTransform tests ---

    #[test]
    fn part_transform_default_is_identity() {
        let t = PartTransform::default();
        assert!(t.is_identity());
        assert!(!t.has_scale());
        assert!(!t.has_rotation());
        assert!((t.pivot - Vec2::splat(0.5)).length() < f32::EPSILON);
    }

    #[test]
    fn part_transform_scale_detected() {
        let t = PartTransform {
            scale: Vec2::new(2.0, 1.0),
            ..Default::default()
        };
        assert!(!t.is_identity());
        assert!(t.has_scale());
    }

    #[test]
    fn part_transform_rotation_detected() {
        let t = PartTransform {
            rotation: 0.5,
            ..Default::default()
        };
        assert!(!t.is_identity());
        assert!(t.has_rotation());
    }

    #[test]
    fn part_transform_clamped_scale_preserves_sign() {
        let t = PartTransform {
            scale: Vec2::new(-2.0, -0.001),
            ..Default::default()
        };
        let s = t.clamped_scale();
        assert!((s.x - (-2.0)).abs() < f32::EPSILON);
        assert!(s.y < 0.0); // sign preserved
        assert!(s.y.abs() >= crate::presentation::MIN_SCALE);
    }

    #[test]
    fn part_transform_sanitised_rotation_handles_nan() {
        let t = PartTransform {
            rotation: f32::NAN,
            ..Default::default()
        };
        assert_eq!(t.sanitised_rotation(), 0.0);
    }

    #[test]
    fn part_transform_anchor_converts_pivot() {
        // Centre pivot → CxAnchor centre
        let t = PartTransform::default();
        let anchor = t.anchor();
        let pos = anchor.pos(UVec2::new(10, 10));
        assert_eq!(pos, UVec2::new(5, 5));

        // Top-left pivot (0, 0) → CxAnchor Custom(0, 1) = top-left
        let t = PartTransform {
            pivot: Vec2::new(0.0, 0.0),
            ..Default::default()
        };
        let pos = t.anchor().pos(UVec2::new(10, 10));
        assert_eq!(pos, UVec2::new(0, 10));

        // Bottom-right pivot (1, 1) → CxAnchor Custom(1, 0) = bottom-right
        let t = PartTransform {
            pivot: Vec2::new(1.0, 1.0),
            ..Default::default()
        };
        let pos = t.anchor().pos(UVec2::new(10, 10));
        assert_eq!(pos, UVec2::new(10, 0));
    }

    #[test]
    fn transformed_bounds_identity_matches_native() {
        let t = PartTransform::default();
        let (min, max) = t.transformed_bounds(IVec2::new(5, 10), UVec2::new(4, 6));
        assert_eq!(min, IVec2::new(5, 10));
        assert_eq!(max, IVec2::new(9, 16));
    }

    #[test]
    fn transformed_bounds_2x_scale_expands() {
        let t = PartTransform {
            scale: Vec2::splat(2.0),
            ..Default::default()
        };
        let (min, max) = t.transformed_bounds(IVec2::ZERO, UVec2::new(4, 4));
        // 2x scale around centre: expands from [-2, -2] to [6, 6] in engine space
        // (native [0..4] × 2 around pivot at 2,2)
        let w = (max.x - min.x) as u32;
        let h = (max.y - min.y) as u32;
        assert!(w >= 8, "width should be ~8, got {w}");
        assert!(h >= 8, "height should be ~8, got {h}");
    }

    #[test]
    fn transformed_bounds_rotation_expands() {
        let t = PartTransform {
            rotation: std::f32::consts::FRAC_PI_4, // 45°
            ..Default::default()
        };
        let (min, max) = t.transformed_bounds(IVec2::ZERO, UVec2::new(4, 4));
        let w = (max.x - min.x) as u32;
        let h = (max.y - min.y) as u32;
        // 45° rotation of a 4x4 square produces a diamond wider than 4.
        assert!(w > 4, "rotated width should exceed native 4, got {w}");
        assert!(h > 4, "rotated height should exceed native 4, got {h}");
    }

    #[test]
    fn transformed_bounds_negative_scale_same_as_positive() {
        let pos = PartTransform {
            scale: Vec2::splat(2.0),
            ..Default::default()
        };
        let neg = PartTransform {
            scale: Vec2::splat(-2.0),
            ..Default::default()
        };
        let (min_p, max_p) = pos.transformed_bounds(IVec2::ZERO, UVec2::new(4, 4));
        let (min_n, max_n) = neg.transformed_bounds(IVec2::ZERO, UVec2::new(4, 4));
        // Bounding box size should be the same regardless of sign.
        assert_eq!(max_p - min_p, max_n - min_n);
    }

    #[test]
    fn metrics_expand_render_size_for_transformed_part() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let sprite = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![1; 4], 2),
            frame_size: 4,
        });

        // Untransformed composite: just 2x2 at origin.
        let composite_plain = CxCompositeSprite::new(vec![CxCompositePart::new(sprite.clone())]);
        let metrics_plain = composite_plain
            .metrics_with(|source| {
                source
                    .resolve(|h| sprites.get(h), |h| atlases.get(h))
                    .ok()
                    .map(|r| r.metrics())
            })
            .unwrap();

        // Transformed composite: 2x scale on the same part.
        let composite_scaled =
            CxCompositeSprite::new(vec![CxCompositePart::new(sprite).with_transform(
                PartTransform {
                    scale: Vec2::splat(2.0),
                    ..Default::default()
                },
            )]);
        let metrics_scaled = composite_scaled
            .metrics_with(|source| {
                source
                    .resolve(|h| sprites.get(h), |h| atlases.get(h))
                    .ok()
                    .map(|r| r.metrics())
            })
            .unwrap();

        // Base size should be the same (native part bounds don't change).
        assert_eq!(
            metrics_scaled.size, metrics_plain.size,
            "base size should be unchanged by per-part transform",
        );

        // Render size should be larger (worst-case envelope expands).
        assert!(
            metrics_scaled.render_size.x > metrics_plain.render_size.x,
            "render width {} should exceed plain render width {}",
            metrics_scaled.render_size.x,
            metrics_plain.render_size.x,
        );
        assert!(
            metrics_scaled.render_size.y > metrics_plain.render_size.y,
            "render height {} should exceed plain render height {}",
            metrics_scaled.render_size.y,
            metrics_plain.render_size.y,
        );
    }

    // --- worst_case_bounds / stable envelope tests ---

    #[test]
    fn worst_case_bounds_independent_of_rotation() {
        let size = UVec2::new(18, 33);
        let offset = IVec2::new(-18, -34);

        let angles = [0.0, 0.3, 0.7, 1.5, std::f32::consts::PI, 4.0, 6.0];
        let mut results = Vec::new();
        for angle in &angles {
            let t = PartTransform {
                rotation: *angle,
                pivot: Vec2::new(1.0, 0.1),
                ..Default::default()
            };
            results.push(t.worst_case_bounds(offset, size));
        }

        // All rotations must produce identical bounds.
        for (i, result) in results.iter().enumerate().skip(1) {
            assert_eq!(
                results[0], *result,
                "worst_case_bounds at angle {} ({}) differs from angle 0 ({}): {:?} vs {:?}",
                i, angles[i], angles[0], result, results[0],
            );
        }
    }

    #[test]
    fn worst_case_bounds_uses_scale_magnitude_not_sign() {
        let pos = PartTransform {
            scale: Vec2::splat(2.0),
            ..Default::default()
        };
        let neg = PartTransform {
            scale: Vec2::splat(-2.0),
            ..Default::default()
        };
        let (min_p, max_p) = pos.worst_case_bounds(IVec2::ZERO, UVec2::new(4, 4));
        let (min_n, max_n) = neg.worst_case_bounds(IVec2::ZERO, UVec2::new(4, 4));
        assert_eq!((min_p, max_p), (min_n, max_n));
    }

    #[test]
    fn worst_case_bounds_expands_beyond_native() {
        let t = PartTransform {
            scale: Vec2::splat(2.0),
            ..Default::default()
        };
        let (min, max) = t.worst_case_bounds(IVec2::ZERO, UVec2::new(4, 4));
        let w = (max.x - min.x) as u32;
        let h = (max.y - min.y) as u32;
        assert!(w > 4, "worst-case width {w} should exceed native 4");
        assert!(h > 4, "worst-case height {h} should exceed native 4");
    }

    #[test]
    fn worst_case_bounds_contains_all_transformed_bounds() {
        let size = UVec2::new(18, 33);
        let offset = IVec2::new(-5, 3);

        let t_wc = PartTransform {
            scale: Vec2::new(1.5, 1.2),
            pivot: Vec2::new(0.8, 0.2),
            ..Default::default()
        };
        let (wc_min, wc_max) = t_wc.worst_case_bounds(offset, size);

        // Sample many rotations — each current-frame bounds must fit inside worst-case.
        for i in 0..36 {
            let angle = (i as f32 / 36.0) * std::f32::consts::TAU;
            let t = PartTransform {
                rotation: angle,
                scale: Vec2::new(1.5, 1.2),
                pivot: Vec2::new(0.8, 0.2),
            };
            let (tf_min, tf_max) = t.transformed_bounds(offset, size);
            assert!(
                wc_min.x <= tf_min.x
                    && wc_min.y <= tf_min.y
                    && wc_max.x >= tf_max.x
                    && wc_max.y >= tf_max.y,
                "worst-case [{wc_min},{wc_max}] must contain transformed [{tf_min},{tf_max}] at angle {angle:.2}",
            );
        }
    }

    #[test]
    fn metrics_stable_across_rotation_changes() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let sprite = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![1; 4], 2),
            frame_size: 4,
        });

        let resolve = |source: &CxCompositePartSource| {
            source
                .resolve(|h| sprites.get(h), |h| atlases.get(h))
                .ok()
                .map(|r| r.metrics())
        };

        // All composites have a non-identity scale so they take the
        // worst_case_bounds path, with different rotation values.
        let composite_a =
            CxCompositeSprite::new(vec![CxCompositePart::new(sprite.clone()).with_transform(
                PartTransform {
                    rotation: 0.1, // non-zero so is_identity() is false
                    ..Default::default()
                },
            )]);
        let metrics_a = composite_a.metrics_with(resolve).unwrap();

        let composite_b =
            CxCompositeSprite::new(vec![CxCompositePart::new(sprite.clone()).with_transform(
                PartTransform {
                    rotation: std::f32::consts::FRAC_PI_4,
                    ..Default::default()
                },
            )]);
        let metrics_b = composite_b.metrics_with(resolve).unwrap();

        let composite_c =
            CxCompositeSprite::new(vec![CxCompositePart::new(sprite).with_transform(
                PartTransform {
                    rotation: std::f32::consts::PI,
                    ..Default::default()
                },
            )]);
        let metrics_c = composite_c.metrics_with(resolve).unwrap();

        // All must produce identical base and render metrics.
        assert_eq!(
            metrics_a.origin, metrics_b.origin,
            "base origin must be stable"
        );
        assert_eq!(metrics_a.origin, metrics_c.origin);
        assert_eq!(metrics_a.size, metrics_b.size, "base size must be stable");
        assert_eq!(metrics_a.size, metrics_c.size);
        assert_eq!(
            metrics_a.render_origin, metrics_b.render_origin,
            "render origin must be stable"
        );
        assert_eq!(metrics_a.render_origin, metrics_c.render_origin);
        assert_eq!(
            metrics_a.render_size, metrics_b.render_size,
            "render size must be stable"
        );
        assert_eq!(metrics_a.render_size, metrics_c.render_size);
    }

    #[test]
    fn base_metrics_unaffected_by_part_transform() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let sprite = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![1; 4], 2),
            frame_size: 4,
        });

        let resolve = |source: &CxCompositePartSource| {
            source
                .resolve(|h| sprites.get(h), |h| atlases.get(h))
                .ok()
                .map(|r| r.metrics())
        };

        // Composite without any transform.
        let plain = CxCompositeSprite::new(vec![CxCompositePart::new(sprite.clone())]);
        let m_plain = plain.metrics_with(resolve).unwrap();

        // Same composite with a big rotation + scale transform.
        let transformed =
            CxCompositeSprite::new(vec![CxCompositePart::new(sprite).with_transform(
                PartTransform {
                    scale: Vec2::splat(3.0),
                    rotation: 1.0,
                    ..Default::default()
                },
            )]);
        let m_trans = transformed.metrics_with(resolve).unwrap();

        // Base origin and size must be identical — transforms don't affect placement.
        assert_eq!(
            m_plain.origin, m_trans.origin,
            "base origin must not change"
        );
        assert_eq!(m_plain.size, m_trans.size, "base size must not change");

        // Render envelope must be larger.
        assert!(
            m_trans.render_size.x >= m_trans.size.x,
            "render size must be >= base size",
        );
    }

    // ---- Exact pixel-grid matrix tests ----
    //
    // All use a 2x2 asymmetric pattern with 4 distinct values (image-space,
    // top-left origin):
    //
    //   1 2
    //   3 4
    //
    // This makes any orientation mistake immediately visible.

    /// Shorthand: build a 2x2 `CxSpriteAsset` with 4 distinct palette indices.
    fn sprite_2x2(sprites: &mut Assets<CxSpriteAsset>) -> Handle<CxSpriteAsset> {
        sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![1, 2, 3, 4], 2),
            frame_size: 4,
        })
    }

    /// Shorthand: draw a one-part composite and return the pixel grid string.
    fn composite_one_part_grid(
        part: CxCompositePart,
        sprites: &Assets<CxSpriteAsset>,
        atlases: &Assets<CxSpriteAtlasAsset>,
    ) -> String {
        let mut composite = CxCompositeSprite::new(vec![part]);
        composite.recompute_metrics_with_atlases(sprites, atlases);
        let mut image = CxImage::new(
            vec![0; (composite.size.x * composite.size.y) as usize],
            composite.size.x as usize,
        );
        draw_composite(&mut image, &composite, None, sprites, atlases);
        image_grid(&image)
    }

    // --- Authored flip exact tests ---

    #[test]
    fn authored_flip_x_exact() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let s = sprite_2x2(&mut sprites);
        let grid = composite_one_part_grid(
            CxCompositePart {
                flip_x: true,
                ..CxCompositePart::new(s)
            },
            &sprites,
            &atlases,
        );
        // Columns mirrored: 1↔2, 3↔4.
        assert_eq!(grid, "02 01\n04 03");
    }

    #[test]
    fn authored_flip_y_exact() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let s = sprite_2x2(&mut sprites);
        let grid = composite_one_part_grid(
            CxCompositePart {
                flip_y: true,
                ..CxCompositePart::new(s)
            },
            &sprites,
            &atlases,
        );
        // Rows mirrored: row0↔row1.
        assert_eq!(grid, "03 04\n01 02");
    }

    #[test]
    fn authored_flip_both_exact() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let s = sprite_2x2(&mut sprites);
        let grid = composite_one_part_grid(
            CxCompositePart {
                flip_x: true,
                flip_y: true,
                ..CxCompositePart::new(s)
            },
            &sprites,
            &atlases,
        );
        // Both axes flipped = 180° rotation.
        assert_eq!(grid, "04 03\n02 01");
    }

    // --- Composite placement exact tests ---

    #[test]
    fn composite_two_parts_side_by_side_exact() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let a = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![1, 2, 3, 4], 2),
            frame_size: 4,
        });
        let b = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![5, 6, 7, 8], 2),
            frame_size: 4,
        });
        let mut composite = CxCompositeSprite::new(vec![
            CxCompositePart::new(a),
            CxCompositePart {
                offset: IVec2::new(2, 0),
                ..CxCompositePart::new(b)
            },
        ]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);
        let mut image = CxImage::new(vec![0; 8], 4);
        draw_composite(&mut image, &composite, None, &sprites, &atlases);
        assert_eq!(image_grid(&image), "01 02 05 06\n03 04 07 08");
    }

    #[test]
    fn composite_one_flipped_one_not_exact() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let s = sprite_2x2(&mut sprites);
        let mut composite = CxCompositeSprite::new(vec![
            // Left: unflipped.
            CxCompositePart::new(s.clone()),
            // Right: h-flipped.
            CxCompositePart {
                offset: IVec2::new(2, 0),
                flip_x: true,
                ..CxCompositePart::new(s)
            },
        ]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);
        let mut image = CxImage::new(vec![0; 8], 4);
        draw_composite(&mut image, &composite, None, &sprites, &atlases);
        // Left: 1 2 / 3 4, Right (h-flip): 2 1 / 4 3
        assert_eq!(image_grid(&image), "01 02 02 01\n03 04 04 03");
    }

    #[test]
    fn composite_part_with_negative_offset_exact() {
        // Part B is placed at x=-2, overlapping to the left of part A.
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let a = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![1, 2, 3, 4], 2),
            frame_size: 4,
        });
        let b = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![5, 6, 7, 8], 2),
            frame_size: 4,
        });
        let mut composite = CxCompositeSprite::new(vec![
            CxCompositePart::new(a),
            CxCompositePart {
                offset: IVec2::new(-2, 0),
                ..CxCompositePart::new(b)
            },
        ]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);
        // Composite should be 4 wide: B at x=-2, A at x=0.
        assert_eq!(composite.size, UVec2::new(4, 2));
        let mut image = CxImage::new(vec![0; 8], 4);
        draw_composite(&mut image, &composite, None, &sprites, &atlases);
        // B occupies columns 0-1, A occupies columns 2-3.
        // A draws second, so it overwrites any overlap (no overlap here).
        assert_eq!(image_grid(&image), "05 06 01 02\n07 08 03 04");
    }

    #[test]
    fn composite_vertical_offset_exact() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let a = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![1, 2, 3, 4], 2),
            frame_size: 4,
        });
        let b = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![5, 6, 7, 8], 2),
            frame_size: 4,
        });
        let mut composite = CxCompositeSprite::new(vec![
            CxCompositePart::new(a),
            CxCompositePart {
                offset: IVec2::new(0, -2), // below part A in engine-space (Y-up)
                ..CxCompositePart::new(b)
            },
        ]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);
        assert_eq!(composite.size, UVec2::new(2, 4));
        let mut image = CxImage::new(vec![0; 8], 2);
        draw_composite(&mut image, &composite, None, &sprites, &atlases);
        // Engine Y-up → image Y-down: A is on top, B on bottom.
        assert_eq!(image_grid(&image), "01 02\n03 04\n05 06\n07 08");
    }

    #[test]
    fn composite_overlapping_parts_draw_order() {
        // Two 2x2 parts at the same position. The second (later) part should
        // overwrite non-zero pixels of the first.
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let a = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![1, 2, 3, 4], 2),
            frame_size: 4,
        });
        let b = sprites.add(CxSpriteAsset {
            data: CxImage::new(vec![5, 0, 0, 8], 2), // only corners non-zero
            frame_size: 4,
        });
        let mut composite = CxCompositeSprite::new(vec![
            CxCompositePart::new(a),
            CxCompositePart::new(b), // same offset, drawn second
        ]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);
        let mut image = CxImage::new(vec![0; 4], 2);
        draw_composite(&mut image, &composite, None, &sprites, &atlases);
        // B's non-zero pixels (5, 8) overwrite A's (1, 4). A's (2, 3) survive.
        assert_eq!(image_grid(&image), "05 02\n03 08");
    }

    // --- Authored flip + runtime flip composition ---

    #[test]
    fn authored_hflip_plus_runtime_hflip_cancels() {
        // Authored h-flip + runtime h-flip (signed scale) should cancel out
        // and produce the original orientation.
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let s = sprite_2x2(&mut sprites);
        let mut composite = CxCompositeSprite::new(vec![CxCompositePart {
            flip_x: true,
            transform: Some(PartTransform {
                scale: Vec2::new(-1.0, 1.0),
                ..Default::default()
            }),
            ..CxCompositePart::new(s)
        }]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);
        // Draw using the render envelope since there's a transform.
        let mut image = CxImage::new(
            vec![0; (composite.render_size.x * composite.render_size.y) as usize],
            composite.render_size.x as usize,
        );
        draw_composite_render(&mut image, &composite, None, &sprites, &atlases);
        // Extract non-zero pixels.
        let grid = image.nonzero_grid();
        // Authored h-flip produces [2,1 / 4,3], then runtime h-flip mirrors
        // back to [1,2 / 3,4].
        assert_eq!(grid, vec![vec![1, 2], vec![3, 4]]);
    }

    #[test]
    fn authored_vflip_plus_runtime_vflip_cancels() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let s = sprite_2x2(&mut sprites);
        let mut composite = CxCompositeSprite::new(vec![CxCompositePart {
            flip_y: true,
            transform: Some(PartTransform {
                scale: Vec2::new(1.0, -1.0),
                ..Default::default()
            }),
            ..CxCompositePart::new(s)
        }]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);
        let mut image = CxImage::new(
            vec![0; (composite.render_size.x * composite.render_size.y) as usize],
            composite.render_size.x as usize,
        );
        draw_composite_render(&mut image, &composite, None, &sprites, &atlases);
        let grid = image.nonzero_grid();
        assert_eq!(grid, vec![vec![1, 2], vec![3, 4]]);
    }

    // --- Per-part transform composite tests ---

    #[test]
    fn composite_one_transformed_one_static() {
        // Two parts side by side: left is 180° rotated, right is static.
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let s = sprite_2x2(&mut sprites);
        let mut composite = CxCompositeSprite::new(vec![
            CxCompositePart {
                transform: Some(PartTransform {
                    rotation: std::f32::consts::PI,
                    ..Default::default()
                }),
                ..CxCompositePart::new(s.clone())
            },
            CxCompositePart {
                offset: IVec2::new(2, 0),
                ..CxCompositePart::new(s)
            },
        ]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);

        let mut image = CxImage::new(
            vec![0; (composite.render_size.x * composite.render_size.y) as usize],
            composite.render_size.x as usize,
        );
        draw_composite_render(&mut image, &composite, None, &sprites, &atlases);

        let grid = image.nonzero_grid();
        // The render envelope may be larger than 2 rows due to worst-case bounds.
        // Verify both parts are present by checking the full content.
        let flat: Vec<u8> = grid.iter().flat_map(|r| r.iter().copied()).collect();
        for v in 1..=4_u8 {
            assert!(
                flat.contains(&v),
                "output should contain value {v}, got {grid:?}"
            );
        }
        // Check that the rotated [4,3] and static [1,2] patterns appear in some row.
        let has_rotated_top = grid.iter().any(|r| r.windows(2).any(|w| w == [4, 3]));
        let has_static_top = grid.iter().any(|r| r.windows(2).any(|w| w == [1, 2]));
        let has_rotated_bot = grid.iter().any(|r| r.windows(2).any(|w| w == [2, 1]));
        let has_static_bot = grid.iter().any(|r| r.windows(2).any(|w| w == [3, 4]));
        assert!(
            has_rotated_top,
            "should contain rotated row [4,3], got {grid:?}"
        );
        assert!(
            has_static_top,
            "should contain static row [1,2], got {grid:?}"
        );
        assert!(
            has_rotated_bot,
            "should contain rotated row [2,1], got {grid:?}"
        );
        assert!(
            has_static_bot,
            "should contain static row [3,4], got {grid:?}"
        );
    }

    #[test]
    fn composite_part_scale_2x_render_envelope() {
        // A single 2x2 part with 2x scale needs a larger render envelope.
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let s = sprite_2x2(&mut sprites);
        let mut composite = CxCompositeSprite::new(vec![CxCompositePart {
            transform: Some(PartTransform {
                scale: Vec2::splat(2.0),
                ..Default::default()
            }),
            ..CxCompositePart::new(s)
        }]);
        composite.recompute_metrics_with_atlases(&sprites, &atlases);

        // Base size stays 2x2 (native part bounds).
        assert_eq!(composite.size, UVec2::new(2, 2));
        // Render size must be larger to accommodate 2x scale.
        assert!(
            composite.render_size.x > composite.size.x
                && composite.render_size.y > composite.size.y,
            "render_size {:?} should exceed base size {:?}",
            composite.render_size,
            composite.size,
        );

        let mut image = CxImage::new(
            vec![0; (composite.render_size.x * composite.render_size.y) as usize],
            composite.render_size.x as usize,
        );
        draw_composite_render(&mut image, &composite, None, &sprites, &atlases);

        let grid = image.nonzero_grid();
        // 2x nearest-neighbour of [1,2/3,4] should produce each pixel doubled.
        // Due to center-anchor rounding the output may be 3x3 instead of 4x4
        // (same as the blit_grid tests in frame.rs), but all 4 values must appear.
        let flat: Vec<u8> = grid.iter().flat_map(|r| r.iter().copied()).collect();
        for v in 1..=4_u8 {
            assert!(
                flat.contains(&v),
                "scaled output should contain value {v}, got {grid:?}"
            );
        }
        // At least 3x3 = 9 pixels (2x scale of 4 pixels).
        assert!(
            flat.len() >= 9,
            "expected at least 9 pixels, got {}",
            flat.len()
        );
    }

    // --- Invariant: authored flip is purely data-level, not spatial ---

    #[test]
    fn authored_flip_does_not_change_metrics() {
        let mut sprites = Assets::default();
        let atlases = Assets::default();
        let s = sprite_2x2(&mut sprites);

        let resolve = |source: &CxCompositePartSource| {
            source
                .resolve(|h| sprites.get(h), |h| atlases.get(h))
                .ok()
                .map(|r| r.metrics())
        };

        let plain = CxCompositeSprite::new(vec![CxCompositePart::new(s.clone())]);
        let flipped = CxCompositeSprite::new(vec![CxCompositePart {
            flip_x: true,
            flip_y: true,
            ..CxCompositePart::new(s)
        }]);

        let m_plain = plain.metrics_with(resolve).unwrap();
        let m_flip = flipped.metrics_with(resolve).unwrap();

        assert_eq!(m_plain.origin, m_flip.origin);
        assert_eq!(m_plain.size, m_flip.size);
        assert_eq!(m_plain.render_origin, m_flip.render_origin);
        assert_eq!(m_plain.render_size, m_flip.render_size);
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
// pub struct ImageToSpriteBundle<L: CxLayer> {
//     /// A [`Handle<CxSprite>`] component
//     pub image: ImageToSprite,
//     /// A [`CxPosition`] component
//     pub position: CxPosition,
//     /// A [`CxAnchor`] component
//     pub anchor: CxAnchor,
//     /// A layer component
//     pub layer: L,
//     /// A [`CxRenderSpace`] component
//     pub canvas: CxRenderSpace,
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
//     &'static CxPosition,
//     &'static CxAnchor,
//     &'static L,
//     &'static CxRenderSpace,
//     Option<&'static Handle<CxFilter>>,
// );
//
// fn extract_image_to_sprites<L: CxLayer>(
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
