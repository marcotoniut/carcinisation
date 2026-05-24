//! Rasterises the Pixeboy typeface into a bitmap atlas for the HUD.

extern crate ab_glyph;
extern crate image;

use ab_glyph::{Font, FontArc, ScaleFont, point};
use image::{Rgba, RgbaImage};

const RESOURCES_PATH: &str = "../../resources/";

/// Renders each glyph to a vertically stacked bitmap strip.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]
fn generate_image(font_path: &str, target_height: u32, characters: &[char]) -> RgbaImage {
    let font_data = std::fs::read(font_path).unwrap();
    let font = FontArc::try_from_vec(font_data).unwrap();

    let scale = target_height as f32;
    let scaled_font = font.as_scaled(scale);

    let max_advance_width = characters
        .iter()
        .map(|c| {
            let glyph_id = scaled_font.glyph_id(*c);
            scaled_font.h_advance(glyph_id).round() as u32
        })
        .max()
        .unwrap_or(0);

    let width = max_advance_width;
    let height = target_height * characters.len() as u32;

    let mut image = RgbaImage::new(width, height);

    let mut y = 0;

    for c in characters {
        let glyph_id = scaled_font.glyph_id(*c);
        let advance_width = scaled_font.h_advance(glyph_id).round() as u32;

        let glyph_x = ((max_advance_width - advance_width) / 2) as i32;

        let glyph_y =
            y as i32 + (target_height as f32 - scaled_font.ascent() + scaled_font.descent()) as i32;

        let glyph = glyph_id.with_scale_and_position(scale, point(glyph_x as f32, glyph_y as f32));

        let Some(glyph) = font.outline_glyph(glyph) else {
            // Empty glyph (e.g. space) — skip rendering, advance to next row.
            y += target_height;
            continue;
        };

        let pixel_bounding_box = glyph.px_bounds();
        let offset_y = scale as i32 + pixel_bounding_box.min.y.round() as i32;
        let offset_x = ((scale - pixel_bounding_box.width()) / 2.0) as i32;

        glyph.draw(|gx, gy, gv| {
            let gx = gx as i32 + offset_x;
            let gy = gy as i32 + offset_y;
            if gx >= 0 && (gx as u32) < width && gy >= 0 && gy < height as i32 {
                let pixel = image.get_pixel_mut(gx as u32, gy as u32);
                *pixel = Rgba([255, 255, 255, if gv < 0.25 { 0 } else { 255 }]);
            }
        });

        y += target_height;
    }

    image
}

/// Writes the HUD typeface atlas to `resources/gfx/typeface`.
fn main() {
    let target_height = 10;
    let binding = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[{]}\\|;:'\",<.>/?"
        .chars()
        .rev()
        .collect::<Vec<_>>();
    let characters = binding.as_slice();

    let image = generate_image(
        format!("{}{}", RESOURCES_PATH, "fonts/Pixeboy.ttf").as_str(),
        target_height,
        characters,
    );

    image
        .save(
            format!(
                "{}{}",
                RESOURCES_PATH, "gfx/typeface/pixeboy.px_typeface.png"
            )
            .as_str(),
        )
        .unwrap();
}
