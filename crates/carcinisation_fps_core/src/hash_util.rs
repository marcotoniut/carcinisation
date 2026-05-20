//! Deterministic hash-based random utilities for flame placement and visual effects.

/// Map a u32 seed to [0.0, 1.0) via xorshift multiply.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn unit(seed: u32) -> f32 {
    let mut x = seed;
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    f32::from((x & 0xFFFF) as u16) / 65_535.0
}

/// Map a u32 seed to [-1.0, 1.0).
#[must_use]
pub fn signed_unit(seed: u32) -> f32 {
    unit(seed) * 2.0 - 1.0
}
