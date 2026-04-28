//! Render orchestration: produces a full-frame [`CxImage`] from map + camera.

use carapace::image::CxImage;
use carapace::raycaster::draw_wall_column;

use crate::billboard::{Billboard, draw_billboard, project_billboard};
use crate::camera::FpCamera;
use crate::map::FpMap;
use crate::raycast::{HitSide, cast_ray};

/// 4x4 Bayer ordered-dither threshold matrix (values 0..15).
pub(crate) const BAYER_4X4: [[u8; 4]; 4] =
    [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// Palette indices for ceiling and floor fills.
pub struct FpPalette {
    pub ceiling: u8,
    pub floor: u8,
    /// Palette index used as the fog/darkness color.
    pub fog_color: u8,
    /// Distance at which fog is fully opaque.
    pub fog_distance: f32,
    /// Distance below which no fog is applied.
    pub fog_start: f32,
}

impl Default for FpPalette {
    fn default() -> Self {
        Self {
            ceiling: 1,
            floor: 3,
            fog_color: 1,
            fog_start: 3.0,
            fog_distance: 12.0,
        }
    }
}

/// Apply distance fog to a column of pixels in the image.
///
/// Blends each non-transparent pixel toward `fog_color` based on `fog_t`
/// (0.0 = no fog, 1.0 = fully fogged). Uses ordered dithering for
/// smooth fog banding at low resolution.
fn apply_column_fog(
    image: &mut CxImage,
    x: i32,
    y_start: i32,
    y_end: i32,
    fog_color: u8,
    fog_t: f32,
) {
    if fog_t <= 0.0 {
        return;
    }
    let w = image.width() as i32;
    let h = image.height() as i32;
    let y_min = y_start.max(0);
    let y_max = y_end.min(h);
    let data = image.data_mut();

    // 4x4 Bayer dither threshold (0..16).
    let bayer = BAYER_4X4;

    let fog_level = (fog_t * 16.0) as u8;

    for y in y_min..y_max {
        let idx = (y * w + x) as usize;
        let pixel = data[idx];
        if pixel == 0 {
            continue;
        }
        let threshold = bayer[(y & 3) as usize][(x & 3) as usize];
        if fog_level > threshold {
            data[idx] = fog_color;
        }
    }
}

/// Render a first-person view into `image` (walls only, no billboards).
///
/// `wall_textures` is indexed by `wall_id - 1` (wall_id 0 is empty).
/// Image is cleared and fully redrawn.
pub fn render_fp_view(
    image: &mut CxImage,
    map: &FpMap,
    camera: &FpCamera,
    wall_textures: &[CxImage],
    palette: &FpPalette,
) {
    render_walls(image, map, camera, wall_textures, palette, None);
}

/// Render walls + billboard entities into `image`.
///
/// Billboards are depth-sorted and drawn back-to-front with per-column
/// z-buffer occlusion against walls.
pub fn render_fp_scene(
    image: &mut CxImage,
    map: &FpMap,
    camera: &FpCamera,
    wall_textures: &[CxImage],
    palette: &FpPalette,
    billboards: &[Billboard],
) {
    let w = image.width();
    let h = image.height() as i32;
    let mut zbuffer = vec![f32::MAX; w];

    render_walls(
        image,
        map,
        camera,
        wall_textures,
        palette,
        Some(&mut zbuffer),
    );

    // Sort billboards back-to-front (farthest first).
    let mut projected: Vec<_> = billboards
        .iter()
        .filter_map(|bb| project_billboard(bb, camera, w as i32, h))
        .collect();
    projected.sort_by(|a, b| {
        b.distance
            .partial_cmp(&a.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for proj in &projected {
        let fog = if palette.fog_distance > palette.fog_start {
            let fog_t = ((proj.distance - palette.fog_start)
                / (palette.fog_distance - palette.fog_start))
                .clamp(0.0, 1.0);
            Some((palette.fog_color, fog_t))
        } else {
            None
        };
        draw_billboard(image, &zbuffer, proj, h, fog);
    }
}

/// Internal wall rendering pass. Optionally writes per-column depth to `zbuffer`.
fn render_walls(
    image: &mut CxImage,
    map: &FpMap,
    camera: &FpCamera,
    wall_textures: &[CxImage],
    palette: &FpPalette,
    mut zbuffer: Option<&mut [f32]>,
) {
    let w = image.width() as i32;
    let h = image.height() as i32;

    image.clear();

    let dir = camera.direction();
    let plane = camera.plane();
    let half_h = h / 2;

    for x in 0..w {
        let camera_x = 2.0 * x as f32 / w as f32 - 1.0;
        let ray_dir = dir + plane * camera_x;

        let hit = cast_ray(map, camera.position, ray_dir);

        // Always write depth so billboards are correctly occluded even in
        // open-sky columns (hit.distance = f32::MAX for escaped rays).
        if let Some(ref mut zb) = zbuffer {
            zb[x as usize] = hit.distance;
        }

        if hit.wall_id == 0 {
            fill_column(image, x, 0, half_h, palette.ceiling);
            fill_column(image, x, half_h, h, palette.floor);
            continue;
        }

        let line_height = (h as f32 / hit.distance) as i32;
        let draw_start = half_h - line_height / 2;
        let draw_end = draw_start + line_height;

        // Ceiling above wall.
        fill_column(image, x, 0, draw_start.max(0), palette.ceiling);

        // Wall texture.
        let tex_idx = (hit.wall_id - 1) as usize;
        if tex_idx < wall_textures.len() {
            let tex = &wall_textures[tex_idx];
            let mut tex_x = (hit.wall_x * tex.width() as f32) as i32;
            tex_x = tex_x.clamp(0, tex.width() as i32 - 1);

            if hit.side == HitSide::Horizontal {
                draw_wall_column_shaded(image, x, draw_start, draw_end, tex, tex_x);
            } else {
                draw_wall_column(image, x, draw_start, draw_end, tex, tex_x);
            }
        }

        // Floor below wall.
        fill_column(image, x, draw_end.min(h), h, palette.floor);

        // Distance fog on the wall strip.
        if palette.fog_distance > palette.fog_start {
            let fog_t = ((hit.distance - palette.fog_start)
                / (palette.fog_distance - palette.fog_start))
                .clamp(0.0, 1.0);
            apply_column_fog(image, x, draw_start, draw_end, palette.fog_color, fog_t);
        }
    }
}

/// Fill a vertical span of a single column with a solid palette index.
fn fill_column(image: &mut CxImage, x: i32, y_start: i32, y_end: i32, color: u8) {
    let w = image.width() as i32;
    let h = image.height() as i32;

    if x < 0 || x >= w || color == 0 {
        return;
    }

    let y_min = y_start.max(0);
    let y_max = y_end.min(h);
    let data = image.data_mut();

    for y in y_min..y_max {
        data[(y * w + x) as usize] = color;
    }
}

/// Apply a dithered tint over the entire image (e.g. red death overlay).
pub fn draw_overlay_tint(image: &mut CxImage, color: u8, density: f32) {
    if density <= 0.0 {
        return;
    }
    let w = image.width() as i32;
    let h = image.height() as i32;
    let data = image.data_mut();

    let bayer = BAYER_4X4;
    let level = (density * 16.0) as u8;

    for y in 0..h {
        for x in 0..w {
            let threshold = bayer[(y & 3) as usize][(x & 3) as usize];
            if level > threshold {
                let idx = (y * w + x) as usize;
                if data[idx] != 0 {
                    data[idx] = color;
                }
            }
        }
    }
}

/// Draw a simple crosshair at screen center.
pub fn draw_crosshair(image: &mut CxImage, color: u8) {
    let w = image.width() as i32;
    let h = image.height() as i32;
    let cx = w / 2;
    let cy = h / 2;
    let data = image.data_mut();
    let size = 2;

    for d in -size..=size {
        // Horizontal arm.
        let hx = cx + d;
        if hx >= 0 && hx < w && d != 0 {
            data[(cy * w + hx) as usize] = color;
        }
        // Vertical arm.
        let vy = cy + d;
        if vy >= 0 && vy < h && d != 0 {
            data[(vy * w + cx) as usize] = color;
        }
    }
}

/// Draw a wall column with basic shading: shift palette index up by 1
/// (clamped to 255) for a darker appearance on Y-side walls.
///
/// This is a cheap side-based shading trick — horizontal walls get their
/// palette index incremented, which in most palettes produces a darker
/// or different variant.
fn draw_wall_column_shaded(
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
    let tex_w = texture.width() as i32;
    let strip_h = y_end - y_start;
    if strip_h <= 0 || tex_h == 0 || tex_x < 0 || tex_x >= tex_w {
        return;
    }

    let y_min = y_start.max(0);
    let y_max = y_end.min(img_h);
    let data = image.data_mut();
    let tex_data = texture.data();

    for y in y_min..y_max {
        let tex_y = ((y - y_start) * tex_h / strip_h).min(tex_h - 1);
        let pixel = tex_data[(tex_y * tex_w + tex_x) as usize];
        if pixel != 0 {
            // Darken Y-side walls via dither: ~25% of pixels shift to
            // fog_color (index 1) for subtle side shading.
            let threshold = BAYER_4X4[(y & 3) as usize][(x & 3) as usize];
            data[(y * img_w + x) as usize] = if threshold < 4 { 1 } else { pixel };
        }
    }
}

/// Create a procedural checkerboard wall texture.
///
/// `size` is both width and height. `block` is the checker square size in pixels.
#[must_use]
pub fn make_checker_texture(size: u32, block: u32, color_a: u8, color_b: u8) -> CxImage {
    assert!(size > 0, "checker texture size must be > 0");
    assert!(block > 0, "checker block size must be > 0");
    let mut data = vec![0u8; (size * size) as usize];
    for y in 0..size {
        for x in 0..size {
            let checker = ((x / block) + (y / block)).is_multiple_of(2);
            data[(y * size + x) as usize] = if checker { color_a } else { color_b };
        }
    }
    CxImage::new(data, size as usize)
}

/// Create a procedural brick-pattern wall texture.
#[must_use]
pub fn make_brick_texture(size: u32, brick_color: u8, mortar_color: u8) -> CxImage {
    assert!(size >= 4, "brick texture size must be >= 4");
    let mut data = vec![brick_color; (size * size) as usize];
    let brick_h = size / 4;
    let brick_w = size / 2;

    for y in 0..size {
        for x in 0..size {
            let row = y / brick_h;
            let offset = if row.is_multiple_of(2) {
                0
            } else {
                brick_w / 2
            };
            let local_x = (x + offset) % brick_w;
            let local_y = y % brick_h;

            // Mortar lines: bottom row of each brick and right edge.
            if local_y == brick_h - 1 || local_x == brick_w - 1 {
                data[(y * size + x) as usize] = mortar_color;
            }
        }
    }
    CxImage::new(data, size as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::test_map;
    use bevy_math::UVec2;

    #[test]
    fn render_produces_nonempty_image() {
        let map = test_map();
        let camera = FpCamera::default();
        let tex = make_checker_texture(16, 4, 1, 2);
        let mut image = CxImage::empty(UVec2::new(32, 24));

        render_fp_view(
            &mut image,
            &map,
            &camera,
            &[tex.clone(), tex],
            &FpPalette::default(),
        );

        // At least some pixels should be non-zero.
        assert!(image.data().iter().any(|&p| p != 0));
    }

    #[test]
    fn checker_texture_has_both_colors() {
        let tex = make_checker_texture(16, 4, 5, 7);
        let data = tex.data();
        assert!(data.contains(&5));
        assert!(data.contains(&7));
    }
}
