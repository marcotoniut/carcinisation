//! Collision detection primitives and optional pixel-mask support.

pub mod shapes;

#[cfg(feature = "pixel-mask")]
pub mod pixel_mask;

pub use shapes::{Collider, ColliderData, ColliderShape};

#[cfg(feature = "pixel-mask")]
pub use pixel_mask::{
    AtlasMaskFrames, AtlasPixelCollisionCache, AtlasPixelData, MaskEdge, PixelCollisionCache,
    PixelMaskSource, SpritePixelData, WorldMaskInstance, WorldMaskRect, atlas_data,
    extract_mask_boundary, extract_mask_boundary_closed, mask_edge_to_world_points, sprite_data,
    world_mask_contains_point, world_mask_overlap, world_mask_rect_from_spatial,
    world_mask_rect_from_top_left,
};
