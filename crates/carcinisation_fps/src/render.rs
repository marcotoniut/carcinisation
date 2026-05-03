//! Render orchestration: produces a full-frame [`CxImage`] from map + camera.

use carapace::image::CxImage;

use crate::billboard::{Billboard, draw_billboard, project_billboard};
use crate::camera::Camera;
use crate::map::Map;
use crate::raycast::{HitSide, WallSurfaceId, cast_ray};
use crate::sky::Sky;

/// 4x4 Bayer ordered-dither threshold matrix (values 0..15).
pub(crate) const BAYER_4X4: [[u8; 4]; 4] =
    [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// Palette indices for ceiling and floor fills.
pub struct Palette {
    pub ceiling: u8,
    pub floor: u8,
    /// Palette index used as the fog/darkness color.
    pub fog_color: u8,
    /// Distance at which fog is fully opaque.
    pub fog_distance: f32,
    /// Distance below which no fog is applied.
    pub fog_start: f32,
}

impl Default for Palette {
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

#[derive(Clone, Copy, Debug)]
pub struct CharDecal {
    pub surface_id: WallSurfaceId,
    pub u: f32,
    pub v: f32,
    pub width: f32,
    pub height: f32,
    pub intensity: f32,
    pub flip_x: bool,
    pub flip_y: bool,
    pub seed: u32,
}

#[derive(Clone, Copy)]
pub struct WallSurfaceSprite<'a> {
    pub surface_id: WallSurfaceId,
    pub u: f32,
    pub v: f32,
    pub width: f32,
    pub height: f32,
    pub texture: &'a CxImage,
    pub flip_x: bool,
    pub flip_y: bool,
}

pub struct FpWallRenderEffects<'a> {
    pub char_decals: &'a [CharDecal],
    pub char_mask: Option<&'a CxImage>,
    pub surface_sprites: &'a [WallSurfaceSprite<'a>],
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
/// `wall_textures` is indexed by `wall_id - 1` (`wall_id` 0 is empty).
/// Image is cleared and fully redrawn.
/// If `sky` is provided, escaped ray columns render the sky instead of a solid ceiling.
pub fn render_fp_view(
    image: &mut CxImage,
    map: &Map,
    camera: &Camera,
    wall_textures: &[CxImage],
    palette: &Palette,
    sky: Option<&Sky>,
) {
    render_walls(image, map, camera, wall_textures, palette, None, None, sky);
}

/// Render walls + billboard entities into `image`.
///
/// Billboards are depth-sorted and drawn back-to-front with per-column
/// z-buffer occlusion against walls.
/// If `sky` is provided, escaped ray columns render the sky instead of a solid ceiling.
pub fn render_fp_scene(
    image: &mut CxImage,
    map: &Map,
    camera: &Camera,
    wall_textures: &[CxImage],
    palette: &Palette,
    billboards: &[Billboard],
    sky: Option<&Sky>,
) {
    let no_decals = [];
    let no_sprites = [];
    let effects = FpWallRenderEffects {
        char_decals: &no_decals,
        char_mask: None,
        surface_sprites: &no_sprites,
    };
    render_fp_scene_with_effects(
        image,
        map,
        camera,
        wall_textures,
        palette,
        billboards,
        &effects,
        sky,
    );
}

/// Render walls with wall-anchored effects + billboard entities.
/// If `sky` is provided, escaped ray columns render the sky instead of a solid ceiling.
pub fn render_fp_scene_with_effects(
    image: &mut CxImage,
    map: &Map,
    camera: &Camera,
    wall_textures: &[CxImage],
    palette: &Palette,
    billboards: &[Billboard],
    effects: &FpWallRenderEffects<'_>,
    sky: Option<&Sky>,
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
        Some(effects),
        sky,
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
/// If `sky` is provided, the ceiling area (above walls and in open columns)
/// renders the sky instead of a solid ceiling color.
fn render_walls(
    image: &mut CxImage,
    map: &Map,
    camera: &Camera,
    wall_textures: &[CxImage],
    palette: &Palette,
    mut zbuffer: Option<&mut [f32]>,
    effects: Option<&FpWallRenderEffects<'_>>,
    sky: Option<&Sky>,
) {
    let w = image.width() as i32;
    let h = image.height() as i32;

    image.clear();

    let dir = camera.direction();
    let plane = camera.plane();
    let half_h = h / 2;
    let yaw_offset = camera.angle / std::f32::consts::TAU;

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
            if let Some(sky_ref) = sky {
                sky_ref.draw_column(image, x, half_h, palette.ceiling, yaw_offset);
            } else {
                fill_column(image, x, 0, half_h, palette.ceiling);
            }
            fill_column(image, x, half_h, h, palette.floor);
            continue;
        }

        let line_height = (h as f32 / hit.distance) as i32;
        let draw_start = half_h - line_height / 2;
        let draw_end = draw_start + line_height;

        // Ceiling above wall — sky replaces ceiling color when available.
        if let Some(sky_ref) = sky {
            sky_ref.draw_column(image, x, draw_start.max(0), palette.ceiling, yaw_offset);
        } else {
            fill_column(image, x, 0, draw_start.max(0), palette.ceiling);
        }

        // Wall texture.
        let tex_idx = (hit.wall_id - 1) as usize;
        if tex_idx < wall_textures.len() {
            let tex = &wall_textures[tex_idx];
            let mut tex_x = (hit.wall_x * tex.width() as f32) as i32;
            tex_x = tex_x.clamp(0, tex.width() as i32 - 1);

            draw_wall_column_textured(
                image,
                x,
                draw_start,
                draw_end,
                tex,
                tex_x,
                hit.side == HitSide::Horizontal,
                hit.surface_id,
                hit.wall_x,
                effects,
            );
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

#[allow(clippy::too_many_arguments)]
fn draw_wall_column_textured(
    image: &mut CxImage,
    x: i32,
    y_start: i32,
    y_end: i32,
    texture: &CxImage,
    tex_x: i32,
    shaded: bool,
    surface_id: Option<WallSurfaceId>,
    wall_u: f32,
    effects: Option<&FpWallRenderEffects<'_>>,
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
        let threshold = BAYER_4X4[(y & 3) as usize][(x & 3) as usize];
        let mut out = (pixel != 0).then_some(if shaded && threshold < 4 { 1 } else { pixel });
        if let (Some(surface_id), Some(effects)) = (surface_id, effects) {
            let wall_v = (y - y_start) as f32 / strip_h as f32;
            if let Some(color) = sample_char_decals(
                effects.char_decals,
                effects.char_mask,
                surface_id,
                wall_u,
                wall_v,
                x,
                y,
            ) {
                out = Some(color);
            }
            if let Some(color) =
                sample_surface_sprites(effects.surface_sprites, surface_id, wall_u, wall_v)
            {
                out = Some(color);
            }
        }
        if let Some(out) = out {
            data[(y * img_w + x) as usize] = out;
        }
    }
}

fn sample_char_decals(
    decals: &[CharDecal],
    mask: Option<&CxImage>,
    surface_id: WallSurfaceId,
    wall_u: f32,
    wall_v: f32,
    screen_x: i32,
    screen_y: i32,
) -> Option<u8> {
    decals.iter().rev().find_map(|decal| {
        let mask = mask?;
        if decal.surface_id != surface_id {
            return None;
        }
        let sample = sample_mask(
            wall_u,
            wall_v,
            decal.u,
            decal.v,
            decal.width,
            decal.height,
            decal.flip_x,
            decal.flip_y,
            Some(mask),
        )?;
        let color = char_color(sample)?;
        let noise =
            ((screen_x as u32).wrapping_mul(17) ^ (screen_y as u32).wrapping_mul(31) ^ decal.seed)
                & 15;
        let density = (decal.intensity.clamp(0.0, 1.0) * 16.0) as u32;
        (density > noise).then_some(color)
    })
}

fn sample_surface_sprites(
    sprites: &[WallSurfaceSprite<'_>],
    surface_id: WallSurfaceId,
    wall_u: f32,
    wall_v: f32,
) -> Option<u8> {
    sprites.iter().rev().find_map(|sprite| {
        if sprite.surface_id != surface_id {
            return None;
        }
        let pixel = sample_mask(
            wall_u,
            wall_v,
            sprite.u,
            sprite.v,
            sprite.width,
            sprite.height,
            sprite.flip_x,
            sprite.flip_y,
            Some(sprite.texture),
        )?;
        if pixel == 0 {
            None
        } else {
            flame_hit_color(pixel)
        }
    })
}

fn flame_hit_color(mask_pixel: u8) -> Option<u8> {
    match mask_pixel {
        0 => None,
        2 => Some(2),
        4 => Some(4),
        other => Some(other),
    }
}

fn char_color(mask_pixel: u8) -> Option<u8> {
    match mask_pixel {
        0 => None,
        _ => Some(1),
    }
}

#[allow(clippy::too_many_arguments)]
fn sample_mask(
    wall_u: f32,
    wall_v: f32,
    center_u: f32,
    center_v: f32,
    width: f32,
    height: f32,
    flip_x: bool,
    flip_y: bool,
    texture: Option<&CxImage>,
) -> Option<u8> {
    let width = width.max(0.001);
    let height = height.max(0.001);
    let mut local_x = (wall_u - (center_u - width * 0.5)) / width;
    let mut local_y = (wall_v - (center_v - height * 0.5)) / height;
    if !(0.0..=1.0).contains(&local_x) || !(0.0..=1.0).contains(&local_y) {
        return None;
    }
    if flip_x {
        local_x = 1.0 - local_x;
    }
    if flip_y {
        local_y = 1.0 - local_y;
    }
    texture.map_or(Some(1), |texture| {
        let tex_w = texture.width() as i32;
        let tex_h = texture.height() as i32;
        if tex_w <= 0 || tex_h <= 0 {
            return None;
        }
        let tex_x = (local_x * tex_w as f32).floor() as i32;
        let tex_y = (local_y * tex_h as f32).floor() as i32;
        let tex_x = tex_x.clamp(0, tex_w - 1);
        let tex_y = tex_y.clamp(0, tex_h - 1);
        Some(texture.data()[(tex_y * tex_w + tex_x) as usize])
    })
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
    use crate::raycast::{HitSide, WallSurfaceId};
    use bevy_math::UVec2;

    #[test]
    fn render_produces_nonempty_image() {
        let map = test_map();
        let camera = Camera::default();
        let tex = make_checker_texture(16, 4, 1, 2);
        let mut image = CxImage::empty(UVec2::new(32, 24));

        render_fp_view(
            &mut image,
            &map,
            &camera,
            &[tex.clone(), tex],
            &Palette::default(),
            None,
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

    #[test]
    fn flame_wall_hit_keeps_secondary_mask_color() {
        let surface_id = WallSurfaceId {
            cell_x: 1,
            cell_y: 1,
            side: HitSide::Vertical,
            normal_sign: -1,
        };
        let texture = CxImage::new(vec![2, 4], 2);
        let sprite = WallSurfaceSprite {
            surface_id,
            u: 0.5,
            v: 0.5,
            width: 1.0,
            height: 1.0,
            texture: &texture,
            flip_x: false,
            flip_y: false,
        };

        assert_eq!(
            sample_surface_sprites(&[sprite], surface_id, 0.25, 0.5),
            Some(2)
        );
        assert_eq!(
            sample_surface_sprites(&[sprite], surface_id, 0.75, 0.5),
            Some(4)
        );
        assert_eq!(
            sample_surface_sprites(&[sprite], surface_id, 0.75, 1.1),
            None
        );
    }

    #[test]
    fn char_decal_filters_secondary_mask_color() {
        let surface_id = WallSurfaceId {
            cell_x: 1,
            cell_y: 1,
            side: HitSide::Vertical,
            normal_sign: -1,
        };
        let mask = CxImage::new(vec![2, 4], 2);
        let decal = CharDecal {
            surface_id,
            u: 0.5,
            v: 0.5,
            width: 1.0,
            height: 1.0,
            intensity: 1.0,
            flip_x: false,
            flip_y: false,
            seed: 0,
        };

        assert_eq!(
            sample_char_decals(&[decal], Some(&mask), surface_id, 0.25, 0.5, 0, 0),
            Some(1)
        );
        assert_eq!(
            sample_char_decals(&[decal], Some(&mask), surface_id, 0.75, 0.5, 0, 0),
            Some(1)
        );
    }

    #[test]
    fn wall_effects_draw_over_transparent_wall_pixels() {
        let surface_id = WallSurfaceId {
            cell_x: 1,
            cell_y: 1,
            side: HitSide::Vertical,
            normal_sign: -1,
        };
        let wall_texture = CxImage::new(vec![0; 16], 4);
        let effect_texture = CxImage::new(vec![2], 1);
        let sprite = WallSurfaceSprite {
            surface_id,
            u: 0.5,
            v: 0.5,
            width: 1.0,
            height: 1.0,
            texture: &effect_texture,
            flip_x: false,
            flip_y: false,
        };
        let sprites = [sprite];
        let effects = FpWallRenderEffects {
            char_decals: &[],
            char_mask: None,
            surface_sprites: &sprites,
        };
        let mut image = CxImage::empty(UVec2::new(4, 4));

        draw_wall_column_textured(
            &mut image,
            2,
            0,
            4,
            &wall_texture,
            2,
            false,
            Some(surface_id),
            0.5,
            Some(&effects),
        );

        assert_eq!(image.data()[2 + 2 * image.width()], 2);
    }
}
