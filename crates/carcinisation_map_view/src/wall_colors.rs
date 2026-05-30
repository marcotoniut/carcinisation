//! Wall colour extraction from texture images.
//!
//! Extracts the two most common non-zero palette indices from each wall
//! texture for use in dithered map-view rendering.

use carapace::image::CxImage;

/// Extract the two most common non-zero palette indices from a wall texture.
///
/// Returns `(primary, secondary)`. If only one colour exists, `secondary`
/// falls back to palette index 1 (darkest). Used by the map view renderer
/// to dither walls so they remain distinguishable from the floor.
#[must_use]
pub fn dominant_pair(texture: &CxImage) -> (u8, u8) {
    let mut counts = [0u32; 256];
    for &p in texture.data() {
        if p != 0 {
            counts[p as usize] += 1;
        }
    }
    let mut best = (0u8, 0u32);
    let mut second = (0u8, 0u32);
    for (i, &c) in counts.iter().enumerate().skip(1) {
        if c > best.1 {
            second = best;
            best = (i as u8, c);
        } else if c > second.1 {
            second = (i as u8, c);
        }
    }
    let primary = if best.1 > 0 { best.0 } else { 1 };
    let secondary = if second.1 > 0 { second.0 } else { 1 };
    (primary, secondary)
}

/// Extract wall colour pairs (primary, secondary) for dithered rendering.
pub fn wall_color_pairs_from_textures(textures: &[CxImage]) -> Vec<(u8, u8)> {
    textures.iter().map(dominant_pair).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_image(colors: &[u8]) -> CxImage {
        CxImage::new(colors.to_vec(), 4)
    }

    #[test]
    fn transparent_texture_falls_back_to_1_1() {
        let tex = make_test_image(&[0, 0, 0, 0]);
        assert_eq!(dominant_pair(&tex), (1, 1));
    }

    #[test]
    fn single_colour_texture_secondary_falls_back() {
        let tex = make_test_image(&[3, 3, 3, 3, 0, 0, 0, 0]);
        assert_eq!(dominant_pair(&tex), (3, 1));
    }

    #[test]
    fn two_colour_texture_picks_both() {
        let tex = make_test_image(&[3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 0, 0, 0, 0]);
        let (primary, secondary) = dominant_pair(&tex);
        assert_eq!(primary, 3);
        assert_eq!(secondary, 2);
    }

    #[test]
    fn three_colour_picks_top_two() {
        let tex = make_test_image(&[4, 4, 4, 4, 4, 4, 4, 4, 2, 2, 2, 2, 1, 1, 0, 0]);
        let (primary, secondary) = dominant_pair(&tex);
        assert_eq!(primary, 4);
        assert_eq!(secondary, 2);
    }

    #[test]
    fn pairs_from_textures_maps_each() {
        let textures = vec![
            make_test_image(&[3, 3, 3, 3, 2, 2, 0, 0]),
            make_test_image(&[4, 4, 4, 4, 0, 0, 0, 0]),
        ];
        let pairs = wall_color_pairs_from_textures(&textures);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], (3, 2));
        assert_eq!(pairs[1], (4, 1));
    }
}
