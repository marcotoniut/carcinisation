extern crate image;
#[macro_use]
extern crate serde_derive;

mod paths;
mod quantize;

use image::*;
use paths::*;
use std::fs;

use crate::quantize::reduce_colors;

fn rescale_image(target_width: u32, img: &DynamicImage) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (width, height) = img.dimensions();
    let divider = width / target_width;
    let new_height = height / divider;

    return imageops::resize(img, target_width, new_height, imageops::FilterType::Nearest);
}

#[derive(Serialize, Deserialize)]
struct Image {
    path: String,
    #[serde(default)]
    invert_colors: bool,
    target_width: u32,
}

#[derive(Serialize, Deserialize)]
struct Config {
    images: Vec<Image>,
}

fn main() {
    let data_str = fs::read_to_string(format!("{}{}", RESOURCES_GFX_PATH, "data.toml")).unwrap();
    let data: Config = toml::from_str(&data_str).unwrap();

    let palette_image: ImageBuffer<Rgba<u8>, Vec<u8>> =
        image::open([ASSETS_PATH, BASE_PALETTE_SUBPATH].concat())
            .expect("could not open base palette")
            .to_rgba8();

    for image in data.images {
        println!("processing {}", image.path);
        println!(
            "{}w {}",
            image.target_width,
            if image.invert_colors { "invert" } else { "" }
        );
        println!();

        image::open(format!("{}{}", RESOURCES_GFX_PATH, image.path))
            .map(|img| rescale_image(image.target_width, &img))
            .map(|img| reduce_colors(&palette_image, image.invert_colors, &img))
            .and_then(|img| img.save(format!("{}{}", ASSETS_PATH, image.path)))
            .unwrap();
    }
}
