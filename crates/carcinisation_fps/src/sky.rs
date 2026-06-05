//! Doom-style scrolling sky for FPS mode.
//!
//! Renders a cylindrical panoramic sky that scrolls with player yaw.
//! Layers are stored as `.pxi` files (4bpp palette-indexed, deflate-compressed)
//! referenced by path in RON config.

use bevy::prelude::Resource;
use carapace::image::CxImage;
use flate2::bufread::DeflateDecoder;
use serde::Deserialize;
use std::io::{Cursor, Read};

#[derive(Deserialize)]
pub struct SkyData {
    pub layers: Vec<SkyLayerData>,
}

#[derive(Deserialize)]
pub struct SkyLayerData {
    pub name: String,
    pub path: String,
    pub width: u32,
    pub height: u32,
}

struct SkyLayer {
    indices: Vec<u8>,
    width: usize,
    height: usize,
}

impl SkyLayer {
    fn from_pxi(bytes: &[u8]) -> Self {
        let (width, height, indices) = decode_pxi(bytes).expect("sky .pxi must decode");
        Self {
            indices,
            width: width as usize,
            height: height as usize,
        }
    }

    fn sample(&self, u: f32, v: f32) -> u8 {
        let u_wrapped = u.rem_euclid(1.0);
        let v_clamped = v.clamp(0.0, 1.0);
        let x = (u_wrapped * self.width as f32).floor() as usize % self.width;
        let y = (v_clamped * (self.height - 1) as f32).floor() as usize;

        let pixel_index = y * self.width + x;
        if pixel_index >= self.indices.len() {
            return 0;
        }
        self.indices[pixel_index]
    }
}

/// Decode a PXI file into width, height, and palette indices.
fn decode_pxi(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    const HEADER_SIZE: usize = 10;
    if bytes.len() < HEADER_SIZE {
        return Err(format!("PXI file too short: {} bytes", bytes.len()));
    }
    if bytes[0..4] != asset_pipeline::pxi::MAGIC {
        return Err("PXI file has invalid magic".to_owned());
    }
    if bytes[4] != asset_pipeline::pxi::VERSION {
        return Err(format!("PXI version {} is unsupported", bytes[4]));
    }

    let width = u32::from(u16::from_le_bytes([bytes[6], bytes[7]]));
    let height = u32::from(u16::from_le_bytes([bytes[8], bytes[9]]));
    let pixel_count = (width * height) as usize;
    let expected_packed_len = pixel_count.div_ceil(2);
    let payload = &bytes[HEADER_SIZE..];
    let packed = match bytes[5] {
        asset_pipeline::pxi::FORMAT_RAW_4BPP => {
            if payload.len() != expected_packed_len {
                return Err(format!(
                    "PXI raw payload size {} != expected {expected_packed_len}",
                    payload.len(),
                ));
            }
            payload.to_vec()
        }
        asset_pipeline::pxi::FORMAT_DEFLATE_4BPP => {
            let mut inflated = Vec::with_capacity(expected_packed_len);
            let mut decoder = DeflateDecoder::new(Cursor::new(payload));
            decoder
                .read_to_end(&mut inflated)
                .map_err(|err| err.to_string())?;
            if inflated.len() != expected_packed_len {
                return Err(format!(
                    "PXI inflated payload size {} != expected {expected_packed_len}",
                    inflated.len(),
                ));
            }
            inflated
        }
        format => return Err(format!("PXI format {format} is unsupported")),
    };

    let mut indices = Vec::with_capacity(pixel_count);
    for byte in packed {
        indices.push(byte >> 4);
        indices.push(byte & 0x0f);
    }
    indices.truncate(pixel_count);

    Ok((width, height, indices))
}

#[derive(Resource)]
pub struct Sky {
    bg: SkyLayer,
    fg: SkyLayer,
}

impl Sky {
    #[must_use]
    pub fn from_ron(ron: &str, workspace_root: &str) -> Self {
        let data: SkyData = ron::from_str(ron).expect("sky RON config must parse");
        let bg_data = data
            .layers
            .iter()
            .find(|l| l.name == "clouds_fps_c")
            .expect("sky RON must have clouds_fps_c layer");
        let fg_data = data
            .layers
            .iter()
            .find(|l| l.name == "clouds_fps_f")
            .expect("sky RON must have clouds_fps_f layer");

        let bg_path = format!("{}/{}", workspace_root, bg_data.path);
        let fg_path = format!("{}/{}", workspace_root, fg_data.path);

        let bg_bytes = std::fs::read(&bg_path)
            .unwrap_or_else(|e| panic!("failed to read sky .pxi {bg_path}: {e}"));
        let fg_bytes = std::fs::read(&fg_path)
            .unwrap_or_else(|e| panic!("failed to read sky .pxi {fg_path}: {e}"));

        Self {
            bg: SkyLayer::from_pxi(&bg_bytes),
            fg: SkyLayer::from_pxi(&fg_bytes),
        }
    }

    #[must_use]
    pub const fn width(&self) -> usize {
        self.bg.width
    }

    #[must_use]
    pub const fn height(&self) -> usize {
        self.bg.height
    }

    /// Draw one column of sky, optionally scrolling vertically with aim pitch.
    ///
    /// `pitch_v_offset`: normalized vertical scroll from aim pitch (0.0 = neutral,
    /// positive = looking up, samples higher into texture). Applied with ease-out
    /// so the scroll slows near max look-up, preventing abrupt sky-edge clamping.
    pub fn draw_column(
        &self,
        image: &mut CxImage,
        x: i32,
        ceil_y: i32,
        ceiling_color: u8,
        yaw_offset: f32,
        pitch_v_offset: f32,
    ) {
        let img_w = image.width() as i32;
        let img_h = image.height() as i32;
        if x < 0 || x >= img_w {
            return;
        }

        let y_end = ceil_y.min(img_h).max(0);
        let sky_h = self.bg.height as f32;
        let bg_u = x as f32 / self.bg.width as f32 + yaw_offset;
        let fg_u = x as f32 / self.fg.width as f32 + yaw_offset;
        let data = image.data_mut();

        for y in 0..y_end {
            // Shift v-coordinate by pitch: positive pitch_v_offset moves the
            // sampling window upward into the texture (revealing higher sky content).
            let v = (y as f32 / sky_h) - pitch_v_offset;

            // Clamp to texture bounds instead of falling back to ceiling_color.
            // This repeats the top/bottom edge row, avoiding a hard color seam.
            let v_clamped = v.clamp(0.0, 1.0);

            let bg_idx = self.bg.sample(bg_u, v_clamped);
            if bg_idx > 0 {
                data[(y * img_w + x) as usize] = bg_idx;
                continue;
            }

            let fg_idx = self.fg.sample(fg_u, v_clamped);
            if fg_idx > 0 {
                data[(y * img_w + x) as usize] = fg_idx;
                continue;
            }

            data[(y * img_w + x) as usize] = ceiling_color;
        }
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args)]
mod tests {
    use super::*;
    use bevy_math::UVec2;

    const TEST_SKY_RON: &str = include_str!("../../../assets/config/sky/park.sky.ron");

    fn make_test_sky() -> Sky {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let root = format!("{}/../..", manifest);
        Sky::from_ron(TEST_SKY_RON, &root)
    }

    #[test]
    fn sky_loads_from_ron() {
        let sky = make_test_sky();
        assert!(sky.width() > 0);
        assert!(sky.height() > 0);
    }

    #[test]
    fn sky_width_is_at_least_screen_width() {
        let sky = make_test_sky();
        assert!(sky.width() >= 160);
    }

    #[test]
    fn sky_sampling_wraps_horizontally() {
        let sky = make_test_sky();
        assert_eq!(sky.bg.sample(0.0, 0.5), sky.bg.sample(1.0, 0.5));
        assert_eq!(sky.fg.sample(0.0, 0.5), sky.fg.sample(1.0, 0.5));
    }

    #[test]
    fn sky_sampling_clamps_vertically() {
        let sky = make_test_sky();
        assert_eq!(sky.bg.sample(0.5, -1.0), sky.bg.sample(0.5, 0.0));
        assert_eq!(sky.bg.sample(0.5, 2.0), sky.bg.sample(0.5, 1.0));
    }

    #[test]
    fn sky_column_produces_nonzero_pixels() {
        let sky = make_test_sky();
        let mut image = CxImage::empty(UVec2::new(160, 144));
        sky.draw_column(&mut image, 80, 72, 1, 0.0, 0.0);

        assert!(image.data().iter().any(|&p| p != 0));
        assert_eq!(image.data()[(100 * 160 + 80) as usize], 0);
    }

    #[test]
    fn sky_scroll_shifts_clouds() {
        let sky = make_test_sky();
        let mut image1 = CxImage::empty(UVec2::new(160, 72));
        let mut image2 = CxImage::empty(UVec2::new(160, 72));
        for x in 0..160 {
            sky.draw_column(&mut image1, x, 72, 1, 0.0, 0.0);
            sky.draw_column(&mut image2, x, 72, 1, 0.5, 0.0);
        }

        assert_ne!(image1.data(), image2.data());
    }

    #[test]
    fn parallax_comes_from_width_differential() {
        let sky = make_test_sky();
        assert!(sky.bg.width > sky.fg.width);
        assert_eq!(sky.bg.width, 295);
        assert_eq!(sky.fg.width, 160);
        let mut image1 = CxImage::empty(UVec2::new(160, 72));
        let mut image2 = CxImage::empty(UVec2::new(160, 72));
        for x in 0..160 {
            sky.draw_column(&mut image1, x, 72, 1, 0.0, 0.0);
            sky.draw_column(&mut image2, x, 72, 1, 1.0, 0.0);
        }
        let diff_count = image1
            .data()
            .iter()
            .zip(image2.data())
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            diff_count > 10,
            "Scrolling should shift many pixels, got {}",
            diff_count
        );
    }

    #[test]
    fn sky_pitch_scroll_shifts_vertically() {
        let sky = make_test_sky();
        let mut image_neutral = CxImage::empty(UVec2::new(160, 72));
        let mut image_up = CxImage::empty(UVec2::new(160, 72));
        for x in 0..160 {
            sky.draw_column(&mut image_neutral, x, 72, 1, 0.0, 0.0);
            sky.draw_column(&mut image_up, x, 72, 1, 0.0, 0.3);
        }
        // Pitch scroll should produce a visibly different sky.
        let diff_count = image_neutral
            .data()
            .iter()
            .zip(image_up.data())
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            diff_count > 10,
            "Vertical pitch scroll should shift sky pixels, got {diff_count}"
        );
    }

    #[test]
    fn sky_pitch_zero_matches_original() {
        let sky = make_test_sky();
        let mut image_a = CxImage::empty(UVec2::new(160, 72));
        let mut image_b = CxImage::empty(UVec2::new(160, 72));
        for x in 0..160 {
            sky.draw_column(&mut image_a, x, 72, 1, 0.0, 0.0);
            sky.draw_column(&mut image_b, x, 72, 1, 0.0, 0.0);
        }
        assert_eq!(image_a.data(), image_b.data());
    }
}
