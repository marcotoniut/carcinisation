extern crate image;
extern crate rusttype;

use image::{Rgba, RgbaImage};
use rusttype::{point, Font, Scale};

const RESOURCES_PATH: &str = "../../resources/";

fn generate_image(font_path: &str, target_height: u32, characters: &[char]) -> RgbaImage {
    let font_data = std::fs::read(font_path).unwrap();
    let font = Font::try_from_vec(font_data).unwrap();

    let scale = Scale {
        x: target_height as f32,
        y: target_height as f32,
    };

    let width = characters
        .iter()
        .map(|c| {
            let glyph = font.glyph(*c).scaled(scale);
            glyph.h_metrics().advance_width.round() as u32
        })
        .max()
        .unwrap_or(0);
    let height = target_height * characters.len() as u32;

    let mut image = RgbaImage::new(width, height);

    let mut y = 0;
    for c in characters {
        let glyph = font
            .glyph(*c)
            .scaled(scale)
            .positioned(point(0.0, y as f32 + scale.y));
        glyph.draw(|gx, gy, gv| {
            let gx = gx as i32;
            let gy = gy as i32;
            if gx >= 0 && gy >= 0 {
                let pixel = image.get_pixel_mut(gx as u32, gy as u32 + y);
                *pixel = Rgba([255, 255, 255, if gv < 0.25 { 0 } else { 255 }]);
            }
        });
        y += target_height;
    }

    image
}

fn main() {
    let target_height = 10;
    let binding = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        // let binding = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[{]}\\|;:'\",<.>/?"
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
        .save(format!("{}{}", RESOURCES_PATH, "gfx/typeface/pixeboy.png").as_str())
        .unwrap();
}
