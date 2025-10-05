extern crate image;

mod paths;
mod quantize;

use image::*;
use paths::*;
use serde_derive::*;
use std::{fs, path::Path};

use crate::quantize::reduce_colors;

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

fn main() {
    let data_str = if cfg!(windows) {
        println!("this is windows");
        let mut root = std::env::current_dir().unwrap();
        root.push(RESOURCES_GFX_PATH);
        println!("{}", root.display());

        let win_path = root.join("data.toml");
        println!("WINDOWS PATH: {}", win_path.display());

        fs::read_to_string(&win_path).unwrap()
    } else {
        fs::read_to_string(format!("{RESOURCES_GFX_PATH}data.toml")).unwrap()
    };

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
                .map_or_else(|| "original".to_owned(), |w| format!("{w}w")),
            if image.invert_colors { "invert" } else { "" }
        );
        println!();

        let _ = match image::open(format!("{}{}", RESOURCES_GFX_PATH, image.path))
            .map(|img| img.into_rgba8())
            .map(|img| {
                if let Some(width) = image.width {
                    rescale_image(width, &img)
                } else {
                    img.to_owned()
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
