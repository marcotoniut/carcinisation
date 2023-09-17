use std::{fs, path::Path};

use image::{ImageBuffer, Rgb};

const PALETTES_PATH: &str = "../../assets/palette/";
const FILTER_PATH: &str = "../../assets/filter/";

fn main() {
    let palettes: Vec<(&str, [Rgb<u8>; 4])> = vec![
        (
            "base",
            [
                // #471226
                Rgb([39, 25, 54]),
                // #99340b
                Rgb([153, 52, 11]),
                // #aa593a
                Rgb([170, 89, 58]),
                // #f6d69c
                Rgb([246, 214, 156]),
            ],
        ),
        (
            "alt",
            [
                // #450000
                Rgb([69, 0, 0]),
                // #914f2c
                Rgb([145, 79, 44]),
                // #a26e79
                Rgb([162, 110, 121]),
                // #d0e0b4
                Rgb([208, 224, 180]),
            ],
        ),
        (
            "gb",
            [
                // #081820
                Rgb([8, 24, 32]),
                // #346856
                Rgb([52, 104, 86]),
                // #88C070
                Rgb([136, 192, 112]),
                // #E0F8D0
                Rgb([224, 248, 208]),
            ],
        ),
        (
            "rust",
            [
                // #442434
                Rgb([39, 25, 54]),
                // #6D3C4D
                Rgb([124, 55, 25]),
                // #B8430F
                Rgb([184, 67, 15]),
                // #d2a07f
                Rgb([210, 160, 127]),
            ],
        ),
    ];

    fs::create_dir_all(Path::new(&PALETTES_PATH)).expect("could not create directory");

    for (key, palette) in palettes.iter() {
        let mut output_image: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::new(palette.len() as u32, 1);

        for ((x, y, _), pixel) in output_image.clone().enumerate_pixels().zip(palette) {
            output_image.put_pixel(x, y, pixel.clone());
        }
        output_image
            .save([PALETTES_PATH, &format!("{}.png", *key)].concat())
            .unwrap();

        if *key == "base" {
            fs::create_dir_all(&FILTER_PATH).expect("could not create directory");
            for (i, color) in palette.iter().enumerate() {
                let mut color_image: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(4, 1);
                for x in 0..4 {
                    color_image.put_pixel(x, 0, color.clone());
                }
                color_image
                    .save([FILTER_PATH, &format!("color{}.png", i)].concat())
                    .unwrap();
            }
        }
    }
}
