//! Easing functions and rotation keyframe interpolation for Carcinisation.
//!
//! Shared across cutscenes, splash screen, and any game mode that uses
//! keyframe-driven rotation.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Easing function applied when interpolating between keyframes.
#[derive(Clone, Copy, Debug, Deserialize, Reflect, Serialize)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    Smoothstep,
    /// Nearly linear (85% linear + 15% cubic).
    SlightEaseIn,
    /// Under-damped spring: overshoots target ~10%, oscillates, settles.
    DampedSpring,
}

impl Easing {
    #[must_use]
    pub fn apply(self, t: f32) -> f32 {
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Self::Smoothstep => t * t * (3.0 - 2.0 * t),
            Self::SlightEaseIn => 0.85 * t + 0.15 * t * t * t,
            Self::DampedSpring => {
                // Under-damped spring: overshoots ~10% at t≈0.35, settles to 1.0.
                let omega = std::f32::consts::TAU; // natural frequency
                let decay = (-4.0 * t).exp();
                1.0 - decay * (omega * t).cos()
            }
        }
    }
}

/// A single rotation keyframe.
#[derive(Clone, Debug, Deserialize, Reflect, Serialize)]
pub struct RotationKeyframe {
    /// Time in milliseconds (within the local time domain).
    pub time_ms: u64,
    /// Target angle in degrees.
    pub angle_deg: f32,
    /// Easing from this keyframe to the next.
    pub easing: Easing,
}

/// Evaluates a rotation keyframe curve at the given elapsed duration.
///
/// Returns the interpolated angle in **radians**.
///
/// - Before the first keyframe: returns the first keyframe's angle.
/// - After the last keyframe: returns the last keyframe's angle.
/// - Between keyframes: interpolates using the left keyframe's easing.
#[must_use]
pub fn evaluate_rotation_keyframes(keyframes: &[RotationKeyframe], elapsed: Duration) -> f32 {
    if keyframes.is_empty() {
        return 0.0;
    }

    let ms = elapsed.as_millis() as u64;

    if ms <= keyframes[0].time_ms {
        return keyframes[0].angle_deg.to_radians();
    }

    let last = keyframes.len() - 1;
    if ms >= keyframes[last].time_ms {
        return keyframes[last].angle_deg.to_radians();
    }

    for pair in keyframes.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if ms >= a.time_ms && ms < b.time_ms {
            let t = (ms - a.time_ms) as f32 / (b.time_ms - a.time_ms) as f32;
            let eased = a.easing.apply(t);
            let angle = a.angle_deg + (b.angle_deg - a.angle_deg) * eased;
            return angle.to_radians();
        }
    }

    0.0
}

/// Component carrying rotation keyframes for a spawned entity.
///
/// The system that drives this reads `Time<D>` for the relevant time domain
/// and writes `CxPresentationTransform::rotation`.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct RotationKeyframes {
    pub keyframes: Vec<RotationKeyframe>,
    /// Optional per-element offset (radians), e.g. to compensate for
    /// pre-rotated art.
    pub offset: f32,
}
