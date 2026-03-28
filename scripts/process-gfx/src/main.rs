//! Converts source graphics into game-ready textures with palette reduction.

extern crate image;

mod paths;
mod quantize;

use image::{ImageBuffer, Rgba, imageops};
use paths::{BASE_PALETTE_SUBPATH, assets_path, resources_gfx_path};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

use crate::quantize::reduce_colors;

/// Rescales an RGBA image to a target width while preserving aspect ratio.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn rescale_image(
    target_width: u32,
    img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (width, height) = img.dimensions();
    let divider: f32 = width as f32 / target_width as f32;
    let new_height = (height as f32 / divider) as u32;

    imageops::resize(img, target_width, new_height, imageops::FilterType::Nearest)
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

/// Loads the processing manifest and emits palette-reduced textures.
fn main() {
    let resources_gfx_path = resources_gfx_path();
    let assets_path = assets_path();
    let data_str = fs::read_to_string(resources_gfx_path.join("data.toml"))
        .expect("could not read resources/gfx/data.toml");

    let data: Config = toml::from_str(&data_str).unwrap();

    let palette_image: ImageBuffer<Rgba<u8>, Vec<u8>> =
        image::open(assets_path.join(BASE_PALETTE_SUBPATH))
            .expect("could not open base palette")
            .to_rgba8();

    for image in data.images {
        let asset_path = assets_path.join(image.target_path.unwrap_or(image.path.clone()));
        if let Some(parent_dir) = Path::new(&asset_path).parent() {
            fs::create_dir_all(parent_dir).expect("could not create directory");
        }

        println!("processing {}", asset_path.display());
        println!(
            "{} {}",
            image
                .width
                .map_or_else(|| "original".to_owned(), |w| format!("{w}w")),
            if image.invert_colors { "invert" } else { "" }
        );
        println!();

        let _ = match image::open(resources_gfx_path.join(&image.path))
            .map(image::DynamicImage::into_rgba8)
            .map(|img| {
                if let Some(width) = image.width {
                    rescale_image(width, &img)
                } else {
                    img.clone()
                }
            })
            .map(|img| reduce_colors(&palette_image, image.invert_colors, &img))
        {
            Ok(img) => Ok(img.save(asset_path)),
            Err(error) => {
                println!("Processing error on: {}", image.path);
                Err(error)
            }
        }
        .unwrap();
    }
}
