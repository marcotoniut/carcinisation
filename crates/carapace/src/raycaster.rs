//! Generic helpers for raycaster-style rendering into palette-indexed buffers.

use crate::image::CxImage;
use crate::palette::TRANSPARENT_INDEX;

/// Draw a vertical texture column into an image.
///
/// Samples the texture at column `tex_x` and stretches it vertically to fill
/// `y_start..y_end` in the destination image using nearest-neighbour sampling.
///
/// All coordinates are in **image space** (top-left origin, Y-down).
/// Transparent pixels (index 0) are skipped.
/// Clips to image bounds. No allocations.
pub fn draw_wall_column(
    image: &mut CxImage,
    x: i32,
    y_start: i32,
    y_end: i32,
    texture: &CxImage,
    tex_x: i32,
) {
    let img_w = image.width() as i32;
    let img_h = image.height() as i32;

    if x < 0 || x >= img_w {
        return;
    }

    let tex_h = texture.height() as i32;
    let strip_h = y_end - y_start;
    if strip_h <= 0 || tex_h == 0 {
        return;
    }

    let y_min = y_start.max(0);
    let y_max = y_end.min(img_h);

    let data = image.data_mut();
    let tex_data = texture.data();
    let tex_w = texture.width() as i32;

    // Bounds-check tex_x once up front.
    if tex_x < 0 || tex_x >= tex_w {
        return;
    }

    for y in y_min..y_max {
        let tex_y = ((y - y_start) * tex_h / strip_h).min(tex_h - 1);
        let pixel = tex_data[(tex_y * tex_w + tex_x) as usize];
        if pixel != TRANSPARENT_INDEX {
            data[(y * img_w + x) as usize] = pixel;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_math::UVec2;

    #[test]
    fn wall_column_basic() {
        let mut image = CxImage::empty(UVec2::new(4, 4));
        // 2x2 texture: palette indices 1,2 / 3,4
        let texture = CxImage::new(vec![1, 2, 3, 4], 2);

        draw_wall_column(&mut image, 1, 0, 4, &texture, 0);

        let data = image.data();
        // Column x=1, stretched 2 rows → 4 rows. tex_x=0 samples column [1, 3].
        // y=0: tex_y = 0*2/4 = 0 → pixel 1
        // y=1: tex_y = 1*2/4 = 0 → pixel 1
        // y=2: tex_y = 2*2/4 = 1 → pixel 3
        // y=3: tex_y = 3*2/4 = 1 → pixel 3
        assert_eq!(data[1], 1);
        assert_eq!(data[5], 1);
        assert_eq!(data[2 * 4 + 1], 3);
        assert_eq!(data[3 * 4 + 1], 3);
    }

    #[test]
    fn wall_column_clips_y() {
        let mut image = CxImage::empty(UVec2::new(4, 4));
        let texture = CxImage::new(vec![5, 5], 1);

        // y_start=-2, y_end=2 → only y=0,1 get drawn
        draw_wall_column(&mut image, 0, -2, 2, &texture, 0);

        let data = image.data();
        assert_eq!(data[0], 5);
        assert_eq!(data[4], 5);
        assert_eq!(data[2 * 4], 0); // untouched
    }

    #[test]
    fn wall_column_out_of_bounds_x() {
        let mut image = CxImage::empty(UVec2::new(4, 4));
        let texture = CxImage::new(vec![5], 1);

        draw_wall_column(&mut image, -1, 0, 4, &texture, 0);
        draw_wall_column(&mut image, 4, 0, 4, &texture, 0);

        // Nothing should have changed.
        assert!(image.data().iter().all(|&p| p == 0));
    }

    #[test]
    fn wall_column_skips_transparent() {
        let mut image = CxImage::empty(UVec2::new(2, 2));
        image.data_mut().fill(7); // prefill
        let texture = CxImage::new(vec![0, 0], 1); // all transparent

        draw_wall_column(&mut image, 0, 0, 2, &texture, 0);

        // Transparent pixels should not overwrite.
        assert!(image.data().iter().all(|&p| p == 7));
    }
}
