extern crate image;
#[macro_use]
extern crate serde_derive;

mod paths;
mod quantize;

use image::*;
use paths::*;
use std::{fs, path::Path};

use crate::quantize::reduce_colors;

fn rescale_image(
    target_width: u32,
    img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (width, height) = img.dimensions();
    let divider = width / target_width;
    let new_height = height / divider;

    return imageops::resize(img, target_width, new_height, imageops::FilterType::Nearest);
}

#[derive(Serialize, Deserialize)]
struct Image {
    #[serde(default)]
    invert_colors: bool,
    path: String,
    target_path: Option<String>,
    #[serde(default)]
    width: Option<u32>,
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
        let asset_path = format!(
            "{}{}",
            ASSETS_PATH,
            image.target_path.unwrap_or(image.path.clone())
        );
        if let Some(parent_dir) = Path::new(&asset_path).parent() {
            fs::create_dir_all(parent_dir).expect("could not create directory");
        }

        println!("processing {}", asset_path);
        println!(
            "{} {}",
            image
                .width
                .map_or_else(|| "original".to_string(), |w| format!("{}w", w.to_string())),
            if image.invert_colors { "invert" } else { "" }
        );
        println!();

        image::open(format!("{}{}", RESOURCES_GFX_PATH, image.path))
            .map(|img| img.into_rgba8())
            .map(|img| {
                if let Some(width) = image.width {
                    rescale_image(width, &img)
                } else {
                    img.to_owned()
                }
            })
            .map(|img| reduce_colors(&palette_image, image.invert_colors, &img))
            .and_then(|img| img.save(asset_path))
            .unwrap();
    }
}
