use image::{ImageBuffer, Rgb};

const PALETTES_PATH: &str = "../../assets/palette/";

fn main() {
    // let palette: [Rgb<u8>; 4] = [
    //     Rgb([15, 56, 15]),
    //     Rgb([48, 98, 48]),
    //     Rgb([139, 172, 15]),
    //     Rgb([155, 188, 15]),
    // ];
    let palette: [Rgb<u8>; 4] = [
        Rgb([8, 24, 32]),
        Rgb([52, 104, 86]),
        Rgb([136, 192, 112]),
        Rgb([224, 248, 208]),
    ];

    let mut output_image: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(palette.len() as u32, 1);

    for ((x, y, _), pixel) in output_image.clone().enumerate_pixels().zip(palette) {
        output_image.put_pixel(x, y, pixel);
    }

    output_image
        .save([PALETTES_PATH, "base.png"].concat())
        .unwrap();
}
