//! Sprite atlas assets (metadata + palette-indexed image data).
//! Atlas metadata is loaded from `.px_atlas.ron`.
//!
//! The `indexed_image` field in `.px_atlas.ron` points to a `.pxi` file containing
//! pre-computed palette indices. Paths are Bevy asset-server paths relative to the
//! game's asset root, not relative to the `.px_atlas.ron` file itself.

use std::{collections::BTreeMap, error::Error, path::PathBuf};

use bevy_asset::{AssetId, AssetLoader, LoadContext, io::Reader};
use bevy_math::{ivec2, uvec2};
use bevy_platform::collections::HashMap;
use bevy_reflect::TypePath;
#[cfg(feature = "headed")]
use bevy_render::{
    Extract, RenderApp,
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin},
    sync_component::SyncComponentPlugin,
    sync_world::RenderEntity,
};
use serde::{Deserialize, Serialize};

use crate::{
    frame::Frames,
    image::{PxImage, PxImageSliceMut},
    position::{DefaultLayer, PxLayer, Spatial},
    prelude::*,
};

pub(crate) fn plug_core(app: &mut App, _palette_path: PathBuf) {
    app.init_asset::<crate::pxi::PxIndexedImage>()
        .register_asset_loader(crate::pxi::PxiLoader)
        .init_asset::<PxSpriteAtlasAsset>()
        .register_asset_loader(PxSpriteAtlasLoader);
}

pub(crate) fn plug<L: PxLayer>(app: &mut App, palette_path: PathBuf) {
    #[cfg(feature = "headed")]
    app.add_plugins((
        RenderAssetPlugin::<PxSpriteAtlasAsset>::default(),
        SyncComponentPlugin::<PxAtlasSprite>::default(),
    ));

    plug_core(app, palette_path);

    #[cfg(feature = "headed")]
    app.sub_app_mut(RenderApp)
        .add_systems(ExtractSchedule, extract_atlas_sprites::<L>);
}

#[derive(Serialize, Deserialize)]
struct PxSpriteAtlasDescriptor {
    /// Path to the compact indexed runtime image (.pxi), relative to the game
    /// asset root. Contains pre-computed palette indices — no PNG decode or
    /// palette lookup needed.
    indexed_image: PathBuf,
    regions: Vec<AtlasRegionDescriptor>,
    #[serde(default)]
    names: BTreeMap<String, u32>,
}

#[derive(Serialize, Deserialize)]
struct AtlasRegionDescriptor {
    frame_size: [u32; 2],
    frames: Vec<AtlasRect>,
}

#[derive(TypePath)]
struct PxSpriteAtlasLoader;

impl AssetLoader for PxSpriteAtlasLoader {
    type Asset = PxSpriteAtlasAsset;
    type Settings = ();
    type Error = Box<dyn Error + Send + Sync>;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        (): &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<PxSpriteAtlasAsset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let descriptor: PxSpriteAtlasDescriptor =
            ron::de::from_bytes(&bytes).map_err(|err| err.to_string())?;

        let indexed = load_context
            .loader()
            .immediate()
            .load::<crate::pxi::PxIndexedImage>(descriptor.indexed_image.clone())
            .await
            .map_err(|err| format!("failed to load indexed image: {err}"))?;
        let data = indexed.get().image.clone();
        let size = data.size();

        let mut regions = Vec::with_capacity(descriptor.regions.len());
        for (region_index, region) in descriptor.regions.iter().enumerate() {
            regions.push(build_region(region_index, region, size)?);
        }

        let mut names = HashMap::default();
        for (name, &index) in &descriptor.names {
            if index as usize >= regions.len() {
                return Err(
                    format!("atlas region name '{name}' points to missing index {index}").into(),
                );
            }
            names.insert(name.clone(), AtlasRegionId(index));
        }

        Ok(PxSpriteAtlasAsset {
            size,
            data,
            regions,
            names,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["px_atlas.ron"]
    }
}

#[cfg(feature = "headed")]
impl RenderAsset for PxSpriteAtlasAsset {
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

/// A palette-indexed sprite atlas.
///
/// Atlas assets are loaded from `.px_atlas.ron` metadata. The metadata
/// `indexed_image` field must be an asset-server path relative to the game
/// asset root. It is not interpreted relative to the `.px_atlas.ron` file, and
/// absolute filesystem paths are not supported.
///
/// Rects use atlas image coordinates with the origin at the top-left.
#[derive(Asset, Clone, Reflect, Debug)]
pub struct PxSpriteAtlasAsset {
    pub(crate) size: UVec2,
    pub(crate) data: PxImage,
    pub(crate) regions: Vec<AtlasRegion>,
    pub(crate) names: HashMap<String, AtlasRegionId>,
}

impl PxSpriteAtlasAsset {
    /// Atlas pixel dimensions.
    #[must_use]
    pub fn size(&self) -> UVec2 {
        self.size
    }

    /// Resolve a region id by name.
    #[must_use]
    pub fn region_id(&self, name: &str) -> Option<AtlasRegionId> {
        self.names.get(name).copied()
    }

    /// Look up a region by id.
    #[must_use]
    pub fn region(&self, id: AtlasRegionId) -> Option<&AtlasRegion> {
        self.regions.get(id.0 as usize)
    }

    /// All regions in this atlas.
    #[must_use]
    pub fn regions(&self) -> &[AtlasRegion] {
        &self.regions
    }
}

/// Identifier for a region within a [`PxSpriteAtlasAsset`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub struct AtlasRegionId(pub u32);

/// A rectangular atlas frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub struct AtlasRect {
    /// Left pixel coordinate.
    pub x: u32,
    /// Top pixel coordinate.
    pub y: u32,
    /// Width in pixels.
    pub w: u32,
    /// Height in pixels.
    pub h: u32,
}

impl AtlasRect {
    /// Size of the rectangle in pixels.
    #[must_use]
    pub fn size(&self) -> UVec2 {
        UVec2::new(self.w, self.h)
    }

    fn max(&self) -> Option<UVec2> {
        Some(UVec2::new(
            self.x.checked_add(self.w)?,
            self.y.checked_add(self.h)?,
        ))
    }
}

/// A region within a sprite atlas, with per-frame bounds.
#[derive(Clone, Debug, Reflect)]
pub struct AtlasRegion {
    /// Size of each frame in pixels.
    pub frame_size: UVec2,
    /// Frame rectangles within the atlas.
    pub frames: Vec<AtlasRect>,
}

impl AtlasRegion {
    /// Number of frames in the region.
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get the frame rectangle at an index.
    #[must_use]
    pub fn frame(&self, index: usize) -> Option<AtlasRect> {
        self.frames.get(index).copied()
    }
}

/// A sprite that draws from a region within an atlas.
#[derive(Component, Default, Clone, Debug)]
#[require(PxPosition, PxAnchor, DefaultLayer, PxCanvas)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct PxAtlasSprite {
    /// Atlas asset handle.
    pub atlas: Handle<PxSpriteAtlasAsset>,
    /// Selected region within the atlas.
    pub region: AtlasRegionId,
}

impl PxAtlasSprite {
    /// Create a new atlas sprite pointing at a region.
    #[must_use]
    pub fn new(atlas: Handle<PxSpriteAtlasAsset>, region: AtlasRegionId) -> Self {
        Self { atlas, region }
    }
}

pub(crate) type AtlasSpriteComponents<L> = (
    &'static PxAtlasSprite,
    &'static PxPosition,
    &'static PxAnchor,
    &'static L,
    &'static PxCanvas,
    Option<&'static PxFrame>,
    Option<&'static PxFilter>,
);

#[cfg(feature = "headed")]
fn extract_atlas_sprites<L: PxLayer>(
    atlas_sprites: Extract<Query<(AtlasSpriteComponents<L>, &InheritedVisibility, RenderEntity)>>,
    mut cmd: Commands,
) {
    for ((sprite, &position, &anchor, layer, &canvas, frame, filter), visibility, id) in
        &atlas_sprites
    {
        let mut entity = cmd.entity(id);

        if !visibility.get() {
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
    }
}

fn build_region(
    region_index: usize,
    region: &AtlasRegionDescriptor,
    atlas_size: UVec2,
) -> Result<AtlasRegion, String> {
    let frame_size = UVec2::new(region.frame_size[0], region.frame_size[1]);
    if frame_size.x == 0 || frame_size.y == 0 {
        return Err(format!("atlas region {region_index} has zero frame_size"));
    }
    if region.frames.is_empty() {
        return Err(format!("atlas region {region_index} has no frames"));
    }

    for (frame_index, rect) in region.frames.iter().enumerate() {
        if rect.w == 0 || rect.h == 0 {
            return Err(format!(
                "atlas region {region_index} frame {frame_index} has zero size"
            ));
        }
        if rect.size() != frame_size {
            return Err(format!(
                "atlas region {region_index} frame {frame_index} size does not match frame_size"
            ));
        }
        let Some(max) = rect.max() else {
            return Err(format!(
                "atlas region {region_index} frame {frame_index} overflows bounds"
            ));
        };
        if max.x > atlas_size.x || max.y > atlas_size.y {
            return Err(format!(
                "atlas region {region_index} frame {frame_index} exceeds atlas bounds"
            ));
        }
    }

    Ok(AtlasRegion {
        frame_size,
        frames: region.frames.clone(),
    })
}

impl Frames for (&PxSpriteAtlasAsset, &AtlasRegion) {
    type Param = ();

    fn frame_count(&self) -> usize {
        self.1.frame_count()
    }

    fn draw(
        &self,
        (): (),
        image: &mut PxImageSliceMut,
        frame: impl Fn(UVec2) -> usize,
        filter: impl Fn(u8) -> u8,
    ) {
        let (atlas, region) = *self;
        let frame_width = region.frame_size.x as usize;
        let image_width = image.image_width();

        image.for_each_mut(|slice_i, image_i, pixel| {
            let x = (slice_i % frame_width) as u32;
            let y = (slice_i / frame_width) as u32;
            let frame_index = frame(uvec2(
                (image_i % image_width) as u32,
                (image_i / image_width) as u32,
            ));
            let rect = &region.frames[frame_index];
            let src_x = rect.x + x;
            let src_y = rect.y + y;

            if let Some(value) = atlas.data.get_pixel(ivec2(src_x as i32, src_y as i32))
                && value != 0
            {
                *pixel = filter(value);
            }
        });
    }
}

impl Spatial for (&PxSpriteAtlasAsset, &AtlasRegion) {
    fn frame_size(&self) -> UVec2 {
        self.1.frame_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::PxCamera;
    use crate::frame::{PxFrameSelector, PxFrameView, draw_frame, draw_spatial};
    use crate::image::PxImage;
    use bevy_math::IVec2;
    use bevy_platform::collections::HashMap;

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

    #[test]
    fn atlas_region_draws_selected_frame() {
        let atlas = PxSpriteAtlasAsset {
            size: UVec2::new(4, 1),
            data: PxImage::new(vec![1, 2, 3, 4], 4),
            regions: vec![AtlasRegion {
                frame_size: UVec2::new(2, 1),
                frames: vec![
                    AtlasRect {
                        x: 0,
                        y: 0,
                        w: 2,
                        h: 1,
                    },
                    AtlasRect {
                        x: 2,
                        y: 0,
                        w: 2,
                        h: 1,
                    },
                ],
            }],
            names: HashMap::default(),
        };

        let region = &atlas.regions[0];
        let mut image = PxImage::new(vec![0; 2], 2);
        let mut slice = image.slice_all_mut();

        draw_frame(
            &(&atlas, region),
            (),
            &mut slice,
            Some(PxFrameView::from(PxFrameSelector::Index(1.))),
            [],
        );

        assert_eq!(pixels(&image), vec![3, 4]);
    }

    fn one_frame_descriptor() -> AtlasRegionDescriptor {
        AtlasRegionDescriptor {
            frame_size: [2, 2],
            frames: vec![AtlasRect {
                x: 0,
                y: 0,
                w: 2,
                h: 2,
            }],
        }
    }

    // build_region: happy path

    #[test]
    fn build_region_ok() {
        let desc = one_frame_descriptor();
        let region = build_region(0, &desc, UVec2::new(4, 4)).unwrap();
        assert_eq!(region.frame_size, UVec2::new(2, 2));
        assert_eq!(region.frame_count(), 1);
    }

    // build_region: validation errors

    #[test]
    fn build_region_zero_frame_width() {
        let desc = AtlasRegionDescriptor {
            frame_size: [0, 2],
            frames: vec![AtlasRect {
                x: 0,
                y: 0,
                w: 0,
                h: 2,
            }],
        };
        let err = build_region(0, &desc, UVec2::new(4, 4)).unwrap_err();
        assert!(err.contains("zero frame_size"), "got: {err}");
    }

    #[test]
    fn build_region_zero_frame_height() {
        let desc = AtlasRegionDescriptor {
            frame_size: [2, 0],
            frames: vec![AtlasRect {
                x: 0,
                y: 0,
                w: 2,
                h: 0,
            }],
        };
        let err = build_region(0, &desc, UVec2::new(4, 4)).unwrap_err();
        assert!(err.contains("zero frame_size"), "got: {err}");
    }

    #[test]
    fn build_region_no_frames() {
        let desc = AtlasRegionDescriptor {
            frame_size: [2, 2],
            frames: vec![],
        };
        let err = build_region(0, &desc, UVec2::new(4, 4)).unwrap_err();
        assert!(err.contains("no frames"), "got: {err}");
    }

    #[test]
    fn build_region_frame_zero_size() {
        let desc = AtlasRegionDescriptor {
            frame_size: [2, 2],
            frames: vec![AtlasRect {
                x: 0,
                y: 0,
                w: 0,
                h: 2,
            }],
        };
        let err = build_region(0, &desc, UVec2::new(4, 4)).unwrap_err();
        assert!(err.contains("zero size"), "got: {err}");
    }

    #[test]
    fn build_region_frame_size_mismatch() {
        let desc = AtlasRegionDescriptor {
            frame_size: [2, 2],
            frames: vec![AtlasRect {
                x: 0,
                y: 0,
                w: 3,
                h: 2,
            }],
        };
        let err = build_region(0, &desc, UVec2::new(4, 4)).unwrap_err();
        assert!(err.contains("does not match frame_size"), "got: {err}");
    }

    #[test]
    fn build_region_frame_exceeds_atlas_bounds() {
        let desc = AtlasRegionDescriptor {
            frame_size: [2, 2],
            frames: vec![AtlasRect {
                x: 3,
                y: 0,
                w: 2,
                h: 2,
            }],
        };
        let err = build_region(0, &desc, UVec2::new(4, 4)).unwrap_err();
        assert!(err.contains("exceeds atlas bounds"), "got: {err}");
    }

    #[test]
    fn build_region_frame_overflow() {
        let desc = AtlasRegionDescriptor {
            frame_size: [2, 2],
            frames: vec![AtlasRect {
                x: u32::MAX,
                y: 0,
                w: 2,
                h: 2,
            }],
        };
        let err = build_region(0, &desc, UVec2::new(4, 4)).unwrap_err();
        assert!(err.contains("overflows bounds"), "got: {err}");
    }

    // atlas_region_frame_count helper

    #[test]
    fn atlas_region_frame_count_missing_region_returns_zero() {
        let atlas = PxSpriteAtlasAsset {
            size: UVec2::new(2, 1),
            data: PxImage::new(vec![1, 2], 2),
            regions: vec![],
            names: HashMap::default(),
        };
        let sprite = PxAtlasSprite::new(Handle::default(), AtlasRegionId(0));
        // No assets resource available in unit tests; call the pure inner logic directly.
        // region(id) returns None => frame_count should be 0.
        let count = atlas
            .region(sprite.region)
            .map_or(0, AtlasRegion::frame_count);
        assert_eq!(count, 0);
    }

    #[test]
    fn atlas_region_frame_count_valid_region() {
        let atlas = PxSpriteAtlasAsset {
            size: UVec2::new(6, 1),
            data: PxImage::new(vec![1, 2, 3, 4, 5, 6], 6),
            regions: vec![AtlasRegion {
                frame_size: UVec2::new(2, 1),
                frames: vec![
                    AtlasRect {
                        x: 0,
                        y: 0,
                        w: 2,
                        h: 1,
                    },
                    AtlasRect {
                        x: 2,
                        y: 0,
                        w: 2,
                        h: 1,
                    },
                    AtlasRect {
                        x: 4,
                        y: 0,
                        w: 2,
                        h: 1,
                    },
                ],
            }],
            names: HashMap::default(),
        };
        let count = atlas
            .region(AtlasRegionId(0))
            .map_or(0, AtlasRegion::frame_count);
        assert_eq!(count, 3);
    }

    // Loader name validation (mirrors the names-map check in PxSpriteAtlasLoader::load).
    // The loader calls build_region then validates names; we test the validation logic directly.

    fn validate_names(
        names: &std::collections::BTreeMap<String, u32>,
        region_count: usize,
    ) -> Result<(), String> {
        for (name, &index) in names {
            if index as usize >= region_count {
                return Err(format!(
                    "atlas region name '{name}' points to missing index {index}"
                ));
            }
        }
        Ok(())
    }

    #[test]
    fn loader_name_valid_index_ok() {
        let names = std::collections::BTreeMap::from([("frame".to_string(), 0u32)]);
        assert!(validate_names(&names, 1).is_ok());
    }

    #[test]
    fn loader_name_out_of_bounds_index_errors() {
        let names = std::collections::BTreeMap::from([("frame".to_string(), 5u32)]);
        let err = validate_names(&names, 1).unwrap_err();
        assert!(err.contains("points to missing index 5"), "got: {err}");
    }

    #[test]
    fn loader_name_index_equals_region_count_errors() {
        // index == region_count is also out of bounds (0-based)
        let names = std::collections::BTreeMap::from([("last".to_string(), 2u32)]);
        let err = validate_names(&names, 2).unwrap_err();
        assert!(err.contains("points to missing index 2"), "got: {err}");
    }

    // draw_spatial integration: exercises the atlas → draw_spatial path that draw_layers
    // uses for AtlasSpriteEntry, without requiring GPU infrastructure.

    fn make_atlas_2x2_two_frames() -> PxSpriteAtlasAsset {
        // 4×2 atlas: left half is frame 0 (pixels 1,2,3,4), right half is frame 1 (5,6,7,8).
        // Layout (row-major, top-left origin):
        //   1 2 5 6
        //   3 4 7 8
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
    fn draw_spatial_atlas_frame0_at_origin() {
        let atlas = make_atlas_2x2_two_frames();
        let region = &atlas.regions[0];
        // 4×4 canvas, camera at origin, sprite at (0,0) bottom-left anchored.
        let mut image = PxImage::new(vec![0; 16], 4);
        let mut slice = image.slice_all_mut();

        draw_spatial(
            &(&atlas, region),
            (),
            &mut slice,
            PxPosition(IVec2::new(0, 2)), // bottom-left of a 2×2 sprite at y=2 puts top at row 0
            PxAnchor::BottomLeft,
            PxCanvas::Camera,
            None, // frame 0
            [],
            PxCamera(IVec2::ZERO),
        );

        assert_eq!(
            pixels(&image),
            vec![1, 2, 0, 0, 3, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,]
        );
    }

    #[test]
    fn draw_spatial_atlas_frame1_selected() {
        let atlas = make_atlas_2x2_two_frames();
        let region = &atlas.regions[0];
        let mut image = PxImage::new(vec![0; 16], 4);
        let mut slice = image.slice_all_mut();

        draw_spatial(
            &(&atlas, region),
            (),
            &mut slice,
            PxPosition(IVec2::new(0, 2)),
            PxAnchor::BottomLeft,
            PxCanvas::Camera,
            Some(PxFrameView::from(PxFrameSelector::Index(1.))),
            [],
            PxCamera(IVec2::ZERO),
        );

        assert_eq!(
            pixels(&image),
            vec![5, 6, 0, 0, 7, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,]
        );
    }
}
