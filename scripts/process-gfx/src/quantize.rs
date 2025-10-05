use image::*;
use palette::color_difference::EuclideanDistance;
use palette::*;

fn find_closest_color(palette: &[Srgb], color: Srgb) -> usize {
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

pub fn reduce_colors(
    palette_image: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    invert: bool,
    img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let palette = palette_image
        .enumerate_pixels()
        .map(|(_, _, pixel)| {
            let x: Srgba = Srgba::from(pixel.0).into_format();
            let y: Srgb = x.without_alpha();
            y
        })
        .collect::<Vec<Srgb>>();

    let (width, height) = img.dimensions();
    let mut output_image: ImageBuffer<image::Rgba<u8>, Vec<u8>> = ImageBuffer::new(width, height);

    for (x, y, input_color) in img.enumerate_pixels() {
        let input_srgba_color: Srgba = Srgba::from(input_color.0).into_format();
        if input_srgba_color.alpha != 0.0 {
            let input_srgb_color = input_srgba_color.without_alpha();

            let closest_color_index = find_closest_color(&palette, input_srgb_color);
            let adjusted_index = if invert {
                palette.len() - 1 - closest_color_index
            } else {
                closest_color_index
            };
            let closest_color = palette[adjusted_index].with_alpha(1.0);

            let output_pixel: Rgba<u8> = image::Rgba(closest_color.into_format().into());

            output_image.put_pixel(x, y, output_pixel);
        }
    }

    output_image
}
