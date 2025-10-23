//! Generates the core palette textures and filter assets used by the game.

use std::fs;

use image::{ImageBuffer, Rgba};

const PALETTES_PATH: &str = "../../assets/palette/";
const FILTER_PATH: &str = "../../assets/filter/";

/// Builds per-palette PNGs plus convenience filter textures.
fn main() {
    let palettes: Vec<(&str, [Rgba<u8>; 4])> = vec![
        (
            "base",
            [
                // #421032
                Rgba([66, 16, 50, 255]),
                // #91300a
                Rgba([145, 48, 10, 255]),
                // #b96740
                Rgba([185, 103, 64, 255]),
                // #fbe1a8
                Rgba([251, 225, 168, 255]),
            ],
        ),
        (
            "alt",
            [
                // #450000
                Rgba([69, 0, 0, 255]),
                // #914f2c
                Rgba([145, 79, 44, 255]),
                // #a26e79
                Rgba([162, 110, 121, 255]),
                // #d0e0b4
                Rgba([208, 224, 180, 255]),
            ],
        ),
        (
            "gb",
            [
                // #081820
                Rgba([8, 24, 32, 255]),
                // #346856
                Rgba([52, 104, 86, 255]),
                // #88C070
                Rgba([136, 192, 112, 255]),
                // #E0F8D0
                Rgba([224, 248, 208, 255]),
            ],
        ),
        (
            "rust",
            [
                // #442434
                Rgba([39, 25, 54, 255]),
                // #6D3C4D
                Rgba([124, 55, 25, 255]),
                // #B8430F
                Rgba([184, 67, 15, 255]),
                // #d2a07f
                Rgba([210, 160, 127, 255]),
            ],
        ),
    ];

    fs::create_dir_all(PALETTES_PATH).expect("could not create directory");

    for (key, palette) in palettes.iter() {
        // Palette needs 5 pixels: 1 transparent + 4 colors
        let mut output_palette_image: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::new(palette.len() as u32 + 1, 1);

        // First pixel is transparent
        output_palette_image.put_pixel(0, 0, Rgba([0, 0, 0, 0]));

        // Remaining pixels are the palette colors
        for (i, color) in palette.iter().enumerate() {
            output_palette_image.put_pixel(i as u32 + 1, 0, *color);
        }

        output_palette_image
            .save(format!("{PALETTES_PATH}{key}.png"))
            .unwrap();

        let mut output_invert_palette: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::new(palette.len() as u32 + 1, 1);

        if *key == "base" {
            // The base palette also drives editor filters, so build per-color swatches
            // and an inverted gradient for shader uniforms.
            fs::create_dir_all(FILTER_PATH).expect("could not create directory");
            let frame_width = palette.len() as u32 + 1;

            for (i, color) in palette.iter().enumerate() {
                let mut color_image: ImageBuffer<Rgba<u8>, Vec<u8>> =
                    ImageBuffer::new(frame_width, 1);

                // Preserve palette index 0 as transparent
                color_image.put_pixel(0, 0, Rgba([0, 0, 0, 0]));

                for x in 1..frame_width {
                    color_image.put_pixel(x, 0, *color);
                }
                color_image
                    .save(format!("{FILTER_PATH}color{i}.px_filter.png"))
                    .unwrap();
            }

            // First pixel is transparent
            output_invert_palette.put_pixel(0, 0, Rgba([0, 0, 0, 0]));

            // Remaining pixels are the inverted palette colors
            let mut palette_invert = *palette;
            palette_invert.reverse();
            for (i, color) in palette_invert.iter().enumerate() {
                output_invert_palette.put_pixel(i as u32 + 1, 0, *color);
            }
            output_invert_palette
                .save(format!("{FILTER_PATH}invert.px_filter.png"))
                .unwrap();
        }
    }
}
