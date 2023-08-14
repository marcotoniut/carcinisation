use image::{ImageBuffer, Rgb};

const PALETTES_PATH: &str = "../../assets/palette/";

fn main() {
    let palettes: Vec<(&str, [Rgb<u8>; 4])> = vec![
        (
            "base",
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
                // #3b190e
                Rgb([59, 25, 14]),
                // #903A19
                Rgb([144, 58, 25]),
                // #B8430F
                Rgb([184, 67, 15]),
                // #d2a07f
                Rgb([210, 160, 127]),
            ],
        ),
    ];

    for (key, palette) in palettes.iter() {
        let mut output_image: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::new(palette.len() as u32, 1);

        for ((x, y, _), pixel) in output_image.clone().enumerate_pixels().zip(palette) {
            output_image.put_pixel(x, y, pixel.clone());
        }
        output_image
            .save([PALETTES_PATH, &format!("{}.png", *key)].concat())
            .unwrap();
    }
}
