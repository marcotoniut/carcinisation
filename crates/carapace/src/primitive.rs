//! General-purpose palette-indexed primitive drawing.
//!
//! [`CxPrimitive`] is a simple filled shape that renders directly into the CPU
//! pixel buffer, alongside sprites, text, and other carapace drawables.
//!
//! Unlike [`CxFilterRect`](crate::rect::CxFilterRect) (which applies a palette
//! filter to an existing region), primitives **produce** pixels: solid fills,
//! checkerboard patterns, and ordered-dither blends.
//!
//! Primitives participate in the normal layer pipeline. Within a given layer
//! they draw **before** sprites, so sprites always paint on top.

#[cfg(feature = "headed")]
use bevy_render::{
    Extract, RenderApp,
    sync_world::{RenderEntity, SyncToRenderWorld},
};

use crate::{image::CxImageSliceMut, prelude::*};

pub(crate) fn plug<L: CxLayer>(app: &mut App) {
    #[cfg(feature = "headed")]
    {
        // CxPrimitive uses manual extraction (not SyncComponentPlugin) because
        // it needs custom clone/insert logic matching the sprite extraction
        // pattern. SyncToRenderWorld is registered as a required component so
        // that Bevy creates render-world counterparts (RenderEntity) for
        // primitive entities.
        app.register_required_components::<CxPrimitive, SyncToRenderWorld>();
        app.sub_app_mut(RenderApp)
            .add_systems(ExtractSchedule, extract_primitives::<L>);
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// A filled primitive shape drawn to the pixel buffer at its entity's position
/// and layer.
///
/// Requires the standard carapace spatial components: [`CxPosition`],
/// [`CxAnchor`], [`CxRenderSpace`], and a layer.
///
/// Unlike sprites, primitives do not support [`CxFilter`](crate::filter::CxFilter)
/// (palette remapping) or [`CxPresentationTransform`] scale/rotation — only
/// `visual_offset` is applied. Primitives produce palette-indexed pixels
/// directly from their [`CxPrimitiveFill`].
#[derive(Component, Clone, Debug)]
#[require(CxPosition, CxAnchor, CxRenderSpace)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct CxPrimitive {
    /// Geometry of the shape to draw.
    pub shape: CxPrimitiveShape,
    /// How the shape is filled (solid, pattern, dither).
    pub fill: CxPrimitiveFill,
}

/// Shape geometry for a [`CxPrimitive`].
#[derive(Clone, Debug, PartialEq)]
pub enum CxPrimitiveShape {
    /// Axis-aligned filled rectangle. `size` is in logical pixels.
    Rect {
        /// Width and height in logical pixels.
        size: UVec2,
    },
    /// Filled circle with the given radius in logical pixels.
    Circle {
        /// Radius in logical pixels. Diameter = `2 * radius + 1`.
        radius: u32,
    },
    /// Filled convex polygon from a vertex list (relative to position).
    ///
    /// Rasterisation is deferred — the variant is accepted but currently
    /// produces no pixels. See module-level docs for status.
    Polygon {
        /// Vertex positions relative to the entity's anchor.
        vertices: Vec<IVec2>,
    },
}

/// Fill mode for a [`CxPrimitive`].
#[derive(Clone, Debug, PartialEq)]
pub enum CxPrimitiveFill {
    /// Every pixel is the same palette index.
    Solid(u8),
    /// Alternating palette indices on a 2x2 grid, evaluated in world-pixel
    /// coordinates so the pattern stays locked to the game world.
    Checker {
        /// Palette index for even-parity pixels.
        a: u8,
        /// Palette index for odd-parity pixels.
        b: u8,
    },
    /// 4x4 ordered (Bayer) dither between two palette indices.
    ///
    /// `threshold` is in the range `0..=16`:
    /// - `0`  → all pixels are `a`
    /// - `16` → all pixels are `b`
    /// - intermediate values blend via the 4x4 Bayer matrix
    ///
    /// Pattern is evaluated in world-pixel coordinates (stable under camera
    /// movement).
    OrderedDither {
        /// Palette index below the dither threshold.
        a: u8,
        /// Palette index at or above the dither threshold.
        b: u8,
        /// Coverage level, `0..=16`. 0 = all `a`, 16 = all `b`.
        threshold: u8,
    },
}

// ---------------------------------------------------------------------------
// 4x4 Bayer threshold matrix
// ---------------------------------------------------------------------------

/// Classic 4x4 Bayer ordered-dither threshold matrix, values 0..16.
///
/// Entry `BAYER_4X4[y % 4][x % 4]` gives the threshold for that pixel.
/// A pixel selects palette index `b` when `threshold > matrix_value`,
/// otherwise `a`.
const BAYER_4X4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

// ---------------------------------------------------------------------------
// Fill evaluation
// ---------------------------------------------------------------------------

/// Evaluate the fill at a world-pixel coordinate, returning a palette index.
///
/// Returns `0` (transparent) only if the fill itself produces index 0.
#[must_use]
fn evaluate_fill(fill: &CxPrimitiveFill, world_x: i32, world_y: i32) -> u8 {
    match *fill {
        CxPrimitiveFill::Solid(idx) => idx,
        CxPrimitiveFill::Checker { a, b } => {
            if (world_x + world_y) & 1 == 0 {
                a
            } else {
                b
            }
        }
        CxPrimitiveFill::OrderedDither { a, b, threshold } => {
            let tx = (world_x.rem_euclid(4)) as usize;
            let ty = (world_y.rem_euclid(4)) as usize;
            if threshold > BAYER_4X4[ty][tx] { b } else { a }
        }
    }
}

// ---------------------------------------------------------------------------
// Rasterisation
// ---------------------------------------------------------------------------

/// Draw a primitive into the given image slice.
///
/// `world_origin` is the world-pixel coordinate of the primitive's top-left
/// corner (after anchor + camera adjustment). It is used for stable pattern
/// evaluation.
pub(crate) fn draw_primitive(
    primitive: &CxPrimitive,
    image: &mut CxImageSliceMut,
    world_origin: IVec2,
) {
    match &primitive.shape {
        CxPrimitiveShape::Rect { size } => {
            draw_filled_rect(image, *size, &primitive.fill, world_origin);
        }
        CxPrimitiveShape::Circle { radius } => {
            draw_filled_circle(image, *radius, &primitive.fill, world_origin);
        }
        CxPrimitiveShape::Polygon { .. } => {
            // Polygon rasterisation is deferred. The shape variant is accepted
            // so consumers can author polygon primitives now, but no pixels are
            // produced until a scanline fill is implemented.
        }
    }
}

fn draw_filled_rect(
    image: &mut CxImageSliceMut,
    size: UVec2,
    fill: &CxPrimitiveFill,
    world_origin: IVec2,
) {
    let img_w = image.width as i32;
    let img_h = image.image.len() as i32;
    let offset = image.offset();

    let x_min = offset.x.max(0);
    let y_min = offset.y.max(0);
    let x_max = (offset.x + size.x as i32).min(img_w);
    let y_max = (offset.y + size.y as i32).min(img_h);

    for y in y_min..y_max {
        for x in x_min..x_max {
            let wx = world_origin.x + (x - offset.x);
            let wy = world_origin.y + (y - offset.y);
            let idx = evaluate_fill(fill, wx, wy);
            if idx != 0 {
                *image.image_pixel_mut(IVec2::new(x, y)) = idx;
            }
        }
    }
}

fn draw_filled_circle(
    image: &mut CxImageSliceMut,
    radius: u32,
    fill: &CxPrimitiveFill,
    world_origin: IVec2,
) {
    let r = radius as i32;
    let diameter = (radius * 2 + 1) as i32;
    let img_w = image.width as i32;
    let img_h = image.image.len() as i32;
    let offset = image.offset();

    let x_min = offset.x.max(0);
    let y_min = offset.y.max(0);
    let x_max = (offset.x + diameter).min(img_w);
    let y_max = (offset.y + diameter).min(img_h);

    let r_sq = r * r;

    for y in y_min..y_max {
        let local_y = y - offset.y;
        let dy = local_y - r;
        for x in x_min..x_max {
            let local_x = x - offset.x;
            let dx = local_x - r;
            if dx * dx + dy * dy <= r_sq {
                let wx = world_origin.x + local_x;
                let wy = world_origin.y + local_y;
                let idx = evaluate_fill(fill, wx, wy);
                if idx != 0 {
                    *image.image_pixel_mut(IVec2::new(x, y)) = idx;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Spatial helpers for the render pipeline
// ---------------------------------------------------------------------------

/// Returns the pixel size of a primitive shape (used for anchor calculation).
#[must_use]
pub(crate) fn primitive_frame_size(shape: &CxPrimitiveShape) -> UVec2 {
    match shape {
        CxPrimitiveShape::Rect { size } => *size,
        CxPrimitiveShape::Circle { radius } => {
            let d = *radius * 2 + 1;
            UVec2::new(d, d)
        }
        CxPrimitiveShape::Polygon { vertices } => {
            if vertices.is_empty() {
                return UVec2::ZERO;
            }
            let (mut min_x, mut min_y) = (i32::MAX, i32::MAX);
            let (mut max_x, mut max_y) = (i32::MIN, i32::MIN);
            for v in vertices {
                min_x = min_x.min(v.x);
                min_y = min_y.min(v.y);
                max_x = max_x.max(v.x);
                max_y = max_y.max(v.y);
            }
            UVec2::new((max_x - min_x + 1) as u32, (max_y - min_y + 1) as u32)
        }
    }
}

// ---------------------------------------------------------------------------
// Extraction
// ---------------------------------------------------------------------------

pub(crate) type PrimitiveComponents<L> = (
    &'static CxPrimitive,
    &'static CxPosition,
    &'static CxAnchor,
    &'static L,
    &'static CxRenderSpace,
    Option<&'static CxPresentationTransform>,
);

#[cfg(feature = "headed")]
fn extract_primitives<L: CxLayer>(
    primitives: Extract<Query<(PrimitiveComponents<L>, &InheritedVisibility, RenderEntity)>>,
    mut cmd: Commands,
) {
    for ((prim, &pos, &anchor, layer, &canvas, presentation), visibility, id) in &primitives {
        if !visibility.get() {
            continue;
        }
        let mut entity = cmd.entity(id);
        entity.insert((prim.clone(), pos, anchor, layer.clone(), canvas));
        if let Some(&pt) = presentation {
            entity.insert(pt);
        } else {
            entity.remove::<CxPresentationTransform>();
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::naive_bytecount)]
mod tests {
    use super::*;
    use crate::image::CxImage;

    fn make_image(w: u32, h: u32) -> CxImage {
        CxImage::empty(UVec2::new(w, h))
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

    // --- Solid rect ---

    #[test]
    fn solid_rect_fills_entire_area() {
        let mut image = make_image(4, 4);
        let prim = CxPrimitive {
            shape: CxPrimitiveShape::Rect {
                size: UVec2::new(2, 2),
            },
            fill: CxPrimitiveFill::Solid(5),
        };
        let mut slice = image.slice_all_mut();
        let mut sub = slice.slice_mut(IRect {
            min: IVec2::new(1, 1),
            max: IVec2::new(3, 3),
        });
        draw_primitive(&prim, &mut sub, IVec2::new(10, 20));

        #[rustfmt::skip]
        let expected = vec![
            0, 0, 0, 0,
            0, 5, 5, 0,
            0, 5, 5, 0,
            0, 0, 0, 0,
        ];
        assert_eq!(pixels(&image), expected);
    }

    #[test]
    fn solid_rect_index_zero_is_transparent() {
        let mut image = CxImage::new(vec![3; 9], 3);
        let prim = CxPrimitive {
            shape: CxPrimitiveShape::Rect {
                size: UVec2::new(3, 3),
            },
            fill: CxPrimitiveFill::Solid(0),
        };
        let mut slice = image.slice_all_mut();
        draw_primitive(&prim, &mut slice, IVec2::ZERO);

        // All pixels should remain 3 because index 0 is skipped.
        assert!(pixels(&image).iter().all(|&p| p == 3));
    }

    #[test]
    fn solid_rect_clips_at_image_bounds() {
        let mut image = make_image(4, 4);
        let prim = CxPrimitive {
            shape: CxPrimitiveShape::Rect {
                size: UVec2::new(3, 3),
            },
            fill: CxPrimitiveFill::Solid(7),
        };
        // Place at (2,2) — extends beyond (4,4) bounds.
        let mut slice = image.slice_all_mut();
        let mut sub = slice.slice_mut(IRect {
            min: IVec2::new(2, 2),
            max: IVec2::new(5, 5),
        });
        draw_primitive(&prim, &mut sub, IVec2::new(2, 2));

        #[rustfmt::skip]
        let expected = vec![
            0, 0, 0, 0,
            0, 0, 0, 0,
            0, 0, 7, 7,
            0, 0, 7, 7,
        ];
        assert_eq!(pixels(&image), expected);
    }

    // --- Checker ---

    #[test]
    fn checker_alternates_by_world_parity() {
        let mut image = make_image(4, 2);
        let prim = CxPrimitive {
            shape: CxPrimitiveShape::Rect {
                size: UVec2::new(4, 2),
            },
            fill: CxPrimitiveFill::Checker { a: 1, b: 2 },
        };
        let mut slice = image.slice_all_mut();
        draw_primitive(&prim, &mut slice, IVec2::new(0, 0));

        #[rustfmt::skip]
        let expected = vec![
            1, 2, 1, 2,
            2, 1, 2, 1,
        ];
        assert_eq!(pixels(&image), expected);
    }

    #[test]
    fn checker_world_offset_shifts_pattern() {
        let mut image = make_image(2, 2);
        let prim = CxPrimitive {
            shape: CxPrimitiveShape::Rect {
                size: UVec2::new(2, 2),
            },
            fill: CxPrimitiveFill::Checker { a: 1, b: 2 },
        };
        // World origin (1, 0) shifts parity by 1 in x.
        let mut slice = image.slice_all_mut();
        draw_primitive(&prim, &mut slice, IVec2::new(1, 0));

        #[rustfmt::skip]
        let expected = vec![
            2, 1,
            1, 2,
        ];
        assert_eq!(pixels(&image), expected);
    }

    #[test]
    fn checker_index_zero_punches_holes() {
        // Pre-fill with color 9, then draw a checker where b=0.
        // Odd-parity pixels should remain 9 (index 0 is skipped = hole).
        let mut image = CxImage::new(vec![9; 4], 2);
        let prim = CxPrimitive {
            shape: CxPrimitiveShape::Rect {
                size: UVec2::new(2, 2),
            },
            fill: CxPrimitiveFill::Checker { a: 5, b: 0 },
        };
        let mut slice = image.slice_all_mut();
        draw_primitive(&prim, &mut slice, IVec2::ZERO);

        #[rustfmt::skip]
        let expected = vec![
            5, 9, // (0,0)=even→5, (1,0)=odd→0→skipped→9
            9, 5, // (0,1)=odd→0→skipped→9, (1,1)=even→5
        ];
        assert_eq!(pixels(&image), expected);
    }

    // --- Ordered dither ---

    #[test]
    fn dither_threshold_zero_is_all_a() {
        let mut image = make_image(4, 4);
        let prim = CxPrimitive {
            shape: CxPrimitiveShape::Rect {
                size: UVec2::new(4, 4),
            },
            fill: CxPrimitiveFill::OrderedDither {
                a: 3,
                b: 7,
                threshold: 0,
            },
        };
        let mut slice = image.slice_all_mut();
        draw_primitive(&prim, &mut slice, IVec2::ZERO);

        assert!(pixels(&image).iter().all(|&p| p == 3));
    }

    #[test]
    fn dither_threshold_16_is_all_b() {
        let mut image = make_image(4, 4);
        let prim = CxPrimitive {
            shape: CxPrimitiveShape::Rect {
                size: UVec2::new(4, 4),
            },
            fill: CxPrimitiveFill::OrderedDither {
                a: 3,
                b: 7,
                threshold: 16,
            },
        };
        let mut slice = image.slice_all_mut();
        draw_primitive(&prim, &mut slice, IVec2::ZERO);

        assert!(pixels(&image).iter().all(|&p| p == 7));
    }

    #[test]
    fn dither_threshold_8_is_roughly_half() {
        let mut image = make_image(4, 4);
        let prim = CxPrimitive {
            shape: CxPrimitiveShape::Rect {
                size: UVec2::new(4, 4),
            },
            fill: CxPrimitiveFill::OrderedDither {
                a: 3,
                b: 7,
                threshold: 8,
            },
        };
        let mut slice = image.slice_all_mut();
        draw_primitive(&prim, &mut slice, IVec2::ZERO);

        let px = pixels(&image);
        let count_b = px.iter().filter(|&&p| p == 7).count();
        assert_eq!(count_b, 8, "threshold 8 on 4x4 should produce 8 of b");
    }

    #[test]
    fn dither_stable_across_world_offset() {
        // Same threshold, different world origins — pattern shifts but pixel
        // count stays the same.
        let mut img_a = make_image(4, 4);
        let mut img_b = make_image(4, 4);
        let prim = CxPrimitive {
            shape: CxPrimitiveShape::Rect {
                size: UVec2::new(4, 4),
            },
            fill: CxPrimitiveFill::OrderedDither {
                a: 3,
                b: 7,
                threshold: 10,
            },
        };
        let mut slice_a = img_a.slice_all_mut();
        draw_primitive(&prim, &mut slice_a, IVec2::ZERO);
        let mut slice_b = img_b.slice_all_mut();
        draw_primitive(&prim, &mut slice_b, IVec2::new(100, 200));

        let count_a = pixels(&img_a).iter().filter(|&&p| p == 7).count();
        let count_b = pixels(&img_b).iter().filter(|&&p| p == 7).count();
        assert_eq!(
            count_a, count_b,
            "dither count must be stable across offsets"
        );
    }

    // --- Circle ---

    #[test]
    fn circle_radius_zero_is_single_pixel() {
        let mut image = make_image(3, 3);
        let prim = CxPrimitive {
            shape: CxPrimitiveShape::Circle { radius: 0 },
            fill: CxPrimitiveFill::Solid(4),
        };
        let mut slice = image.slice_all_mut();
        let mut sub = slice.slice_mut(IRect {
            min: IVec2::new(1, 1),
            max: IVec2::new(2, 2),
        });
        draw_primitive(&prim, &mut sub, IVec2::ZERO);

        #[rustfmt::skip]
        let expected = vec![
            0, 0, 0,
            0, 4, 0,
            0, 0, 0,
        ];
        assert_eq!(pixels(&image), expected);
    }

    #[test]
    fn circle_radius_2_draws_disk() {
        let mut image = make_image(5, 5);
        let prim = CxPrimitive {
            shape: CxPrimitiveShape::Circle { radius: 2 },
            fill: CxPrimitiveFill::Solid(1),
        };
        let mut slice = image.slice_all_mut();
        draw_primitive(&prim, &mut slice, IVec2::ZERO);

        let px = pixels(&image);
        // Center pixel must be filled.
        assert_eq!(px[12], 1, "center (2,2) must be filled");
        // Corners of 5x5 must be empty (distance > 2).
        assert_eq!(px[0], 0, "corner (0,0) must be empty");
        assert_eq!(px[4], 0, "corner (4,0) must be empty");
        assert_eq!(px[20], 0, "corner (0,4) must be empty");
        assert_eq!(px[24], 0, "corner (4,4) must be empty");
        // Total filled pixels for r=2 disk: 13.
        let filled = px.iter().filter(|&&p| p != 0).count();
        assert_eq!(filled, 13, "radius-2 disk has 13 filled pixels");
    }

    #[test]
    fn circle_clips_at_image_bounds() {
        let mut image = make_image(3, 3);
        let prim = CxPrimitive {
            shape: CxPrimitiveShape::Circle { radius: 2 },
            fill: CxPrimitiveFill::Solid(1),
        };
        // Circle diameter 5 placed at (0,0) in a 3x3 image — clips to top-left.
        let mut slice = image.slice_all_mut();
        draw_primitive(&prim, &mut slice, IVec2::ZERO);

        let px = pixels(&image);
        let filled = px.iter().filter(|&&p| p != 0).count();
        assert!(filled > 0, "some pixels must be filled");
        assert!(filled < 13, "must be clipped from full 13-pixel disk");
    }

    // --- Polygon (deferred) ---

    #[test]
    fn polygon_produces_no_pixels_yet() {
        let mut image = make_image(4, 4);
        let prim = CxPrimitive {
            shape: CxPrimitiveShape::Polygon {
                vertices: vec![IVec2::ZERO, IVec2::new(3, 0), IVec2::new(0, 3)],
            },
            fill: CxPrimitiveFill::Solid(5),
        };
        let mut slice = image.slice_all_mut();
        draw_primitive(&prim, &mut slice, IVec2::ZERO);

        assert!(
            pixels(&image).iter().all(|&p| p == 0),
            "polygon rasterisation is deferred"
        );
    }

    // --- primitive_frame_size ---

    #[test]
    fn frame_size_rect() {
        let size = primitive_frame_size(&CxPrimitiveShape::Rect {
            size: UVec2::new(10, 20),
        });
        assert_eq!(size, UVec2::new(10, 20));
    }

    #[test]
    fn frame_size_circle() {
        let size = primitive_frame_size(&CxPrimitiveShape::Circle { radius: 3 });
        assert_eq!(size, UVec2::new(7, 7)); // diameter = 2*3+1
    }

    #[test]
    fn frame_size_polygon_from_bounds() {
        let size = primitive_frame_size(&CxPrimitiveShape::Polygon {
            vertices: vec![IVec2::new(-5, 0), IVec2::new(5, 10)],
        });
        // -5..5 inclusive = 11 pixels wide, 0..10 inclusive = 11 pixels tall.
        assert_eq!(size, UVec2::new(11, 11));
    }

    // --- Render pipeline integration ---

    #[test]
    fn primitive_via_sub_slice_writes_to_parent_image() {
        // Simulates the exact render pipeline flow:
        // layer_image → slice_all_mut → slice_mut(sub_rect) → draw_primitive
        let mut layer_image = make_image(8, 8);
        let mut layer_slice = layer_image.slice_all_mut();

        let prim = CxPrimitive {
            shape: CxPrimitiveShape::Rect {
                size: UVec2::new(4, 3),
            },
            fill: CxPrimitiveFill::Solid(5),
        };

        let image_pos = IVec2::new(2, 2);
        let size = UVec2::new(4, 3);
        let mut prim_slice = layer_slice.slice_mut(IRect {
            min: image_pos,
            max: image_pos + size.as_ivec2(),
        });

        draw_primitive(&prim, &mut prim_slice, IVec2::new(100, 200));

        // Verify the parent image has the pixels.
        let px = pixels(&layer_image);
        let row2 = 2 * 8;
        assert_eq!(px[row2 + 2], 5, "pixel (2,2)");
        assert_eq!(px[row2 + 5], 5, "pixel (5,2)");
        assert_eq!(px[row2 + 1], 0, "pixel (1,2) outside rect");
        let filled = px.iter().filter(|&&p| p != 0).count();
        assert_eq!(filled, 12, "4x3 rect = 12 filled pixels");
    }
}
