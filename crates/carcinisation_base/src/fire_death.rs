//! Fire death mechanics: damage types, flame config, and perimeter flame placement.

use bevy::prelude::Vec2;

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
    pub burning_flame_count: usize,
    pub burning_flame_perimeter_padding_px: f32,
    pub burning_flame_jitter_px: f32,
    pub burning_flame_scale_min: f32,
    pub burning_flame_scale_max: f32,
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
///
/// Scans opaque pixels for edges, distributes `count` flames evenly
/// using `seed` for deterministic jitter/scale.
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
                    x as f32 - width as f32 * 0.5,
                    height as f32 * 0.5 - y as f32,
                ));
            }
        }
    }

    if perimeter.is_empty() {
        return Vec::new();
    }

    let count = config
        .burning_flame_count
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

            let scale = scale_min + (scale_max - scale_min) * unit(mixed.rotate_left(21));
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

/// Map hash seed to [-1.0, 1.0).
fn signed_unit(seed: u32) -> f32 {
    unit(seed) * 2.0 - 1.0
}

/// Map hash seed to [0.0, 1.0) via xorshift multiply.
fn unit(seed: u32) -> f32 {
    let mut x = seed;
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    f32::from((x & 0xFFFF) as u16) / 65_535.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perimeter_flames_from_mask_cover_visible_edges_deterministically() {
        let config = FireDeathConfig {
            burning_corpse_duration_secs: 1.0,
            burning_flame_count: 8,
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

        assert!(first.iter().any(|flame| flame.offset_px.x < -4.0));
        assert!(first.iter().any(|flame| flame.offset_px.x > 4.0));
        assert!(first.iter().any(|flame| flame.offset_px.y > 10.0));
        assert!(first.iter().any(|flame| flame.offset_px.y < -10.0));

        assert!(first.iter().all(|flame| flame.offset_px.x.abs() < 10.0));
        assert!(first.iter().all(|flame| flame.offset_px.y.abs() < 15.0));
    }
}
