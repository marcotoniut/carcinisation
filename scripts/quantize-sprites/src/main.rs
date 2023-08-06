use std::collections::HashSet;
use std::path::Path;

use image::{ImageBuffer, Rgba};
use palette::color_difference::EuclideanDistance;
use palette::*;
use walkdir::WalkDir;

const ASSETS_PATH: &str = "../../assets/";
const BASE_PALETTE_SUBPATH: &str = "palette/base.png";
const RESOURCES_PATH: &str = "../../resources/";

fn main() {
    let paths = get_png_paths(RESOURCES_PATH);

    let inverted_palette_resources: HashSet<&str> =
        HashSet::from_iter(["sprites/ball_red_large.png"]);

    for path in paths {
        let invert = inverted_palette_resources.contains(path.as_str());
        reduce_colors(RESOURCES_PATH, &path, invert);
    }
}

fn reduce_colors(base_path: &str, path: &str, invert: bool) {
    let palette_image = image::open([ASSETS_PATH, BASE_PALETTE_SUBPATH].concat())
        .expect("could not open base palette")
        .to_rgba8();

    let input_image = image::open([base_path, path].concat())
        .expect(&format!("could not open '{}'", path))
        .to_rgba8();

    let palette = palette_image
        .enumerate_pixels()
        .map(|(_, _, pixel)| {
            let x: Srgba = Srgba::from(pixel.0).into_format();
            let y: Srgb = x.without_alpha();
            y
        })
        .collect::<Vec<Srgb>>();

    let mut pick_palette = palette.clone();
    if invert {
        pick_palette.reverse();
    }

    let (width, height) = input_image.dimensions();
    let mut output_image: ImageBuffer<image::Rgba<u8>, Vec<u8>> = ImageBuffer::new(width, height);

    for (x, y, input_color) in input_image.enumerate_pixels() {
        let input_srgba_color: Srgba = Srgba::from(input_color.0).into_format();
        if input_srgba_color.alpha != 0.0 {
            let input_srgb_color = input_srgba_color.without_alpha();

            let closest_color_index = find_closest_color(&palette, input_srgb_color);
            let closest_color = pick_palette[closest_color_index].with_alpha(1.0);

            let output_pixel: Rgba<u8> = image::Rgba(closest_color.into_format().into());

            output_image.put_pixel(x, y, output_pixel);
        }
    }

    output_image.save([ASSETS_PATH, path].concat()).unwrap();
}

fn find_closest_color(palette: &Vec<Srgb>, color: Srgb) -> usize {
    let mut closest_color_index = 0;
    let mut closest_color_distance = f32::MAX;

    for (index, palette_color) in palette.iter().enumerate() {
        let color_distance = palette_color.distance(color);

        if color_distance < closest_color_distance {
            closest_color_index = index;
            closest_color_distance = color_distance;
        }
    }

    closest_color_index
}

fn get_png_paths(base_path: &str) -> Vec<String> {
    let base_dir = Path::new(base_path);
    let ext = "png";

    let files: Vec<_> = WalkDir::new(base_dir)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some(ext) {
                path.strip_prefix(base_dir)
                    .ok()
                    .and_then(|p| p.to_str().map(|s| s.to_string()))
            } else {
                None
            }
        })
        .collect();

    files
}
