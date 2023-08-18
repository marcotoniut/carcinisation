extern crate image;
#[macro_use]
extern crate serde_derive;

use image::*;
use std::fs;

const ASSETS_PATH: &str = "../../assets/";
const RESOURCES_GFX_PATH: &str = "../../resources/gfx/";

fn rescale_image(img: &DynamicImage, target_width: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
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

    for image in data.images {
        println!("processing {}", image.path);
        println!(
            "{}w {}",
            image.target_width,
            if image.invert_colors { "invert" } else { "" }
        );
        println!();

        let img = image::open(format!("{}{}", RESOURCES_GFX_PATH, image.path)).unwrap();

        let new_img = rescale_image(&img, image.target_width);

        new_img
            .save(format!("{}{}", ASSETS_PATH, image.path))
            .unwrap();
    }
}
