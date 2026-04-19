#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use bevy::asset::AssetId;
use bevy::prelude::*;
use bevy::reflect::{Reflect, ReflectRef};
use carapace::prelude::*;
use std::{collections::HashMap, sync::Arc};

// Ordered 4x4 dithering threshold map used by PxFrameTransition::Dither.
// Converts fractional frame progress into a stable per-pixel mask for collisions.
const DITHERING: [u16; 16] = [
    0b0000_0000_0000_0000,
    0b1000_0000_0000_0000,
    0b1000_0000_0010_0000,
    0b1010_0000_0010_0000,
    0b1010_0000_1010_0000,
    0b1010_0100_1010_0000,
    0b1010_0100_1010_0001,
    0b1010_0101_1010_0001,
    0b1010_0101_1010_0101,
    0b1110_0101_1010_0101,
    0b1110_0101_1011_0101,
    0b1111_0101_1011_0101,
    0b1111_0101_1111_0101,
    0b1111_1101_1111_0101,
    0b1111_1101_1111_0111,
    0b1111_1111_1111_0111,
];

#[derive(Default)]
pub struct PixelCollisionCache {
    sprites: HashMap<AssetId<PxSpriteAsset>, Arc<SpritePixelData>>,
}

impl PixelCollisionCache {
    pub fn clear(&mut self) {
        self.sprites.clear();
    }
}

#[derive(Default)]
pub struct AtlasPixelCollisionCache {
    atlases: HashMap<AssetId<PxSpriteAtlasAsset>, Arc<AtlasPixelData>>,
}

impl AtlasPixelCollisionCache {
    pub fn clear(&mut self) {
        self.atlases.clear();
    }
}

/// Pixel mask derived from a sprite asset, cached for overlap tests.
#[derive(Debug)]
pub struct SpritePixelData {
    width: u32,
    height: u32,
    frame_count: usize,
    pixels: Vec<u8>,
    segments_per_row: usize,
    // Row-major u64 bitmasks for fast pixel overlap.
    row_masks: Vec<u64>,
}

/// Pixel data extracted from a palette-indexed atlas image.
#[derive(Debug)]
pub struct AtlasPixelData {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

/// Frame source for an atlas-backed pixel mask.
#[derive(Clone, Copy, Debug)]
pub enum AtlasMaskFrames<'a> {
    /// Use the full animated region metadata from a sprite atlas.
    Region(&'a AtlasRegion),
    /// Use a single static atlas rect (e.g. composed fragment binding).
    Single(AtlasRect),
}

/// Generic pixel-mask source independent of gameplay entity type.
#[derive(Clone, Copy, Debug)]
pub enum PixelMaskSource<'a> {
    Sprite(&'a SpritePixelData),
    Atlas {
        atlas: &'a AtlasPixelData,
        frames: AtlasMaskFrames<'a>,
    },
}

/// World-space placement for a pixel mask in gameplay Y-up coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldMaskRect {
    /// Exclusive world rect in gameplay coordinates (Y-up).
    pub rect: IRect,
    /// Horizontal flip applied at render/collision time.
    pub flip_x: bool,
    /// Vertical flip applied at render/collision time.
    pub flip_y: bool,
}

/// Fully resolved mask instance ready for generic point/overlap tests.
#[derive(Clone, Copy)]
pub struct WorldMaskInstance<'a> {
    pub source: PixelMaskSource<'a>,
    pub frame: Option<PxFrameView>,
    pub world: WorldMaskRect,
}

impl SpritePixelData {
    fn from_asset(asset: &PxSpriteAsset) -> Option<Self> {
        // PxSpriteAsset hides pixel buffers; use reflection to build a collision snapshot.
        let ReflectRef::Struct(sprite_struct) = (asset as &dyn Reflect).reflect_ref() else {
            return None;
        };
        let frame_size = sprite_struct
            .field("frame_size")
            .and_then(|value| value.try_downcast_ref::<usize>().copied())?;
        let data = sprite_struct.field("data")?;
        let ReflectRef::Struct(image_struct) = data.reflect_ref() else {
            return None;
        };
        let width = image_struct
            .field("width")
            .and_then(|value| value.try_downcast_ref::<usize>().copied())?;
        let pixels = image_struct
            .field("image")
            .and_then(|value| value.try_downcast_ref::<Vec<u8>>())?;

        if width == 0 || frame_size == 0 || frame_size % width != 0 {
            return None;
        }

        let height = frame_size / width;
        if height == 0 {
            return None;
        }

        let frame_count = pixels.len() / frame_size;
        if frame_count == 0 {
            return None;
        }

        let segments_per_row = width.div_ceil(64);
        let mut row_masks = vec![0u64; frame_count * height * segments_per_row];
        for frame in 0..frame_count {
            for row in 0..height {
                for x in 0..width {
                    let index = (frame * height + row) * width + x;
                    if pixels[index] == 0 {
                        continue;
                    }
                    let segment = x / 64;
                    let bit = x % 64;
                    let offset = (frame * height + row) * segments_per_row + segment;
                    row_masks[offset] |= 1u64 << bit;
                }
            }
        }

        Some(Self {
            width: width as u32,
            height: height as u32,
            frame_count,
            pixels: pixels.clone(),
            segments_per_row,
            row_masks,
        })
    }

    #[must_use]
    pub fn frame_size(&self) -> UVec2 {
        UVec2::new(self.width, self.height)
    }

    fn row_mask(&self, frame: usize, row: u32) -> &[u64] {
        let row = row as usize;
        let offset = (frame * self.height as usize + row) * self.segments_per_row;
        &self.row_masks[offset..offset + self.segments_per_row]
    }
}

impl AtlasPixelData {
    fn from_asset(asset: &PxSpriteAtlasAsset) -> Option<Self> {
        let ReflectRef::Struct(atlas_struct) = (asset as &dyn Reflect).reflect_ref() else {
            return None;
        };
        let data = atlas_struct.field("data")?;
        let ReflectRef::Struct(image_struct) = data.reflect_ref() else {
            return None;
        };
        let width = image_struct
            .field("width")
            .and_then(|value| value.try_downcast_ref::<usize>().copied())?;
        let pixels = image_struct
            .field("image")
            .and_then(|value| value.try_downcast_ref::<Vec<u8>>())?;
        if width == 0 || pixels.is_empty() || pixels.len() % width != 0 {
            return None;
        }

        Some(Self {
            width: width as u32,
            height: (pixels.len() / width) as u32,
            pixels: pixels.clone(),
        })
    }

    fn pixel_visible(&self, atlas_x: u32, atlas_y: u32) -> bool {
        if atlas_x >= self.width || atlas_y >= self.height {
            return false;
        }
        let index = atlas_y as usize * self.width as usize + atlas_x as usize;
        self.pixels.get(index).is_some_and(|pixel| *pixel != 0)
    }
}

impl<'a> AtlasMaskFrames<'a> {
    fn frame_count(self) -> usize {
        match self {
            Self::Region(region) => region.frame_count(),
            Self::Single(_) => 1,
        }
    }

    fn frame_size(self) -> UVec2 {
        match self {
            Self::Region(region) => region.frame_size,
            Self::Single(rect) => UVec2::new(rect.w, rect.h),
        }
    }

    fn frame_rect(self, index: usize) -> Option<AtlasRect> {
        match self {
            Self::Region(region) => region.frame(index),
            Self::Single(rect) => (index == 0).then_some(rect),
        }
    }
}

impl<'a> PixelMaskSource<'a> {
    #[must_use]
    pub fn frame_size(self) -> UVec2 {
        match self {
            Self::Sprite(sprite) => sprite.frame_size(),
            Self::Atlas { frames, .. } => frames.frame_size(),
        }
    }

    #[must_use]
    pub fn frame_count(self) -> usize {
        match self {
            Self::Sprite(sprite) => sprite.frame_count,
            Self::Atlas { frames, .. } => frames.frame_count(),
        }
    }
}

/// Resolve the gameplay/world rect for a rendered mask source from the same
/// spatial inputs used by the sprite render path.
///
/// Returns `None` when the source has zero area or when a non-zero presentation
/// rotation is present. Current gameplay hittables use scale/flip only; refusing
/// rotation here avoids silently claiming render/collision parity for an
/// unsupported transform mode.
#[must_use]
pub fn world_mask_rect_from_spatial(
    source_size: UVec2,
    position: PxPosition,
    anchor: PxAnchor,
    canvas: PxCanvas,
    camera: IVec2,
    presentation: Option<PxPresentationTransform>,
) -> Option<WorldMaskRect> {
    if source_size.x == 0 || source_size.y == 0 {
        return None;
    }

    let (scale, offset, rotation) = presentation.map_or((Vec2::ONE, Vec2::ZERO, 0.0), |value| {
        (value.scale, value.offset, value.rotation)
    });
    if rotation.abs() >= f32::EPSILON {
        return None;
    }

    let display_size = UVec2::new(
        scaled_dimension(source_size.x, scale.x.abs()),
        scaled_dimension(source_size.y, scale.y.abs()),
    );
    let mut position = *position + offset.round().as_ivec2();
    if matches!(canvas, PxCanvas::Camera) {
        position += camera;
    }
    let min = position - anchor_offset(anchor, display_size).as_ivec2();

    Some(WorldMaskRect {
        rect: IRect {
            min,
            max: min.saturating_add(display_size.as_ivec2()),
        },
        flip_x: scale.x.is_sign_negative(),
        flip_y: scale.y.is_sign_negative(),
    })
}

/// Resolve the gameplay/world rect for a mask whose final top-left and display
/// size are already known (e.g. composed fragment state).
#[must_use]
pub fn world_mask_rect_from_top_left(
    top_left_world: Vec2,
    display_size: UVec2,
    flip_x: bool,
    flip_y: bool,
) -> Option<WorldMaskRect> {
    if display_size.x == 0 || display_size.y == 0 {
        return None;
    }

    let top_left = top_left_world.round().as_ivec2();
    Some(WorldMaskRect {
        rect: IRect {
            min: IVec2::new(top_left.x, top_left.y - display_size.y as i32),
            max: IVec2::new(top_left.x + display_size.x as i32, top_left.y),
        },
        flip_x,
        flip_y,
    })
}

/// Returns true when a generic mask instance covers a world-space gameplay
/// point (Y-up coordinates).
#[must_use]
pub fn world_mask_contains_point(mask: WorldMaskInstance<'_>, point: IVec2) -> bool {
    if point.x < mask.world.rect.min.x
        || point.x >= mask.world.rect.max.x
        || point.y < mask.world.rect.min.y
        || point.y >= mask.world.rect.max.y
    {
        return false;
    }

    let display_w = (mask.world.rect.max.x - mask.world.rect.min.x) as u32;
    let display_h = (mask.world.rect.max.y - mask.world.rect.min.y) as u32;
    if display_w == 0 || display_h == 0 {
        return false;
    }

    let source_size = mask.source.frame_size();
    let local_x = (point.x - mask.world.rect.min.x) as u32;
    let local_y = (mask.world.rect.max.y - 1 - point.y) as u32;
    let mapped_x = local_x * source_size.x / display_w;
    let mapped_y = local_y * source_size.y / display_h;
    let source_x = if mask.world.flip_x {
        source_size.x.saturating_sub(1).saturating_sub(mapped_x)
    } else {
        mapped_x
    };
    let source_y = if mask.world.flip_y {
        source_size.y.saturating_sub(1).saturating_sub(mapped_y)
    } else {
        mapped_y
    };

    pixel_source_visible(mask.source, mask.frame, UVec2::new(source_x, source_y))
}

/// Returns the first overlapping world-space point between two generic mask
/// instances.
#[must_use]
pub fn world_mask_overlap(a: WorldMaskInstance<'_>, b: WorldMaskInstance<'_>) -> Option<IVec2> {
    let min = IVec2::new(
        a.world.rect.min.x.max(b.world.rect.min.x),
        a.world.rect.min.y.max(b.world.rect.min.y),
    );
    let max = IVec2::new(
        a.world.rect.max.x.min(b.world.rect.max.x),
        a.world.rect.max.y.min(b.world.rect.max.y),
    );

    if min.x >= max.x || min.y >= max.y {
        return None;
    }

    for y in min.y..max.y {
        for x in min.x..max.x {
            let point = IVec2::new(x, y);
            if world_mask_contains_point(a, point) && world_mask_contains_point(b, point) {
                return Some(point);
            }
        }
    }

    None
}

/// Extract boundary edges from a generic mask source/frame in source-local
/// coordinates (top-left origin).
#[must_use]
pub fn extract_mask_boundary(
    source: PixelMaskSource<'_>,
    frame: Option<PxFrameView>,
) -> Vec<MaskEdge> {
    let size = source.frame_size();
    let mut edges = Vec::new();

    for y in 0..size.y {
        for x in 0..size.x {
            if !pixel_source_visible(source, frame, UVec2::new(x, y)) {
                continue;
            }

            if y == 0 || !pixel_source_visible(source, frame, UVec2::new(x, y - 1)) {
                edges.push(MaskEdge {
                    a: (x, y),
                    b: (x + 1, y),
                });
            }
            if y + 1 >= size.y || !pixel_source_visible(source, frame, UVec2::new(x, y + 1)) {
                edges.push(MaskEdge {
                    a: (x, y + 1),
                    b: (x + 1, y + 1),
                });
            }
            if x == 0 || !pixel_source_visible(source, frame, UVec2::new(x - 1, y)) {
                edges.push(MaskEdge {
                    a: (x, y),
                    b: (x, y + 1),
                });
            }
            if x + 1 >= size.x || !pixel_source_visible(source, frame, UVec2::new(x + 1, y)) {
                edges.push(MaskEdge {
                    a: (x + 1, y),
                    b: (x + 1, y + 1),
                });
            }
        }
    }

    edges
}

/// Maps one source-local boundary edge (top-left origin, Y-down) into world-space
/// gameplay coordinates (Y-up), using the same placement/flip basis as generic
/// mask collision.
#[must_use]
pub fn mask_edge_to_world_points(
    world: WorldMaskRect,
    source_size: UVec2,
    edge: MaskEdge,
) -> (Vec2, Vec2) {
    let display_w = (world.rect.max.x - world.rect.min.x) as f32;
    let display_h = (world.rect.max.y - world.rect.min.y) as f32;
    let source_w = source_size.x.max(1) as f32;
    let source_h = source_size.y.max(1) as f32;

    let point = |x: u32, y: u32| {
        let x_ratio = x as f32 / source_w;
        let y_ratio = y as f32 / source_h;
        let world_x = if world.flip_x {
            world.rect.max.x as f32 - x_ratio * display_w
        } else {
            world.rect.min.x as f32 + x_ratio * display_w
        };
        let world_y = if world.flip_y {
            world.rect.min.y as f32 + y_ratio * display_h
        } else {
            world.rect.max.y as f32 - y_ratio * display_h
        };
        Vec2::new(world_x, world_y)
    };

    (point(edge.a.0, edge.a.1), point(edge.b.0, edge.b.1))
}

#[must_use]
pub fn sprite_data(
    cache: &mut PixelCollisionCache,
    assets: &Assets<PxSpriteAsset>,
    handle: &Handle<PxSpriteAsset>,
) -> Option<Arc<SpritePixelData>> {
    let id = handle.id();
    match cache.sprites.entry(id) {
        std::collections::hash_map::Entry::Occupied(entry) => Some(entry.get().clone()),
        std::collections::hash_map::Entry::Vacant(entry) => {
            let asset = assets.get(handle)?;
            let data = SpritePixelData::from_asset(asset)?;
            let data = Arc::new(data);
            entry.insert(data.clone());
            Some(data)
        }
    }
}

#[must_use]
pub fn atlas_data(
    cache: &mut AtlasPixelCollisionCache,
    assets: &Assets<PxSpriteAtlasAsset>,
    handle: &Handle<PxSpriteAtlasAsset>,
) -> Option<Arc<AtlasPixelData>> {
    let id = handle.id();
    match cache.atlases.entry(id) {
        std::collections::hash_map::Entry::Occupied(entry) => Some(entry.get().clone()),
        std::collections::hash_map::Entry::Vacant(entry) => {
            let asset = assets.get(handle)?;
            let data = AtlasPixelData::from_asset(asset)?;
            let data = Arc::new(data);
            entry.insert(data.clone());
            Some(data)
        }
    }
}

#[must_use]
pub fn sprite_rect(
    size: UVec2,
    position: PxPosition,
    anchor: PxAnchor,
    canvas: PxCanvas,
    camera: IVec2,
) -> IRect {
    let position = *position - anchor_offset(anchor, size).as_ivec2();
    let position = match canvas {
        PxCanvas::World => position - camera,
        PxCanvas::Camera => position,
    };

    IRect {
        min: position,
        max: position.saturating_add(size.as_ivec2()),
    }
}

#[must_use]
pub fn pixel_overlap(
    attack_data: &SpritePixelData,
    attack_frame: Option<PxFrameView>,
    attack_rect: IRect,
    enemy_data: &SpritePixelData,
    enemy_frame: Option<PxFrameView>,
    enemy_rect: IRect,
) -> Option<IVec2> {
    let attack_dither = attack_frame
        .as_ref()
        .is_some_and(|frame| matches!(frame.transition, PxFrameTransition::Dither));
    let enemy_dither = enemy_frame
        .as_ref()
        .is_some_and(|frame| matches!(frame.transition, PxFrameTransition::Dither));
    if !attack_dither
        && !enemy_dither
        && let (Some(attack_index), Some(enemy_index)) = (
            frame_index_for_static(attack_frame, attack_data.frame_count),
            frame_index_for_static(enemy_frame, enemy_data.frame_count),
        )
    {
        return pixel_overlap_fast(
            attack_data,
            attack_index,
            attack_rect,
            enemy_data,
            enemy_index,
            enemy_rect,
        );
    }

    pixel_overlap_slow(
        attack_data,
        attack_frame,
        attack_rect,
        enemy_data,
        enemy_frame,
        enemy_rect,
    )
}

/// Returns true when the sprite's pixel mask covers the given screen-space point.
#[must_use]
pub fn mask_contains_point(
    sprite: &SpritePixelData,
    frame: Option<PxFrameView>,
    sprite_rect: IRect,
    point: IVec2,
) -> bool {
    world_mask_contains_point(
        WorldMaskInstance {
            source: PixelMaskSource::Sprite(sprite),
            frame,
            world: WorldMaskRect {
                rect: sprite_rect,
                flip_x: false,
                flip_y: false,
            },
        },
        point,
    )
}

/// Returns true when an atlas-backed part contains the given world-space point.
///
/// `sprite_rect` uses the resolved part's top-left world position and frame size.
/// The composed runtime uses this for pixel-perfect semantic part selection after
/// coarse collision volumes identify candidate parts.
#[must_use]
/// Tests whether `point` (in screen convention) falls on an opaque pixel
/// within an atlas-backed fragment.
///
/// `sprite_rect` is the fragment's screen-space bounding box (may be scaled
/// by gameplay_scale).  `region_rect` is the unscaled atlas sub-region.
/// When their dimensions differ the function maps screen-local coordinates
/// proportionally into atlas space (nearest-neighbour), so a scaled-down
/// fragment still tests the correct atlas pixel.
pub fn atlas_region_contains_point(
    atlas: &AtlasPixelData,
    region_rect: AtlasRect,
    sprite_rect: IRect,
    point: IVec2,
    flip_x: bool,
    flip_y: bool,
) -> bool {
    world_mask_contains_point(
        WorldMaskInstance {
            source: PixelMaskSource::Atlas {
                atlas,
                frames: AtlasMaskFrames::Single(region_rect),
            },
            frame: None,
            world: WorldMaskRect {
                rect: sprite_rect,
                flip_x,
                flip_y,
            },
        },
        point,
    )
}

/// Returns the first world-space point where an attack sprite mask overlaps
/// visible pixels in one atlas-backed fragment.
///
/// Like [`atlas_region_contains_point`], maps screen-local coordinates
/// proportionally when `region_sprite_rect` is scaled relative to
/// `region_rect`.
#[must_use]
pub fn atlas_region_overlaps_sprite_mask(
    atlas: &AtlasPixelData,
    region_rect: AtlasRect,
    region_sprite_rect: IRect,
    flip: (bool, bool),
    attack: &SpritePixelData,
    attack_frame: Option<PxFrameView>,
    attack_rect: IRect,
) -> Option<IVec2> {
    let (flip_x, flip_y) = flip;
    world_mask_overlap(
        WorldMaskInstance {
            source: PixelMaskSource::Atlas {
                atlas,
                frames: AtlasMaskFrames::Single(region_rect),
            },
            frame: None,
            world: WorldMaskRect {
                rect: region_sprite_rect,
                flip_x,
                flip_y,
            },
        },
        WorldMaskInstance {
            source: PixelMaskSource::Sprite(attack),
            frame: attack_frame,
            world: WorldMaskRect {
                rect: attack_rect,
                flip_x: false,
                flip_y: false,
            },
        },
    )
}

fn pixel_overlap_fast(
    attack_data: &SpritePixelData,
    attack_frame: usize,
    attack_rect: IRect,
    enemy_data: &SpritePixelData,
    enemy_frame: usize,
    enemy_rect: IRect,
) -> Option<IVec2> {
    let min = IVec2::new(
        attack_rect.min.x.max(enemy_rect.min.x),
        attack_rect.min.y.max(enemy_rect.min.y),
    );
    let max = IVec2::new(
        attack_rect.max.x.min(enemy_rect.max.x),
        attack_rect.max.y.min(enemy_rect.max.y),
    );

    if min.x >= max.x || min.y >= max.y {
        return None;
    }

    let delta_x = enemy_rect.min.x - attack_rect.min.x;
    let overlap_min_x = (min.x - attack_rect.min.x) as u32;
    let overlap_max_x = (max.x - attack_rect.min.x) as u32;
    let start_word = (overlap_min_x / 64) as usize;
    let end_word = ((overlap_max_x - 1) / 64) as usize;

    for y in min.y..max.y {
        let attack_local_y = (y - attack_rect.min.y) as u32;
        let enemy_local_y = (y - enemy_rect.min.y) as u32;
        let attack_y = attack_data
            .height
            .saturating_sub(1)
            .saturating_sub(attack_local_y);
        let enemy_y = enemy_data
            .height
            .saturating_sub(1)
            .saturating_sub(enemy_local_y);

        let attack_row = attack_data.row_mask(attack_frame, attack_y);
        let enemy_row = enemy_data.row_mask(enemy_frame, enemy_y);

        for word in start_word..=end_word {
            let mut mask = !0u64;
            if word == start_word {
                let start_bit = overlap_min_x % 64;
                mask &= !0u64 << start_bit;
            }
            if word == end_word {
                let end_bit = overlap_max_x % 64;
                if end_bit != 0 {
                    mask &= (1u64 << end_bit) - 1;
                }
            }

            let attack_word = attack_row.get(word).copied().unwrap_or(0) & mask;
            if attack_word == 0 {
                continue;
            }

            let enemy_word = shifted_row_word(enemy_row, delta_x, word) & mask;
            let overlap = attack_word & enemy_word;
            if overlap != 0 {
                let bit = overlap.trailing_zeros() as i32;
                let screen_x = attack_rect.min.x + (word as i32 * 64) + bit;
                return Some(IVec2::new(screen_x, y));
            }
        }
    }

    None
}

fn shifted_row_word(row: &[u64], shift: i32, word_index: usize) -> u64 {
    if shift == 0 {
        return row.get(word_index).copied().unwrap_or(0);
    }

    if shift > 0 {
        let shift = shift as u32;
        let word_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;
        let Some(src) = word_index.checked_sub(word_shift) else {
            return 0;
        };
        let low = row.get(src).copied().unwrap_or(0);
        if bit_shift == 0 {
            return low;
        }
        let high = if src == 0 {
            0
        } else {
            row.get(src - 1).copied().unwrap_or(0)
        };
        (low << bit_shift) | (high >> (64 - bit_shift))
    } else {
        let shift = (-shift) as u32;
        let word_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;
        let src = word_index + word_shift;
        let low = row.get(src).copied().unwrap_or(0);
        if bit_shift == 0 {
            return low;
        }
        let high = row.get(src + 1).copied().unwrap_or(0);
        (low >> bit_shift) | (high << (64 - bit_shift))
    }
}

fn pixel_overlap_slow(
    attack_data: &SpritePixelData,
    attack_frame: Option<PxFrameView>,
    attack_rect: IRect,
    enemy_data: &SpritePixelData,
    enemy_frame: Option<PxFrameView>,
    enemy_rect: IRect,
) -> Option<IVec2> {
    let min = IVec2::new(
        attack_rect.min.x.max(enemy_rect.min.x),
        attack_rect.min.y.max(enemy_rect.min.y),
    );
    let max = IVec2::new(
        attack_rect.max.x.min(enemy_rect.max.x),
        attack_rect.max.y.min(enemy_rect.max.y),
    );

    if min.x >= max.x || min.y >= max.y {
        return None;
    }

    for y in min.y..max.y {
        let attack_local_y = (y - attack_rect.min.y) as u32;
        let enemy_local_y = (y - enemy_rect.min.y) as u32;
        let attack_y = attack_data
            .height
            .saturating_sub(1)
            .saturating_sub(attack_local_y);
        let enemy_y = enemy_data
            .height
            .saturating_sub(1)
            .saturating_sub(enemy_local_y);

        for x in min.x..max.x {
            let attack_local_x = (x - attack_rect.min.x) as u32;
            let enemy_local_x = (x - enemy_rect.min.x) as u32;
            let attack_pos = UVec2::new(attack_local_x, attack_y);
            let enemy_pos = UVec2::new(enemy_local_x, enemy_y);

            if sprite_pixel_visible(attack_data, attack_frame, attack_pos)
                && sprite_pixel_visible(enemy_data, enemy_frame, enemy_pos)
            {
                return Some(IVec2::new(x, y));
            }
        }
    }

    None
}

fn sprite_pixel_visible(
    sprite: &SpritePixelData,
    frame: Option<PxFrameView>,
    local_pos: UVec2,
) -> bool {
    pixel_source_visible(PixelMaskSource::Sprite(sprite), frame, local_pos)
}

fn frame_index_for_static(frame: Option<PxFrameView>, frame_count: usize) -> Option<usize> {
    if frame_count == 0 {
        return None;
    }

    let Some(frame) = frame else {
        return Some(0);
    };

    let index = match frame.selector {
        PxFrameSelector::Normalized(value) => {
            if frame_count <= 1 {
                0.
            } else {
                value * (frame_count - 1) as f32
            }
        }
        PxFrameSelector::Index(value) => value,
    };

    Some(index.floor() as usize % frame_count)
}

fn frame_index_for_pos(frame: Option<PxFrameView>, frame_count: usize, pos: UVec2) -> usize {
    let Some(frame) = frame else {
        return 0;
    };

    if frame_count == 0 {
        return 0;
    }

    let index = match frame.selector {
        PxFrameSelector::Normalized(value) => {
            if frame_count <= 1 {
                0.
            } else {
                value * (frame_count - 1) as f32
            }
        }
        PxFrameSelector::Index(value) => value,
    };

    let dithering = match frame.transition {
        PxFrameTransition::Dither => DITHERING[(index.fract() * 16.) as usize % 16],
        PxFrameTransition::None => 0,
    };
    let base = index.floor() as usize;
    let bit = 0b1000_0000_0000_0000u16 >> (pos.x % 4 + pos.y % 4 * 4);
    let offset = usize::from(bit & dithering != 0);

    (base + offset) % frame_count
}

fn anchor_offset(anchor: PxAnchor, size: UVec2) -> UVec2 {
    let x = match anchor {
        PxAnchor::BottomLeft | PxAnchor::CenterLeft | PxAnchor::TopLeft => 0,
        PxAnchor::BottomCenter | PxAnchor::Center | PxAnchor::TopCenter => size.x / 2,
        PxAnchor::BottomRight | PxAnchor::CenterRight | PxAnchor::TopRight => size.x,
        PxAnchor::Custom(value) => (size.x as f32 * value.x).round() as u32,
    };
    let y = match anchor {
        PxAnchor::BottomLeft | PxAnchor::BottomCenter | PxAnchor::BottomRight => 0,
        PxAnchor::CenterLeft | PxAnchor::Center | PxAnchor::CenterRight => size.y / 2,
        PxAnchor::TopLeft | PxAnchor::TopCenter | PxAnchor::TopRight => size.y,
        PxAnchor::Custom(value) => (size.y as f32 * value.y).round() as u32,
    };
    UVec2::new(x, y)
}

fn pixel_source_visible(
    source: PixelMaskSource<'_>,
    frame: Option<PxFrameView>,
    local_pos: UVec2,
) -> bool {
    match source {
        PixelMaskSource::Sprite(sprite) => sprite_source_pixel_visible(sprite, frame, local_pos),
        PixelMaskSource::Atlas { atlas, frames } => {
            atlas_source_pixel_visible(atlas, frames, frame, local_pos)
        }
    }
}

fn sprite_source_pixel_visible(
    sprite: &SpritePixelData,
    frame: Option<PxFrameView>,
    local_pos: UVec2,
) -> bool {
    if sprite.width == 0 || sprite.height == 0 {
        return false;
    }
    if local_pos.x >= sprite.width || local_pos.y >= sprite.height {
        return false;
    }

    let frame_count = sprite.frame_count;
    if frame_count == 0 {
        return false;
    }

    let frame_index = frame_index_for_pos(frame, frame_count, local_pos);
    let pixel_y = frame_index as u32 * sprite.height + local_pos.y;
    let index = pixel_y as usize * sprite.width as usize + local_pos.x as usize;
    sprite.pixels.get(index).is_some_and(|pixel| *pixel != 0)
}

fn atlas_source_pixel_visible(
    atlas: &AtlasPixelData,
    frames: AtlasMaskFrames<'_>,
    frame: Option<PxFrameView>,
    local_pos: UVec2,
) -> bool {
    let frame_size = frames.frame_size();
    if frame_size.x == 0 || frame_size.y == 0 {
        return false;
    }
    if local_pos.x >= frame_size.x || local_pos.y >= frame_size.y {
        return false;
    }

    let frame_count = frames.frame_count();
    if frame_count == 0 {
        return false;
    }

    let frame_index = frame_index_for_pos(frame, frame_count, local_pos);
    let Some(rect) = frames.frame_rect(frame_index) else {
        return false;
    };
    atlas.pixel_visible(rect.x + local_pos.x, rect.y + local_pos.y)
}

fn scaled_dimension(size: u32, scale: f32) -> u32 {
    if size == 0 {
        0
    } else {
        ((size as f32 * scale).round() as u32).max(1)
    }
}

// ── Pixel mask boundary extraction ──────────────────────────────────

/// A line segment in mask-local pixel space (top-left origin, Y-down).
///
/// Each segment is one pixel-edge long and lies on a cell boundary.
/// For a pixel at `(x, y)`, the four possible edges are:
/// - top:    `(x, y)   → (x+1, y)`
/// - bottom: `(x, y+1) → (x+1, y+1)`
/// - left:   `(x, y)   → (x, y+1)`
/// - right:  `(x+1, y) → (x+1, y+1)`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaskEdge {
    /// Start of the edge in pixel-grid coordinates.
    pub a: (u32, u32),
    /// End of the edge in pixel-grid coordinates.
    pub b: (u32, u32),
}

/// Extract boundary edges from an atlas region's pixel mask.
///
/// For each opaque pixel in the region, emits edges where an opaque pixel
/// borders a transparent pixel or the region boundary.  The edges are in
/// region-local coordinates (0,0 = top-left of the atlas sub-region).
///
/// This is designed to be called once and cached — not per-frame.
#[must_use]
pub fn extract_atlas_region_boundary(atlas: &AtlasPixelData, region: AtlasRect) -> Vec<MaskEdge> {
    let mut edges = Vec::new();

    for ly in 0..region.h {
        for lx in 0..region.w {
            let ax = region.x + lx;
            let ay = region.y + ly;
            if !atlas.pixel_visible(ax, ay) {
                continue;
            }

            // Top edge: neighbour above is transparent or out of bounds
            if ly == 0 || !atlas.pixel_visible(ax, ay - 1) {
                edges.push(MaskEdge {
                    a: (lx, ly),
                    b: (lx + 1, ly),
                });
            }
            // Bottom edge
            if ly + 1 >= region.h || !atlas.pixel_visible(ax, ay + 1) {
                edges.push(MaskEdge {
                    a: (lx, ly + 1),
                    b: (lx + 1, ly + 1),
                });
            }
            // Left edge
            if lx == 0 || !atlas.pixel_visible(ax - 1, ay) {
                edges.push(MaskEdge {
                    a: (lx, ly),
                    b: (lx, ly + 1),
                });
            }
            // Right edge
            if lx + 1 >= region.w || !atlas.pixel_visible(ax + 1, ay) {
                edges.push(MaskEdge {
                    a: (lx + 1, ly),
                    b: (lx + 1, ly + 1),
                });
            }
        }
    }

    edges
}

/// Cached boundary edges for an atlas region, keyed by sprite ID.
///
/// Built once when the atlas is first loaded, reused every frame for
/// debug gizmo rendering.
#[derive(Clone, Debug, Default)]
pub struct AtlasBoundaryCache {
    entries: Vec<(String, Vec<MaskEdge>)>,
}

impl AtlasBoundaryCache {
    /// Insert boundary edges for a sprite region.
    pub fn insert(&mut self, sprite_id: String, edges: Vec<MaskEdge>) {
        self.entries.push((sprite_id, edges));
    }

    /// Look up cached edges by sprite ID.
    #[must_use]
    pub fn get(&self, sprite_id: &str) -> Option<&[MaskEdge]> {
        self.entries
            .iter()
            .find(|(id, _)| id == sprite_id)
            .map(|(_, edges)| edges.as_slice())
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mask(
        width: u32,
        height: u32,
        frames: usize,
        on: &[(u32, u32, usize)],
    ) -> SpritePixelData {
        let mut pixels = vec![0u8; width as usize * height as usize * frames];
        for (x, y, frame) in on {
            let flipped_y = height.saturating_sub(1).saturating_sub(*y) as usize;
            let index = (frame * height as usize + flipped_y) * width as usize + *x as usize;
            pixels[index] = 1;
        }
        let segments_per_row = width.div_ceil(64) as usize;
        let mut row_masks = vec![0u64; frames * height as usize * segments_per_row];
        for frame in 0..frames {
            for row in 0..height {
                for x in 0..width {
                    let index =
                        (frame * height as usize + row as usize) * width as usize + x as usize;
                    if pixels[index] == 0 {
                        continue;
                    }
                    let segment = (x / 64) as usize;
                    let bit = x % 64;
                    let offset =
                        (frame * height as usize + row as usize) * segments_per_row + segment;
                    row_masks[offset] |= 1u64 << bit;
                }
            }
        }
        SpritePixelData {
            width,
            height,
            frame_count: frames,
            pixels,
            segments_per_row,
            row_masks,
        }
    }

    fn rect_for_mask(min: IVec2, mask: &SpritePixelData) -> IRect {
        IRect {
            min,
            max: min + mask.frame_size().as_ivec2(),
        }
    }

    fn make_atlas(width: u32, height: u32, on: &[(u32, u32)]) -> AtlasPixelData {
        let mut pixels = vec![0u8; width as usize * height as usize];
        for (x, y) in on {
            let index = *y as usize * width as usize + *x as usize;
            pixels[index] = 1;
        }
        AtlasPixelData {
            width,
            height,
            pixels,
        }
    }

    fn mask_overlaps_box(
        mask: &SpritePixelData,
        frame: Option<PxFrameView>,
        mask_rect: IRect,
        box_center: Vec2,
        half: Vec2,
    ) -> bool {
        let box_min = box_center - half;
        let box_max = box_center + half;
        let min = IVec2::new(
            mask_rect.min.x.max(box_min.x.floor() as i32),
            mask_rect.min.y.max(box_min.y.floor() as i32),
        );
        let max = IVec2::new(
            mask_rect.max.x.min(box_max.x.ceil() as i32),
            mask_rect.max.y.min(box_max.y.ceil() as i32),
        );

        for y in min.y..max.y {
            let local_y = (y - mask_rect.min.y) as u32;
            let sprite_y = mask.height.saturating_sub(1).saturating_sub(local_y);
            for x in min.x..max.x {
                let local_x = (x - mask_rect.min.x) as u32;
                let local = UVec2::new(local_x, sprite_y);
                if !sprite_pixel_visible(mask, frame, local) {
                    continue;
                }

                let point = Vec2::new(x as f32, y as f32);
                let delta = (point - box_center).abs();
                if delta.x <= half.x && delta.y <= half.y {
                    return true;
                }
            }
        }

        false
    }

    fn mask_overlaps_circle(
        mask: &SpritePixelData,
        frame: Option<PxFrameView>,
        mask_rect: IRect,
        center: Vec2,
        radius: f32,
    ) -> bool {
        let min = IVec2::new(
            mask_rect.min.x.max((center.x - radius).floor() as i32),
            mask_rect.min.y.max((center.y - radius).floor() as i32),
        );
        let max = IVec2::new(
            mask_rect.max.x.min((center.x + radius).ceil() as i32),
            mask_rect.max.y.min((center.y + radius).ceil() as i32),
        );
        let radius_sq = radius * radius;

        for y in min.y..max.y {
            let local_y = (y - mask_rect.min.y) as u32;
            let sprite_y = mask.height.saturating_sub(1).saturating_sub(local_y);
            for x in min.x..max.x {
                let local_x = (x - mask_rect.min.x) as u32;
                let local = UVec2::new(local_x, sprite_y);
                if !sprite_pixel_visible(mask, frame, local) {
                    continue;
                }

                let point = Vec2::new(x as f32, y as f32);
                if point.distance_squared(center) <= radius_sq {
                    return true;
                }
            }
        }

        false
    }

    #[test]
    fn pixel_mask_overlaps_pixel_mask() {
        let attack = make_mask(3, 3, 1, &[(2, 1, 0)]);
        let enemy = make_mask(3, 3, 1, &[(0, 1, 0)]);
        let attack_rect = rect_for_mask(IVec2::new(0, 0), &attack);
        let enemy_rect = rect_for_mask(IVec2::new(2, 0), &enemy);

        let hit = pixel_overlap(&attack, None, attack_rect, &enemy, None, enemy_rect);
        assert_eq!(hit, Some(IVec2::new(2, 1)));
    }

    #[test]
    fn pixel_mask_does_not_overlap_pixel_mask() {
        let attack = make_mask(2, 2, 1, &[(0, 0, 0)]);
        let enemy = make_mask(2, 2, 1, &[(1, 1, 0)]);
        let attack_rect = rect_for_mask(IVec2::new(0, 0), &attack);
        let enemy_rect = rect_for_mask(IVec2::new(3, 0), &enemy);

        let hit = pixel_overlap(&attack, None, attack_rect, &enemy, None, enemy_rect);
        assert!(hit.is_none());
    }

    #[test]
    fn pixel_mask_overlaps_pixel_mask_wide() {
        let attack = make_mask(70, 1, 1, &[(65, 0, 0)]);
        let enemy = make_mask(70, 1, 1, &[(0, 0, 0)]);
        let attack_rect = rect_for_mask(IVec2::new(0, 0), &attack);
        let enemy_rect = rect_for_mask(IVec2::new(65, 0), &enemy);

        let hit = pixel_overlap(&attack, None, attack_rect, &enemy, None, enemy_rect);
        assert_eq!(hit, Some(IVec2::new(65, 0)));
    }

    #[test]
    fn pixel_mask_contains_point() {
        let mask = make_mask(4, 4, 1, &[(2, 1, 0)]);
        let rect = rect_for_mask(IVec2::new(10, 5), &mask);

        assert!(mask_contains_point(&mask, None, rect, IVec2::new(12, 6)));
        assert!(!mask_contains_point(&mask, None, rect, IVec2::new(11, 6)));
    }

    #[test]
    fn atlas_region_contains_point_respects_flip_x() {
        let atlas = make_atlas(2, 1, &[(0, 0)]);
        let region = AtlasRect {
            x: 0,
            y: 0,
            w: 2,
            h: 1,
        };
        let sprite_rect = IRect {
            min: IVec2::new(10, 5),
            max: IVec2::new(12, 6),
        };

        assert!(atlas_region_contains_point(
            &atlas,
            region,
            sprite_rect,
            IVec2::new(10, 5),
            false,
            false
        ));
        assert!(!atlas_region_contains_point(
            &atlas,
            region,
            sprite_rect,
            IVec2::new(11, 5),
            false,
            false
        ));
        assert!(atlas_region_contains_point(
            &atlas,
            region,
            sprite_rect,
            IVec2::new(11, 5),
            true,
            false
        ));
    }

    #[test]
    fn atlas_region_contains_point_proportional_mapping_for_scaled_rect() {
        // Atlas region: 4x2, opaque at (0,0) and (3,1)
        let atlas = make_atlas(4, 2, &[(0, 0), (3, 1)]);
        let region = AtlasRect {
            x: 0,
            y: 0,
            w: 4,
            h: 2,
        };
        // Display rect is HALF size: 2x1 (simulating scale 0.5)
        let sprite_rect = IRect {
            min: IVec2::new(10, 20),
            max: IVec2::new(12, 21),
        };

        // Screen point (10, 20) → local (0, 0) → atlas (0*4/2, 0*2/1) = (0, 0) → opaque
        assert!(atlas_region_contains_point(
            &atlas,
            region,
            sprite_rect,
            IVec2::new(10, 20),
            false,
            false
        ));

        // Screen point (11, 20) → local (1, 0) → atlas (1*4/2, 0) = (2, 0) → transparent
        assert!(!atlas_region_contains_point(
            &atlas,
            region,
            sprite_rect,
            IVec2::new(11, 20),
            false,
            false
        ));

        // Point outside scaled rect → miss
        assert!(!atlas_region_contains_point(
            &atlas,
            region,
            sprite_rect,
            IVec2::new(12, 20),
            false,
            false
        ));
    }

    #[test]
    fn atlas_region_contains_point_identity_when_unscaled() {
        // When display size equals region size, mapping is 1:1 (same as before)
        let atlas = make_atlas(3, 3, &[(1, 1)]);
        let region = AtlasRect {
            x: 0,
            y: 0,
            w: 3,
            h: 3,
        };
        let sprite_rect = IRect {
            min: IVec2::new(10, 20),
            max: IVec2::new(13, 23),
        };

        // (11, 21) → local (1, 1) → atlas (1, 1) → opaque
        assert!(atlas_region_contains_point(
            &atlas,
            region,
            sprite_rect,
            IVec2::new(11, 21),
            false,
            false
        ));
        // (10, 20) → local (0, 0) → atlas (0, 0) → transparent
        assert!(!atlas_region_contains_point(
            &atlas,
            region,
            sprite_rect,
            IVec2::new(10, 20),
            false,
            false
        ));
    }

    #[test]
    fn atlas_region_contains_point_respects_flip_y() {
        let atlas = make_atlas(1, 2, &[(0, 0)]);
        let region = AtlasRect {
            x: 0,
            y: 0,
            w: 1,
            h: 2,
        };
        let sprite_rect = IRect {
            min: IVec2::new(4, 8),
            max: IVec2::new(5, 10),
        };

        assert!(!atlas_region_contains_point(
            &atlas,
            region,
            sprite_rect,
            IVec2::new(4, 8),
            false,
            false
        ));
        assert!(atlas_region_contains_point(
            &atlas,
            region,
            sprite_rect,
            IVec2::new(4, 9),
            false,
            false
        ));
        assert!(atlas_region_contains_point(
            &atlas,
            region,
            sprite_rect,
            IVec2::new(4, 8),
            false,
            true
        ));
    }

    #[test]
    fn atlas_region_overlaps_sprite_mask_returns_hit_point() {
        let atlas = make_atlas(3, 3, &[(1, 1)]);
        let region = AtlasRect {
            x: 0,
            y: 0,
            w: 3,
            h: 3,
        };
        let region_rect = IRect {
            min: IVec2::new(10, 20),
            max: IVec2::new(13, 23),
        };
        let attack = make_mask(3, 3, 1, &[(0, 0, 0)]);
        let attack_rect = IRect {
            min: IVec2::new(11, 21),
            max: IVec2::new(14, 24),
        };

        assert_eq!(
            atlas_region_overlaps_sprite_mask(
                &atlas,
                region,
                region_rect,
                (false, false),
                &attack,
                None,
                attack_rect,
            ),
            Some(IVec2::new(11, 21))
        );
    }

    #[test]
    fn atlas_region_overlaps_sprite_mask_respects_flip_x() {
        let atlas = make_atlas(2, 1, &[(0, 0)]);
        let region = AtlasRect {
            x: 0,
            y: 0,
            w: 2,
            h: 1,
        };
        let region_rect = IRect {
            min: IVec2::new(10, 5),
            max: IVec2::new(12, 6),
        };
        let attack = make_mask(2, 1, 1, &[(1, 0, 0)]);
        let attack_rect = region_rect;

        assert_eq!(
            atlas_region_overlaps_sprite_mask(
                &atlas,
                region,
                region_rect,
                (true, false),
                &attack,
                None,
                attack_rect,
            ),
            Some(IVec2::new(11, 5))
        );
    }

    #[test]
    fn box_overlaps_pixel_mask() {
        let mask = make_mask(3, 3, 1, &[(1, 1, 0)]);
        let rect = rect_for_mask(IVec2::new(0, 0), &mask);

        assert!(mask_overlaps_box(
            &mask,
            None,
            rect,
            Vec2::new(1.0, 1.0),
            Vec2::new(0.6, 0.6)
        ));
        assert!(!mask_overlaps_box(
            &mask,
            None,
            rect,
            Vec2::new(4.0, 4.0),
            Vec2::new(0.6, 0.6)
        ));
    }

    #[test]
    fn circle_overlaps_pixel_mask() {
        let mask = make_mask(3, 3, 1, &[(0, 0, 0)]);
        let rect = rect_for_mask(IVec2::new(0, 0), &mask);

        assert!(mask_overlaps_circle(
            &mask,
            None,
            rect,
            Vec2::new(0.0, 0.0),
            0.5
        ));
        assert!(!mask_overlaps_circle(
            &mask,
            None,
            rect,
            Vec2::new(2.0, 2.0),
            0.5
        ));
    }

    // ── P-shooter 5-point cross behavioural tests ──────────────────────
    //
    // These prove the multi-point hit pattern works end-to-end through the
    // same `mask_contains_point` function used by real hit detection.
    // The test sprite has a single opaque pixel; the cross pattern tests
    // whether each offset reaches or misses it.

    /// Helper: run the 5-point cross hit pattern against a sprite.
    /// Returns true if ANY offset produces a hit.
    fn cross_pattern_hits(
        sprite: &SpritePixelData,
        sprite_rect: IRect,
        attack_screen: IVec2,
    ) -> bool {
        let offsets = [IVec2::ZERO, IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y];
        offsets
            .iter()
            .any(|&offset| mask_contains_point(sprite, None, sprite_rect, attack_screen + offset))
    }

    #[test]
    fn cross_pattern_centre_hit() {
        // Opaque pixel at local (2, 2). Attack directly on it.
        let sprite = make_mask(5, 5, 1, &[(2, 2, 0)]);
        let rect = rect_for_mask(IVec2::new(10, 10), &sprite);
        let attack = IVec2::new(12, 12); // local (2, 2)
        assert!(cross_pattern_hits(&sprite, rect, attack));
    }

    #[test]
    fn cross_pattern_cardinal_right_hit() {
        // Opaque pixel at (2, 2). Attack at (1, 2) — the +X offset reaches (2, 2).
        let sprite = make_mask(5, 5, 1, &[(2, 2, 0)]);
        let rect = rect_for_mask(IVec2::new(10, 10), &sprite);
        let attack = IVec2::new(11, 12); // local (1, 2)
        assert!(
            cross_pattern_hits(&sprite, rect, attack),
            "right offset should reach opaque pixel"
        );
    }

    #[test]
    fn cross_pattern_cardinal_left_hit() {
        let sprite = make_mask(5, 5, 1, &[(2, 2, 0)]);
        let rect = rect_for_mask(IVec2::new(10, 10), &sprite);
        let attack = IVec2::new(13, 12); // local (3, 2) — NEG_X reaches (2, 2)
        assert!(
            cross_pattern_hits(&sprite, rect, attack),
            "left offset should reach opaque pixel"
        );
    }

    #[test]
    fn cross_pattern_cardinal_up_hit() {
        let sprite = make_mask(5, 5, 1, &[(2, 2, 0)]);
        let rect = rect_for_mask(IVec2::new(10, 10), &sprite);
        let attack = IVec2::new(12, 11); // local (2, 1) — +Y reaches (2, 2)
        assert!(
            cross_pattern_hits(&sprite, rect, attack),
            "up offset should reach opaque pixel"
        );
    }

    #[test]
    fn cross_pattern_cardinal_down_hit() {
        let sprite = make_mask(5, 5, 1, &[(2, 2, 0)]);
        let rect = rect_for_mask(IVec2::new(10, 10), &sprite);
        let attack = IVec2::new(12, 13); // local (2, 3) — NEG_Y reaches (2, 2)
        assert!(
            cross_pattern_hits(&sprite, rect, attack),
            "down offset should reach opaque pixel"
        );
    }

    #[test]
    fn cross_pattern_diagonal_miss() {
        // Opaque pixel at (2, 2). Attack at (1, 1) — diagonal.
        // No cross offset reaches (2, 2) from (1, 1).
        let sprite = make_mask(5, 5, 1, &[(2, 2, 0)]);
        let rect = rect_for_mask(IVec2::new(10, 10), &sprite);
        let attack = IVec2::new(11, 11); // local (1, 1)
        assert!(
            !cross_pattern_hits(&sprite, rect, attack),
            "diagonal should not hit — no offset reaches (2,2) from (1,1)"
        );
    }

    #[test]
    fn cross_pattern_far_miss() {
        // Opaque pixel at (2, 2). Attack at (0, 0) — too far.
        let sprite = make_mask(5, 5, 1, &[(2, 2, 0)]);
        let rect = rect_for_mask(IVec2::new(10, 10), &sprite);
        let attack = IVec2::new(10, 10); // local (0, 0)
        assert!(
            !cross_pattern_hits(&sprite, rect, attack),
            "far point should miss entirely"
        );
    }

    #[test]
    fn cross_pattern_outside_sprite_miss() {
        let sprite = make_mask(5, 5, 1, &[(2, 2, 0)]);
        let rect = rect_for_mask(IVec2::new(10, 10), &sprite);
        let attack = IVec2::new(20, 20); // completely outside sprite rect
        assert!(
            !cross_pattern_hits(&sprite, rect, attack),
            "point outside sprite should miss"
        );
    }

    #[test]
    fn world_mask_rect_from_spatial_keeps_world_canvas_in_world_space() {
        let rect = world_mask_rect_from_spatial(
            UVec2::new(4, 2),
            PxPosition(IVec2::new(20, 30)),
            PxAnchor::BottomLeft,
            PxCanvas::World,
            IVec2::new(7, 9),
            None,
        )
        .unwrap();

        assert_eq!(
            rect,
            WorldMaskRect {
                rect: IRect {
                    min: IVec2::new(20, 30),
                    max: IVec2::new(24, 32),
                },
                flip_x: false,
                flip_y: false,
            }
        );
    }

    #[test]
    fn world_mask_rect_from_spatial_offsets_camera_canvas_by_camera() {
        let rect = world_mask_rect_from_spatial(
            UVec2::new(4, 2),
            PxPosition(IVec2::new(20, 30)),
            PxAnchor::BottomLeft,
            PxCanvas::Camera,
            IVec2::new(7, 9),
            None,
        )
        .unwrap();

        assert_eq!(
            rect.rect,
            IRect {
                min: IVec2::new(27, 39),
                max: IVec2::new(31, 41),
            }
        );
    }

    #[test]
    fn world_mask_rect_from_spatial_matches_scaled_anchor_and_flip() {
        let rect = world_mask_rect_from_spatial(
            UVec2::new(4, 2),
            PxPosition(IVec2::new(100, 50)),
            PxAnchor::Center,
            PxCanvas::World,
            IVec2::new(12, 34),
            Some(PxPresentationTransform {
                scale: Vec2::new(-1.5, 2.0),
                rotation: 0.0,
                offset: Vec2::new(1.2, -0.7),
            }),
        )
        .unwrap();

        assert_eq!(
            rect,
            WorldMaskRect {
                rect: IRect {
                    min: IVec2::new(98, 47),
                    max: IVec2::new(104, 51),
                },
                flip_x: true,
                flip_y: false,
            }
        );
    }

    #[test]
    fn atlas_mask_point_collision_respects_scale_and_flip() {
        let atlas = make_atlas(2, 2, &[(0, 0)]);
        let mask = WorldMaskInstance {
            source: PixelMaskSource::Atlas {
                atlas: &atlas,
                frames: AtlasMaskFrames::Single(region_full(2, 2)),
            },
            frame: None,
            world: WorldMaskRect {
                rect: IRect {
                    min: IVec2::new(10, 20),
                    max: IVec2::new(14, 24),
                },
                flip_x: true,
                flip_y: false,
            },
        };

        assert!(world_mask_contains_point(mask, IVec2::new(13, 23)));
        assert!(!world_mask_contains_point(mask, IVec2::new(10, 23)));
    }

    #[test]
    fn atlas_mask_overlaps_sprite_mask_in_world_space() {
        let atlas = make_atlas(2, 2, &[(0, 0)]);
        let attack = make_mask(2, 2, 1, &[(1, 1, 0)]);
        let atlas_mask = WorldMaskInstance {
            source: PixelMaskSource::Atlas {
                atlas: &atlas,
                frames: AtlasMaskFrames::Single(region_full(2, 2)),
            },
            frame: None,
            world: WorldMaskRect {
                rect: IRect {
                    min: IVec2::new(50, 10),
                    max: IVec2::new(54, 14),
                },
                flip_x: true,
                flip_y: false,
            },
        };
        let attack_mask = WorldMaskInstance {
            source: PixelMaskSource::Sprite(&attack),
            frame: None,
            world: WorldMaskRect {
                rect: IRect {
                    min: IVec2::new(50, 10),
                    max: IVec2::new(54, 14),
                },
                flip_x: false,
                flip_y: false,
            },
        };

        assert_eq!(
            world_mask_overlap(atlas_mask, attack_mask),
            Some(IVec2::new(52, 12))
        );
    }

    #[test]
    fn mask_edge_to_world_points_tracks_mask_geometry() {
        let (a, b) = mask_edge_to_world_points(
            WorldMaskRect {
                rect: IRect {
                    min: IVec2::new(10, 20),
                    max: IVec2::new(14, 24),
                },
                flip_x: false,
                flip_y: false,
            },
            UVec2::new(2, 2),
            MaskEdge {
                a: (0, 0),
                b: (2, 0),
            },
        );

        assert_eq!(a, Vec2::new(10.0, 24.0));
        assert_eq!(b, Vec2::new(14.0, 24.0));
    }

    #[test]
    fn single_point_centre_only_misses_neighbour() {
        // Prove that without offsets, a 1px miss is a miss.
        let sprite = make_mask(5, 5, 1, &[(2, 2, 0)]);
        let rect = rect_for_mask(IVec2::new(10, 10), &sprite);
        let attack = IVec2::new(11, 12); // local (1, 2) — 1px left of opaque
        assert!(
            !mask_contains_point(&sprite, None, rect, attack),
            "single-point check at (1,2) should miss opaque at (2,2)"
        );
        // But the cross pattern hits:
        assert!(
            cross_pattern_hits(&sprite, rect, attack),
            "cross pattern should hit via +X offset"
        );
    }

    // ── Boundary extraction tests ──────────────────────────────────

    fn region_full(w: u32, h: u32) -> AtlasRect {
        AtlasRect { x: 0, y: 0, w, h }
    }

    #[test]
    fn boundary_single_pixel_has_4_edges() {
        let atlas = make_atlas(1, 1, &[(0, 0)]);
        let edges = extract_atlas_region_boundary(&atlas, region_full(1, 1));
        assert_eq!(edges.len(), 4);
        // All four sides of the single pixel
        assert!(edges.contains(&MaskEdge {
            a: (0, 0),
            b: (1, 0)
        })); // top
        assert!(edges.contains(&MaskEdge {
            a: (0, 1),
            b: (1, 1)
        })); // bottom
        assert!(edges.contains(&MaskEdge {
            a: (0, 0),
            b: (0, 1)
        })); // left
        assert!(edges.contains(&MaskEdge {
            a: (1, 0),
            b: (1, 1)
        })); // right
    }

    #[test]
    fn boundary_two_horizontal_pixels_share_interior_edge() {
        let atlas = make_atlas(2, 1, &[(0, 0), (1, 0)]);
        let edges = extract_atlas_region_boundary(&atlas, region_full(2, 1));
        // 2 pixels side by side: 6 edges (not 8), shared interior edge removed
        // Top: 2, Bottom: 2, Left: 1, Right: 1 = 6
        assert_eq!(edges.len(), 6);
        // Interior vertical edges should NOT be present
        assert!(!edges.contains(&MaskEdge {
            a: (1, 0),
            b: (1, 1)
        }));
    }

    #[test]
    fn boundary_l_shape_has_correct_edges() {
        // L-shape: (0,0), (0,1), (1,1) — concave corner
        let atlas = make_atlas(2, 2, &[(0, 0), (0, 1), (1, 1)]);
        let edges = extract_atlas_region_boundary(&atlas, region_full(2, 2));
        // (0,0): 3 boundary edges (top, left, right — bottom shared with (0,1))
        // (0,1): 2 boundary edges (bottom, left — top shared, right shared)
        // (1,1): 3 boundary edges (top, bottom, right — left shared with (0,1))
        assert_eq!(edges.len(), 8);
    }

    #[test]
    fn boundary_empty_mask_has_no_edges() {
        let atlas = make_atlas(3, 3, &[]);
        let edges = extract_atlas_region_boundary(&atlas, region_full(3, 3));
        assert!(edges.is_empty());
    }

    #[test]
    fn boundary_transparent_padding_ignored() {
        // Opaque pixel at (1,1) surrounded by transparent padding
        let atlas = make_atlas(3, 3, &[(1, 1)]);
        let edges = extract_atlas_region_boundary(&atlas, region_full(3, 3));
        // Single isolated pixel → 4 edges, all at local (1,1)
        assert_eq!(edges.len(), 4);
        assert!(edges.contains(&MaskEdge {
            a: (1, 1),
            b: (2, 1)
        })); // top
        assert!(edges.contains(&MaskEdge {
            a: (1, 2),
            b: (2, 2)
        })); // bottom
        assert!(edges.contains(&MaskEdge {
            a: (1, 1),
            b: (1, 2)
        })); // left
        assert!(edges.contains(&MaskEdge {
            a: (2, 1),
            b: (2, 2)
        })); // right
    }

    #[test]
    fn boundary_cache_lookup() {
        let mut cache = AtlasBoundaryCache::default();
        cache.insert(
            "body_0".into(),
            vec![MaskEdge {
                a: (0, 0),
                b: (1, 0),
            }],
        );
        assert!(cache.get("body_0").is_some());
        assert_eq!(cache.get("body_0").unwrap().len(), 1);
        assert!(cache.get("missing").is_none());
    }
}
