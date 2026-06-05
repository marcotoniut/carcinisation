//! Fire death mechanics: damage types, flame config, and perimeter flame placement.

use bevy_math::Vec2;
use std::num::NonZeroUsize;

use crate::hash_util::{signed_unit, unit};

/// Damage type applied to an entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageKind {
    Physical,
    Fire,
}

/// Config for burning corpse visual effect.
#[derive(Clone, Copy, Debug)]
pub struct FireDeathConfig {
    pub burning_corpse_duration_secs: f32,
    pub burning_flame_count: NonZeroUsize,
    pub burning_flame_perimeter_padding_px: f32,
    pub burning_flame_jitter_px: f32,
    pub burning_flame_scale_min: f32,
    pub burning_flame_scale_max: f32,
}

impl Default for FireDeathConfig {
    fn default() -> Self {
        Self {
            burning_corpse_duration_secs: 1.25,
            burning_flame_count: NonZeroUsize::new(8).unwrap(),
            burning_flame_perimeter_padding_px: 2.0,
            burning_flame_jitter_px: 1.0,
            burning_flame_scale_min: 0.8,
            burning_flame_scale_max: 1.2,
        }
    }
}

/// A flame particle placed along a burning corpse perimeter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PerimeterFlame {
    pub offset_px: Vec2,
    pub scale: f32,
    pub phase_secs: f32,
    pub front: bool,
}

/// Generate deterministic seed from world position for flame placement.
#[must_use]
pub fn corpse_seed(position: Vec2) -> u32 {
    let x = (position.x * 1024.0).round().to_bits();
    let y = (position.y * 1024.0).round().to_bits();
    x.wrapping_mul(0x9E37_79B9) ^ y.wrapping_mul(0x85EB_CA6B) ^ 0xC2B2_AE35
}

/// Generate flame particles along sprite mask perimeter.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn perimeter_flames_from_mask(
    seed: u32,
    width: usize,
    height: usize,
    opaque: impl Fn(usize, usize) -> bool,
    config: &FireDeathConfig,
) -> Vec<PerimeterFlame> {
    let density_px = 10.0;
    let jitter_px = config.burning_flame_jitter_px.max(0.0);

    let mut perimeter = Vec::new();
    for y in 0..height {
        for x in 0..width {
            if !opaque(x, y) {
                continue;
            }
            let is_edge = x == 0
                || y == 0
                || x + 1 >= width
                || y + 1 >= height
                || !opaque(x - 1, y)
                || !opaque(x + 1, y)
                || !opaque(x, y - 1)
                || !opaque(x, y + 1);
            if is_edge {
                perimeter.push(Vec2::new(
                    (width as f32).mul_add(-0.5, x as f32),
                    (height as f32).mul_add(0.5, -(y as f32)),
                ));
            }
        }
    }
    if perimeter.is_empty() {
        return Vec::new();
    }

    let count = config
        .burning_flame_count
        .get()
        .max((perimeter.len() as f32 / density_px).round() as usize)
        .min(perimeter.len());
    let scale_min = config
        .burning_flame_scale_min
        .min(config.burning_flame_scale_max);
    let scale_max = config
        .burning_flame_scale_min
        .max(config.burning_flame_scale_max);

    (0..count)
        .map(|i| {
            let index = i * perimeter.len() / count;
            let mixed = seed ^ (i as u32).wrapping_mul(0x9E37_79B9);
            let jitter = Vec2::new(
                signed_unit(mixed.rotate_left(5)),
                signed_unit(mixed.rotate_left(13)),
            ) * jitter_px;
            let scale = (scale_max - scale_min).mul_add(unit(mixed.rotate_left(21)), scale_min);
            let phase_secs = unit(mixed.rotate_left(7)) * 0.3;
            PerimeterFlame {
                offset_px: perimeter[index] + jitter,
                scale,
                phase_secs,
                front: unit(mixed.rotate_left(3)) > 0.7,
            }
        })
        .collect()
}

/// Generate flame positions clustered toward the center of a sprite mask.
/// Used for alive burning enemies (as opposed to perimeter flames for corpses).
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn centered_flames_from_mask(
    seed: u32,
    width: usize,
    height: usize,
    opaque: impl Fn(usize, usize) -> bool,
    count: usize,
) -> Vec<PerimeterFlame> {
    if count == 0 {
        return Vec::new();
    }

    // Collect all opaque pixels.
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            if opaque(x, y) {
                pixels.push(Vec2::new(
                    (width as f32).mul_add(-0.5, x as f32),
                    (height as f32).mul_add(0.5, -(y as f32)),
                ));
            }
        }
    }
    if pixels.is_empty() {
        return Vec::new();
    }

    // Center of mass.
    let center = pixels.iter().copied().sum::<Vec2>() / pixels.len() as f32;

    (0..count)
        .map(|i| {
            let mixed = seed ^ (i as u32).wrapping_mul(0x9E37_79B9);
            // Pick a pixel, biased toward center: blend between random pixel and center.
            let pixel_idx =
                (unit(mixed.rotate_left(3)) * pixels.len() as f32) as usize % pixels.len();
            let bias = 0.2 + unit(mixed.rotate_left(11)) * 0.3; // 0.2–0.5 toward center
            let pos = pixels[pixel_idx].lerp(center, bias);
            let scale = 0.8 + unit(mixed.rotate_left(21)) * 0.4;
            let phase_secs = unit(mixed.rotate_left(7)) * 0.3;
            PerimeterFlame {
                offset_px: pos,
                scale,
                phase_secs,
                front: unit(mixed.rotate_left(17)) > 0.5,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // corpse_seed
    // -----------------------------------------------------------------------

    #[test]
    fn corpse_seed_deterministic() {
        let a = corpse_seed(Vec2::new(3.5, 7.25));
        let b = corpse_seed(Vec2::new(3.5, 7.25));
        assert_eq!(a, b);
    }

    #[test]
    fn corpse_seed_different_positions_differ() {
        let a = corpse_seed(Vec2::new(1.0, 2.0));
        let b = corpse_seed(Vec2::new(2.0, 1.0));
        assert_ne!(a, b);
    }

    #[test]
    fn corpse_seed_near_positions_differ() {
        // Resolution is 1/1024: delta of 0.001 rounds to 1 bit difference.
        // Smaller deltas (e.g. 0.0004) would hash identically.
        let a = corpse_seed(Vec2::new(1.0, 1.0));
        let b = corpse_seed(Vec2::new(1.001, 1.0));
        assert_ne!(a, b);
    }

    // -----------------------------------------------------------------------
    // centered_flames_from_mask
    // -----------------------------------------------------------------------

    fn solid_rect(w: usize, h: usize) -> impl Fn(usize, usize) -> bool {
        move |x, y| x < w && y < h
    }

    #[test]
    fn centered_flames_zero_count_returns_empty() {
        let flames = centered_flames_from_mask(42, 10, 10, solid_rect(10, 10), 0);
        assert!(flames.is_empty());
    }

    #[test]
    fn centered_flames_no_opaque_returns_empty() {
        let flames = centered_flames_from_mask(42, 10, 10, |_, _| false, 5);
        assert!(flames.is_empty());
    }

    #[test]
    fn centered_flames_deterministic() {
        let a = centered_flames_from_mask(99, 10, 10, solid_rect(10, 10), 5);
        let b = centered_flames_from_mask(99, 10, 10, solid_rect(10, 10), 5);
        assert_eq!(a, b);
    }

    #[test]
    fn centered_flames_count_matches_requested() {
        let flames = centered_flames_from_mask(42, 10, 10, solid_rect(10, 10), 7);
        assert_eq!(flames.len(), 7);
    }

    #[test]
    fn centered_flames_biased_toward_center() {
        let w = 20;
        let h = 20;
        let flames = centered_flames_from_mask(42, w, h, solid_rect(w, h), 50);
        // Center of a 20×20 rect centered at origin is (0, 0).
        // With center bias (0.2–0.5 lerp), mean distance from center
        // should be well under half the sprite extent.
        let mean_dist: f32 =
            flames.iter().map(|f| f.offset_px.length()).sum::<f32>() / flames.len() as f32;
        let max_extent = (w as f32 / 2.0).hypot(h as f32 / 2.0);
        assert!(
            mean_dist < max_extent * 0.5,
            "mean distance {mean_dist:.1} should be < half of max extent {max_extent:.1}"
        );
    }

    #[test]
    fn centered_flames_different_seeds_differ() {
        let a = centered_flames_from_mask(1, 10, 10, solid_rect(10, 10), 5);
        let b = centered_flames_from_mask(2, 10, 10, solid_rect(10, 10), 5);
        assert_ne!(a, b);
    }

    // -----------------------------------------------------------------------
    // perimeter_flames_from_mask
    // -----------------------------------------------------------------------

    #[test]
    fn perimeter_flames_from_mask_cover_visible_edges_deterministically() {
        let config = FireDeathConfig {
            burning_corpse_duration_secs: 1.0,
            burning_flame_count: NonZeroUsize::new(8).unwrap(),
            burning_flame_perimeter_padding_px: 2.0,
            burning_flame_jitter_px: 0.0,
            burning_flame_scale_min: 0.8,
            burning_flame_scale_max: 1.2,
        };
        let width = 20;
        let height = 30;
        let opaque = |x: usize, y: usize| {
            let cx = width / 2;
            let cy = height / 2;
            let dx = x.abs_diff(cx);
            let dy = y.abs_diff(cy);
            dx <= 4 && dy <= 12 || dx <= 7 && dy <= 5
        };
        let first = perimeter_flames_from_mask(123, width, height, opaque, &config);
        let second = perimeter_flames_from_mask(123, width, height, opaque, &config);
        assert_eq!(first, second);
        assert_eq!(first.len(), 8);
        assert!(first.iter().any(|f| f.offset_px.x < -4.0));
        assert!(first.iter().any(|f| f.offset_px.x > 4.0));
        assert!(first.iter().any(|f| f.offset_px.y > 10.0));
        assert!(first.iter().any(|f| f.offset_px.y < -10.0));
    }

    #[test]
    fn perimeter_flames_no_opaque_returns_empty() {
        let config = FireDeathConfig::default();
        let flames = perimeter_flames_from_mask(42, 10, 10, |_, _| false, &config);
        assert!(flames.is_empty());
    }

    #[test]
    fn perimeter_flames_single_pixel() {
        let config = FireDeathConfig {
            burning_flame_count: NonZeroUsize::new(1).unwrap(),
            ..FireDeathConfig::default()
        };
        // Single opaque pixel at (5,5) in a 10×10 grid — always an edge.
        let flames = perimeter_flames_from_mask(42, 10, 10, |x, y| x == 5 && y == 5, &config);
        assert_eq!(flames.len(), 1);
    }
}
