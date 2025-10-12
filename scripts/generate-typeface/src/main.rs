//! Rasterises the Pixeboy typeface into a bitmap atlas for the HUD.

extern crate image;
extern crate rusttype;

use image::{Rgba, RgbaImage};
use rusttype::{point, Font, Scale};

const RESOURCES_PATH: &str = "../../resources/";

/// Renders each glyph to a vertically stacked bitmap strip.
fn generate_image(font_path: &str, target_height: u32, characters: &[char]) -> RgbaImage {
    let font_data = std::fs::read(font_path).unwrap();
    let font = Font::try_from_vec(font_data).unwrap();

    let scale = Scale::uniform(target_height as f32);
    let v_metrics = font.v_metrics(scale);

    let max_advance_width = characters
        .iter()
        .map(|c| {
            let glyph = font.glyph(*c).scaled(scale);
            glyph.h_metrics().advance_width.round() as u32
        })
        .max()
        .unwrap_or(0);

    let width = max_advance_width;
    let height = target_height * characters.len() as u32;

    let mut image = RgbaImage::new(width, height);

    let mut y = 0;

    for c in characters.iter() {
        let glyph = font.glyph(*c).scaled(scale);
        let h_metrics = glyph.h_metrics();

        let glyph_x = ((max_advance_width - h_metrics.advance_width.round() as u32) / 2) as i32;

        let glyph_y =
            y as i32 + (target_height as f32 - v_metrics.ascent + v_metrics.descent) as i32;

        let glyph = glyph.positioned(point(glyph_x as f32, glyph_y as f32));

        let pixel_bounding_box = glyph.pixel_bounding_box().unwrap();

        let offset_y = (scale.y as i32 + pixel_bounding_box.min.y) as i32;
        let offset_x =
            ((scale.x - (pixel_bounding_box.max.x - pixel_bounding_box.min.x) as f32) / 2.0) as i32;

        glyph.draw(|gx, gy, gv| {
            let gx = gx as i32 + offset_x;
            let gy = gy as i32 + offset_y;
            if gx >= 0 && gy >= 0 && gy < height as i32 {
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
