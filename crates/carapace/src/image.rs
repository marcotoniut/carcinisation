use bevy_render::render_resource::TextureFormat;
use serde::{Deserialize, Serialize};

use crate::{math::RectExt, palette::Palette, prelude::*};

/// Palette-indexed raster buffer with a valid row layout.
///
/// # Layout invariant
///
/// - `width` is always > 0 (row stride for index arithmetic).
/// - Storage is row-aligned: when non-empty, `image.len()` is a multiple of
///   `width`.
/// - Height may be 0 (empty data with a valid width). This is a valid layout
///   — it simply contains no rows.
/// - Width 0 is unconditionally invalid; it would cause division-by-zero in
///   [`height()`](Self::height), [`size()`](Self::size), and
///   [`get_pixel()`](Self::get_pixel).
///
/// Zero-area *render* states (unresolved composites, assets not yet loaded)
/// are represented in render metrics ([`UVec2`] size fields on composites) —
/// not by manufacturing a degenerate `CxImage`. The renderer skips zero-area
/// metrics at the draw boundary.
#[derive(Serialize, Deserialize, Clone, Reflect, Debug)]
pub struct CxImage {
    image: Vec<u8>,
    width: usize,
}

impl CxImage {
    /// Construct from raw palette-index data.
    ///
    /// Caller must satisfy the layout invariant: `width` > 0, and when `image`
    /// is non-empty its length must be a multiple of `width`. An empty `image`
    /// vec with a positive width is valid (zero rows, height 0).
    ///
    /// # Panics (debug)
    ///
    /// - `width` is 0.
    /// - `image` is non-empty and its length is not a multiple of `width`.
    pub fn new(image: Vec<u8>, width: usize) -> Self {
        debug_assert!(width > 0, "CxImage: width must be > 0 (got width 0)");
        debug_assert!(
            image.is_empty() || image.len().is_multiple_of(width),
            "CxImage: data length ({}) must be a multiple of width ({width})",
            image.len(),
        );
        Self { image, width }
    }

    /// Allocate a zeroed raster of the given size (both dimensions must be > 0).
    ///
    /// # Panics (debug)
    ///
    /// Either dimension is 0.
    pub fn empty(size: UVec2) -> Self {
        debug_assert!(
            size.x > 0 && size.y > 0,
            "CxImage::empty: dimensions must be > 0 (got {size})",
        );
        Self {
            image: vec![0; (size.x * size.y) as usize],
            width: size.x as usize,
        }
    }

    /// Whether this image has no pixel data.
    #[allow(unused)]
    pub(crate) fn is_empty(&self) -> bool {
        self.image.is_empty()
    }

    pub(crate) fn empty_from_image(image: &Image) -> Self {
        Self::empty(image.size())
    }

    pub(crate) fn palette_indices(palette: &Palette, image: &Image) -> Result<Self> {
        Ok(Self {
            image: image
                .convert(TextureFormat::Rgba8UnormSrgb)
                .ok_or("could not convert image to `Rgba8UnormSrgb`")?
                .data
                .ok_or("image is not initialized")?
                .chunks_exact(4)
                .map(|color| {
                    if color[3] == 0 {
                        Ok(0)
                    } else {
                        palette
                            .indices
                            .get(&[color[0], color[1], color[2]])
                            .copied()
                            .ok_or_else(|| {
                                format!(
                                    "a sprite contained a color `#{:02X}{:02X}{:02X}` \
                                    that wasn't in the palette",
                                    color[0], color[1], color[2]
                                )
                                .into()
                            })
                    }
                })
                .collect::<Result<_>>()?,
            width: image.texture_descriptor.size.width as usize,
        })
    }

    /// Read a single palette index at the given image-space position (unchecked).
    ///
    /// # Panics
    ///
    /// Out-of-bounds access. Prefer [`get_pixel`](Self::get_pixel) from external code.
    pub(crate) fn pixel(&self, position: IVec2) -> u8 {
        self.image[(position.x + position.y * self.width as i32) as usize]
    }

    /// Bounds-checked pixel read. Returns `None` for out-of-bounds positions.
    pub fn get_pixel(&self, position: IVec2) -> Option<u8> {
        IRect {
            min: IVec2::splat(0),
            max: IVec2::new(self.width as i32, (self.image.len() / self.width) as i32),
        }
        .contains_exclusive(position)
        .then(|| self.pixel(position))
    }

    /// Dimensions as `(width, height)`.
    pub fn size(&self) -> UVec2 {
        UVec2::new(self.width as u32, (self.image.len() / self.width) as u32)
    }

    /// Width in pixels.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Height in pixels.
    pub fn height(&self) -> usize {
        self.image.len() / self.width
    }

    /// Total pixel count (`width * height`).
    pub fn area(&self) -> usize {
        self.image.len()
    }

    /// Raw palette-index data (row-major, image-space: top-left origin, Y-down).
    pub fn data(&self) -> &[u8] {
        &self.image
    }

    /// Mutable access to raw palette-index data.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.image
    }

    #[expect(unused)]
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut u8> {
        self.image.iter_mut()
    }

    #[expect(unused)]
    pub(crate) fn slice_mut(&mut self, slice: IRect) -> CxImageSliceMut<'_> {
        CxImageSliceMut {
            slice,
            image: self.image.chunks_exact_mut(self.width).collect(),
            width: self.width,
        }
    }

    pub(crate) fn slice_all_mut(&mut self) -> CxImageSliceMut<'_> {
        CxImageSliceMut {
            slice: IRect {
                min: IVec2::splat(0),
                max: IVec2::new(self.width as i32, (self.image.len() / self.width) as i32),
            },
            image: self.image.chunks_exact_mut(self.width).collect(),
            width: self.width,
        }
    }

    pub(crate) fn split_vert(self, chunk_height: usize) -> Vec<Self> {
        self.image
            .chunks_exact(chunk_height * self.width)
            .map(|chunk| Self {
                image: chunk.into(),
                width: self.width,
            })
            .collect()
    }

    pub(crate) fn split_horz(self, chunk_width: usize) -> Vec<Self> {
        let chunk_count = self.width / chunk_width;
        let mut images = vec![Vec::with_capacity(self.area() / chunk_width); chunk_count];

        for (i, chunk_row) in self.image.chunks_exact(chunk_width).enumerate() {
            images[i % chunk_count].push(chunk_row);
        }

        images
            .into_iter()
            .map(|image| Self {
                image: image.into_iter().flatten().copied().collect(),
                width: chunk_width,
            })
            .collect()
    }

    pub(crate) fn trim_right(&mut self) {
        if self.image.is_empty() {
            return;
        }

        while self.width > 1
            && (0..self.height()).all(|row| self.image[self.width * (row + 1) - 1] == 0)
        {
            for row in (0..self.height()).rev() {
                self.image.remove(row * self.width + self.width - 1);
            }

            self.width -= 1;
        }
    }

    pub(crate) fn from_parts_vert(parts: impl IntoIterator<Item = Self>) -> Option<Self> {
        let (images, widths): (Vec<_>, Vec<_>) = parts
            .into_iter()
            .map(|image| (image.image, image.width))
            .unzip();

        match (&widths) as &[_] {
            [width, other_widths @ ..] => other_widths
                .iter()
                .all(|other_width| other_width == width)
                .then(|| Self {
                    image: images.into_iter().flatten().collect(),
                    width: *width,
                }),
            [] => None,
        }
    }

    /// Zero-fill all pixel data (all pixels become transparent index 0).
    pub fn clear(&mut self) {
        self.image.fill(default());
    }
}

pub(crate) struct CxImageSliceMut<'a> {
    // TODO Currently, this is the entire image. Trim it down to the slice that this should have
    // access to.
    pub image: Vec<&'a mut [u8]>,
    pub width: usize,
    pub slice: IRect,
}

/// Clamp a 1D span `[start, start+len)` into `[0, max)`, returning a `usize` range.
#[inline]
pub(crate) fn clamp_span(start: i32, len: i32, max: i32) -> std::ops::Range<usize> {
    let lo = start.max(0).min(max) as usize;
    let hi = (start + len).max(0).min(max) as usize;
    lo..hi
}

impl<'a> CxImageSliceMut<'a> {
    pub(crate) fn from_image_mut(image: &'a mut Image) -> Result<Self> {
        let w = image.texture_descriptor.size.width as usize;
        let h = image.texture_descriptor.size.height as i32;
        Ok(Self {
            slice: IRect {
                min: IVec2::ZERO,
                max: IVec2::new(w as i32, h),
            },
            image: image
                .data
                .as_mut()
                .ok_or("image is not initialized")?
                .chunks_exact_mut(w)
                .collect(),
            width: w,
        })
    }

    // --- Dimensions ---

    /// Backing image width as `i32` (avoids `as` casts at call sites).
    #[inline]
    pub(crate) fn img_width_i(&self) -> i32 {
        self.width as i32
    }

    /// Backing image height as `i32`.
    #[inline]
    pub(crate) fn img_height_i(&self) -> i32 {
        self.image.len() as i32
    }

    /// Flip a world-space Y-up position to image-space Y-down.
    ///
    /// `image_y = height - world_y`. The caller subtracts sprite height
    /// separately when computing the top-left corner of a rect.
    #[inline]
    pub(crate) fn flip_y(&self, world_y: i32) -> i32 {
        self.slice.height() - world_y
    }

    // --- Pixel access ---

    /// First `usize` is the index in the slice. Second `usize` is the index in the image.
    /// First `usize` is the index in the slice. Second `usize` is the index in the image.
    pub(crate) fn for_each_mut(&mut self, mut f: impl FnMut(usize, usize, &mut u8)) {
        let x_range = clamp_span(self.slice.min.x, self.slice.width(), self.img_width_i());
        let y_range = clamp_span(self.slice.min.y, self.slice.height(), self.img_height_i());
        let slice_w = self.slice.width().max(0) as usize;
        // slice_index offsets from the unclamped slice origin, not the clamped
        // image bounds. A slice at min=(-5, -3) has pixel (0,0) at slice
        // index (5, 3), not (0, 0).
        let sx_off = self.slice.min.x;
        let sy_off = self.slice.min.y;

        for (row_index, row) in self.image[y_range.clone()].iter_mut().enumerate() {
            let y = y_range.start + row_index;
            for x in x_range.clone() {
                let slice_index =
                    (y as i32 - sy_off) as usize * slice_w + (x as i32 - sx_off) as usize;
                let image_index = y * self.width + x;
                f(slice_index, image_index, &mut row[x]);
            }
        }
    }

    pub(crate) fn contains_pixel(&self, position: IVec2) -> bool {
        let img_bounds = IRect {
            min: IVec2::ZERO,
            max: IVec2::new(self.img_width_i(), self.img_height_i()),
        };
        img_bounds.contains_exclusive(position - self.slice.min)
            && self.slice.contains_exclusive(position)
    }

    /// Pixel at a **slice-local** position (offset by `slice.min` internally).
    pub(crate) fn slice_pixel_mut(&mut self, position: IVec2) -> &mut u8 {
        &mut self.image[(self.slice.min.y + position.y) as usize]
            [(self.slice.min.x + position.x) as usize]
    }

    pub(crate) fn get_pixel_mut(&mut self, position: IVec2) -> Option<&mut u8> {
        self.contains_pixel(position)
            .then(|| self.slice_pixel_mut(position))
    }

    /// Pixel at an **absolute image** position (no slice offset).
    pub(crate) fn abs_pixel_mut(&mut self, position: IVec2) -> &mut u8 {
        &mut self.image[position.y as usize][position.x as usize]
    }

    #[expect(unused)]
    pub(crate) fn size(&self) -> UVec2 {
        self.slice.size().as_uvec2()
    }

    #[allow(unused)]
    pub(crate) fn offset(&self) -> IVec2 {
        self.slice.min
    }

    pub(crate) fn slice_mut(&mut self, slice: IRect) -> CxImageSliceMut<'_> {
        CxImageSliceMut {
            image: self.image.iter_mut().map(|row| &mut **row).collect(),
            width: self.width,
            slice: IRect {
                min: slice.min + self.slice.min,
                max: slice.max + self.slice.min,
            },
        }
    }

    pub(crate) fn draw(&mut self, image: &CxImage) {
        self.for_each_mut(|i, _, pixel| {
            let new_pixel = image.image[i];
            if new_pixel != crate::palette::TRANSPARENT_INDEX {
                *pixel = new_pixel;
            }
        });
    }
}

// --- Test-only helpers ---

#[cfg(test)]
impl CxImage {
    /// Extract the tight bounding box of non-zero pixels as a row-major 2D grid.
    ///
    /// Useful for verifying exact pixel layouts in tiny-matrix tests without
    /// depending on a specific image size or padding.
    pub(crate) fn nonzero_grid(&self) -> Vec<Vec<u8>> {
        let size = self.size();
        let (mut min_x, mut min_y) = (size.x as i32, size.y as i32);
        let (mut max_x, mut max_y) = (-1_i32, -1_i32);
        for y in 0..size.y as i32 {
            for x in 0..size.x as i32 {
                if self.pixel(IVec2::new(x, y)) != crate::palette::TRANSPARENT_INDEX {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }
        if max_x < 0 {
            return vec![];
        }
        (min_y..=max_y)
            .map(|y| {
                (min_x..=max_x)
                    .map(|x| self.pixel(IVec2::new(x, y)))
                    .collect()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::Palette;
    use bevy_asset::RenderAssetUsages;
    use bevy_image::Image;
    use bevy_render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    fn rgba_image(width: u32, height: u32, pixels: &[[u8; 4]]) -> Image {
        let data = pixels.iter().flatten().copied().collect();
        Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        )
    }

    fn make_palette() -> Palette {
        // 2×1 palette: pixel 0 transparent (background), pixel 1 = red.
        let img = rgba_image(2, 1, &[[0, 0, 0, 0], [255, 0, 0, 255]]);
        Palette::new(&img).unwrap()
    }

    #[test]
    fn palette_indices_transparent_maps_to_zero() {
        let palette = make_palette();
        // Transparent pixel → index 0 regardless of RGB.
        let img = rgba_image(1, 1, &[[99, 0, 0, 0]]);
        let px = CxImage::palette_indices(&palette, &img).unwrap();
        assert_eq!(px.pixel(IVec2::ZERO), 0);
    }

    #[test]
    fn palette_indices_known_color_maps_to_index() {
        let palette = make_palette();
        // Red opaque → index 1.
        let img = rgba_image(1, 1, &[[255, 0, 0, 255]]);
        let px = CxImage::palette_indices(&palette, &img).unwrap();
        assert_eq!(px.pixel(IVec2::ZERO), 1);
    }

    #[test]
    fn palette_indices_unknown_color_errors() {
        let palette = make_palette();
        // Green is not in the palette.
        let img = rgba_image(1, 1, &[[0, 255, 0, 255]]);
        let err = CxImage::palette_indices(&palette, &img).unwrap_err();
        assert!(
            err.to_string().contains("wasn't in the palette"),
            "got: {err}"
        );
    }

    #[test]
    fn palette_indices_width_is_preserved() {
        let palette = make_palette();
        // 2×1 image: transparent then red.
        let img = rgba_image(2, 1, &[[0, 0, 0, 0], [255, 0, 0, 255]]);
        let px = CxImage::palette_indices(&palette, &img).unwrap();
        assert_eq!(px.size(), UVec2::new(2, 1));
        assert_eq!(px.pixel(IVec2::new(0, 0)), 0);
        assert_eq!(px.pixel(IVec2::new(1, 0)), 1);
    }

    // -- Positive-width invariant tests --
    //
    // CxImage requires width > 0 unconditionally. Zero-area render states are
    // represented in composite metrics, not in CxImage itself.

    #[test]
    #[should_panic(expected = "width must be > 0")]
    fn new_rejects_zero_width_with_data() {
        let _ = CxImage::new(vec![1, 2, 3], 0);
    }

    #[test]
    #[should_panic(expected = "width must be > 0")]
    fn new_rejects_zero_width_with_empty_data() {
        let _ = CxImage::new(vec![], 0);
    }

    #[test]
    #[should_panic(expected = "must be a multiple")]
    fn new_rejects_misaligned_data() {
        let _ = CxImage::new(vec![1, 2, 3], 2);
    }

    #[test]
    #[should_panic(expected = "dimensions must be > 0")]
    fn empty_rejects_zero_width() {
        let _ = CxImage::empty(UVec2::new(0, 10));
    }

    #[test]
    #[should_panic(expected = "dimensions must be > 0")]
    fn empty_rejects_zero_height() {
        let _ = CxImage::empty(UVec2::new(10, 0));
    }

    #[test]
    fn zero_height_with_valid_width_satisfies_layout_invariant() {
        // Empty data + positive width is a valid layout: zero rows, height 0.
        // The layout invariant (width > 0, row-aligned) holds — no panic.
        let img = CxImage::new(vec![], 1);
        assert!(img.is_empty());
        assert_eq!(img.width(), 1);
        assert_eq!(img.height(), 0);
        assert_eq!(img.area(), 0);
        assert_eq!(img.size(), UVec2::new(1, 0));
    }

    #[test]
    fn valid_image_reports_correct_dimensions() {
        let img = CxImage::empty(UVec2::new(4, 3));
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 3);
        assert_eq!(img.size(), UVec2::new(4, 3));
        assert!(!img.is_empty());
    }

    // --- for_each_mut slice_index regression tests ---

    #[test]
    fn for_each_mut_slice_index_with_positive_offset() {
        // A 6x4 image with a 2x2 slice at offset (2, 1).
        // slice_index must be relative to the slice origin, not the
        // clamped image bounds.
        let mut image = CxImage::empty(UVec2::new(6, 4));
        let mut full = image.slice_all_mut();
        let mut sub = full.slice_mut(IRect {
            min: IVec2::new(2, 1),
            max: IVec2::new(4, 3),
        });

        let mut indices = Vec::new();
        sub.for_each_mut(|slice_i, image_i, _pixel| {
            indices.push((slice_i, image_i));
        });

        // Slice is 2x2: slice_index should be 0,1,2,3.
        // image_index should be row*6+col for the absolute positions.
        // image_index = row * width + col, width = 6
        assert_eq!(indices, vec![(0, 8), (1, 9), (2, 14), (3, 15)]);
    }

    #[test]
    fn for_each_mut_slice_index_with_negative_offset() {
        // A 4x4 image with a slice starting at (-1, -1).
        // Only the portion within [0,0)→(2,2) is iterable.
        // slice_index must still offset from the unclamped slice origin.
        let mut image = CxImage::empty(UVec2::new(4, 4));
        let mut full = image.slice_all_mut();
        let mut sub = full.slice_mut(IRect {
            min: IVec2::new(-1, -1),
            max: IVec2::new(2, 2),
        });

        let mut indices = Vec::new();
        sub.for_each_mut(|slice_i, _image_i, _pixel| {
            indices.push(slice_i);
        });

        // Slice is 3x3 (from -1 to 2), but only pixels at x=0,1 y=0,1
        // are within image bounds. The slice_index for pixel (0,0) in
        // the image maps to slice column 1, row 1 (offset from -1,-1):
        //   slice_index = (0 - (-1)) * 3 + (0 - (-1)) = 1*3 + 1 = 4
        // slice is 3 wide (from -1 to 2). Image pixel (0,0) maps to
        // slice row 1 col 1 (offset from -1,-1): index = 1*3+1 = 4.
        assert_eq!(indices, vec![4, 5, 7, 8]);
    }

    #[test]
    fn draw_via_sub_slice_places_pixels_correctly() {
        // Regression: a sprite drawn into a sub-slice must land at the
        // correct absolute image positions. The Frames::draw callback
        // reads sprite data via slice_index and writes via the pixel
        // reference (absolute). If slice_index is wrong, the sprite
        // data is read from the wrong offset.
        let mut image = CxImage::empty(UVec2::new(8, 8));
        // Write a recognisable pattern: palette index = (row + 1)
        let pattern = CxImage::new(vec![1, 1, 2, 2, 3, 3, 4, 4], 2);

        let mut full = image.slice_all_mut();
        // Place the 2x4 pattern at image position (3, 2).
        let mut sub = full.slice_mut(IRect {
            min: IVec2::new(3, 2),
            max: IVec2::new(5, 6),
        });
        sub.draw(&pattern);

        // Verify the pattern landed at the right place.
        assert_eq!(image.pixel(IVec2::new(3, 2)), 1);
        assert_eq!(image.pixel(IVec2::new(4, 2)), 1);
        assert_eq!(image.pixel(IVec2::new(3, 3)), 2);
        assert_eq!(image.pixel(IVec2::new(4, 3)), 2);
        assert_eq!(image.pixel(IVec2::new(3, 4)), 3);
        assert_eq!(image.pixel(IVec2::new(3, 5)), 4);
        // Surrounding pixels must be untouched.
        assert_eq!(image.pixel(IVec2::new(2, 2)), 0);
        assert_eq!(image.pixel(IVec2::new(5, 2)), 0);
    }
}
