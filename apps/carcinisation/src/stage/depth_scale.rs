//! Generated-curve fallback depth scaling for entities without depth-specific
//! authored visuals.
//!
//! # Absolute scale vs applied scale
//!
//! Each visible depth (1..=9) has an **absolute scale** value on a geometric
//! curve. Depth 1 (nearest) = [`MAX_SCALE`], depth 9 (farthest) =
//! [`MIN_SCALE`], with a constant ratio between consecutive depths.
//!
//! The **applied scale** (what the entity actually receives) is a ratio:
//!
//! ```text
//! applied = absolute[current_depth] / absolute[authored_depth]
//! ```
//!
//! This means an entity always renders at 1.0 scale at its authored depth,
//! scales up when moving shallower, and scales down when moving deeper —
//! regardless of which depth it was authored for.
//!
//! # Why geometric progression
//!
//! A geometric curve produces a constant *ratio* between adjacent depths.
//! This gives visually uniform perspective scaling — each depth step looks
//! like the same proportional change regardless of where you are on the
//! depth axis.
//!
//! # Fallback reference selection
//!
//! 1. If the entity has authored visuals for the exact current depth → no fallback.
//! 2. Otherwise, pick the **nearest shallower** (numerically smaller) authored depth.
//! 3. If no shallower authored depth exists, pick the **nearest deeper** one.
//!
//! The authored depth defines the visual baseline. There is no privileged
//! "canonical depth" — any depth can be the authored reference.
//!
//! # Depth 0
//!
//! Depth 0 is excluded from normal fallback scaling.
//!
//! # Integration
//!
//! The fallback scale is multiplied into the entity's [`PxPresentationTransform`]
//! scale, preserving sign (flip semantics) and stacking with any existing
//! presentation effects. It does not affect gameplay, collision, or anchoring.

use std::{collections::HashMap, fs};

use bevy::prelude::*;
use carapace::prelude::PxPresentationTransform;
use serde::{Deserialize, Serialize};

use crate::components::DespawnMark;

use super::components::placement::{AuthoredDepths, Depth};

const CONFIG_PATH: &str = "assets/config/depth_scale.ron";

/// Depth range for fallback scaling. Depth 0 is included so that effects
/// spawned at the player depth (e.g. hit/destroy animations authored at
/// depth 1) receive correct scaling.
const VISIBLE_DEPTH_MIN: i8 = 0;
const VISIBLE_DEPTH_MAX: i8 = 9;
/// Number of depths in the original 1-9 range. The geometric ratio is computed
/// so that depth 1 = `MAX_SCALE` and depth 9 = `MIN_SCALE`, same as before.
/// Depth 0 is extrapolated one step beyond depth 1.
const DEPTH_STEP_COUNT: i8 = VISIBLE_DEPTH_MAX;

/// Absolute scale at depth 1 (nearest / largest).
const MAX_SCALE: f32 = 1.0;
/// Absolute scale at depth 9 (farthest / smallest).
///
/// With `MAX_SCALE = 1.0` and `MIN_SCALE = 0.04`, the per-step ratio is
/// `(0.04)^(1/8) ≈ 0.6687`. A sprite authored at depth 3 rendered at
/// depth 1 gets `1.0 / ratio^2 ≈ 2.24x`.
const MIN_SCALE: f32 = 0.04;

/// Generate the default scale table using a geometric progression.
///
/// `scale[d] = MAX_SCALE * ratio^(d - 1)` where
/// `ratio = (MIN_SCALE / MAX_SCALE)^(1 / (DEPTH_STEP_COUNT - 1))`.
///
/// Depth 1 = `MAX_SCALE` (1.0). Depth 0 extrapolates one step closer
/// (= `MAX_SCALE / ratio`), giving hit/destroy effects authored at depth 1
/// a slight scale-up when rendered at the player depth.
fn generate_scale_table() -> HashMap<i8, f32> {
    let ratio = (MIN_SCALE / MAX_SCALE).powf(1.0 / f32::from(DEPTH_STEP_COUNT - 1));
    (VISIBLE_DEPTH_MIN..=VISIBLE_DEPTH_MAX)
        .map(|d| {
            // Anchor at depth 1 = MAX_SCALE; depth 0 = one step shallower.
            let exponent = f32::from(d - 1);
            (d, MAX_SCALE * ratio.powf(exponent))
        })
        .collect()
}

/// Absolute scale values for visible depths 1..=9.
///
/// By default, generated from a geometric progression between [`MAX_SCALE`]
/// (depth 1) and [`MIN_SCALE`] (depth 9). Can be overridden by loading
/// `assets/config/depth_scale.ron`.
#[derive(Resource, Debug, Clone, Deserialize, Serialize, Reflect)]
#[reflect(Resource)]
pub struct DepthScaleConfig {
    /// Absolute scale factor per visible depth. Keys must be 1..=9.
    pub scales: HashMap<i8, f32>,
}

impl Default for DepthScaleConfig {
    fn default() -> Self {
        Self {
            scales: generate_scale_table(),
        }
    }
}

impl DepthScaleConfig {
    /// Load from the RON config file, falling back to generated defaults on error.
    pub fn load_or_default() -> Self {
        if let Ok(body) = fs::read_to_string(CONFIG_PATH) {
            match ron::from_str::<Self>(&body) {
                Ok(config) => {
                    if let Err(e) = config.validate() {
                        warn!(
                            "depth_scale config validation failed ({e}), using generated defaults"
                        );
                        return Self::default();
                    }
                    config
                }
                Err(e) => {
                    warn!("failed to parse {CONFIG_PATH}: {e}, using generated defaults");
                    Self::default()
                }
            }
        } else {
            info!("{CONFIG_PATH} not found, using generated defaults");
            Self::default()
        }
    }

    /// Validate that all visible depths 1..=9 are present and have sane values.
    ///
    /// # Errors
    ///
    /// Returns an error string if any depth is missing, non-positive, or non-finite.
    pub fn validate(&self) -> Result<(), String> {
        for d in VISIBLE_DEPTH_MIN..=VISIBLE_DEPTH_MAX {
            match self.scales.get(&d) {
                None => return Err(format!("missing scale for depth {d}")),
                Some(&v) if v <= 0.0 => {
                    return Err(format!("scale for depth {d} must be positive, got {v}"));
                }
                Some(&v) if v.is_nan() || v.is_infinite() => {
                    return Err(format!("scale for depth {d} is not finite: {v}"));
                }
                Some(_) => {}
            }
        }
        Ok(())
    }

    /// Look up the configured absolute scale for a depth.
    ///
    /// Returns `None` if the depth is not in the config.
    #[must_use]
    pub fn scale_for(&self, depth: Depth) -> Option<f32> {
        self.scales.get(&depth.to_i8()).copied()
    }

    /// Compute the fallback scale ratio between two depths.
    ///
    /// Returns `None` if either depth is 0 or not in the config.
    /// Returns `1.0` when current == reference.
    #[must_use]
    pub fn fallback_scale(&self, current: Depth, reference_depth: Depth) -> Option<f32> {
        let current_scale = self.scale_for(current)?;
        let reference_scale = self.scale_for(reference_depth)?;
        if reference_scale.abs() < f32::EPSILON {
            return None;
        }
        Some(current_scale / reference_scale)
    }

    /// Compute the fallback scale for an entity at `current_depth` given its
    /// [`AuthoredDepths`].
    ///
    /// Returns `1.0` if the current depth is authored or no reference can be resolved.
    #[must_use]
    pub fn resolve_fallback(&self, current_depth: Depth, authored: &AuthoredDepths) -> f32 {
        if authored.is_empty() {
            return 1.0;
        }
        if authored.contains(current_depth) {
            return 1.0;
        }
        let Some(reference) = authored.resolve_reference(current_depth) else {
            return 1.0;
        };
        self.fallback_scale(current_depth, reference).unwrap_or(1.0)
    }
}

/// Tracks the last applied fallback scale so it can be cleanly reversed/updated
/// without compounding across frames.
#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct DepthFallbackScale(pub Vec2);

/// System that applies or updates the fallback depth scale on entities with
/// [`AuthoredDepths`] and a current [`Depth`].
///
/// Runs in `Update`. Multiplies the fallback ratio into the entity's
/// [`PxPresentationTransform`] scale, stacking with any existing effects.
pub fn apply_depth_fallback_scale(
    config: Res<DepthScaleConfig>,
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &Depth,
            &AuthoredDepths,
            Option<&mut PxPresentationTransform>,
            Option<&DepthFallbackScale>,
        ),
        Without<DespawnMark>,
    >,
) {
    for (entity, &current_depth, authored, presentation_opt, prev_fallback) in &mut query {
        let ratio = config.resolve_fallback(current_depth, authored);
        let new_fallback = Vec2::splat(ratio);

        // Check if the fallback scale actually changed.
        if let Some(prev) = prev_fallback {
            if (prev.0 - new_fallback).length_squared() < f32::EPSILON {
                continue;
            }
        } else if (ratio - 1.0).abs() < f32::EPSILON {
            continue;
        }

        let prev_scale = prev_fallback.map_or(Vec2::ONE, |f| f.0);

        if let Some(mut pt) = presentation_opt {
            // Undo previous fallback, apply new one.
            // Preserve sign (flip semantics) by working with magnitudes.
            let sign_x = pt.scale.x.signum();
            let sign_y = pt.scale.y.signum();
            let base_x = pt.scale.x.abs() / prev_scale.x;
            let base_y = pt.scale.y.abs() / prev_scale.y;

            pt.scale = Vec2::new(
                sign_x * base_x * new_fallback.x,
                sign_y * base_y * new_fallback.y,
            );
        } else if (ratio - 1.0).abs() >= f32::EPSILON {
            commands.entity(entity).insert(PxPresentationTransform {
                scale: new_fallback,
                ..Default::default()
            });
        }

        commands
            .entity(entity)
            .insert(DepthFallbackScale(new_fallback));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> DepthScaleConfig {
        DepthScaleConfig::default()
    }

    // --- Generation tests ---

    #[test]
    fn generated_depth_1_equals_max_scale() {
        let config = default_config();
        let s = config.scales[&1];
        assert!(
            (s - MAX_SCALE).abs() < 1e-6,
            "depth 1 should be {MAX_SCALE}, got {s}"
        );
    }

    #[test]
    fn generated_depth_3_matches_ratio_squared() {
        let config = default_config();
        let ratio = (MIN_SCALE / MAX_SCALE).powf(1.0 / f32::from(DEPTH_STEP_COUNT - 1));
        let expected = MAX_SCALE * ratio * ratio;
        let s = config.scales[&3];
        assert!(
            (s - expected).abs() < 1e-6,
            "depth 3 should be {expected}, got {s}"
        );
    }

    #[test]
    fn generated_depth_9_equals_min_scale() {
        let config = default_config();
        let s = config.scales[&9];
        assert!(
            (s - MIN_SCALE).abs() < 1e-4,
            "depth 9 should be ~{MIN_SCALE}, got {s}"
        );
    }

    #[test]
    fn generated_table_is_monotonically_decreasing() {
        let config = default_config();
        let mut prev = f32::MAX;
        for d in VISIBLE_DEPTH_MIN..=VISIBLE_DEPTH_MAX {
            let s = config.scales[&d];
            assert!(s < prev, "depth {d} scale {s} should be < previous {prev}");
            prev = s;
        }
    }

    #[test]
    fn generated_table_has_constant_ratio() {
        let config = default_config();
        let first_ratio = config.scales[&2] / config.scales[&1];
        for d in 2..VISIBLE_DEPTH_MAX {
            let ratio = config.scales[&(d + 1)] / config.scales[&d];
            assert!(
                (ratio - first_ratio).abs() < 1e-5,
                "ratio at depth {d}→{} = {ratio:.6}, expected {first_ratio:.6}",
                d + 1,
            );
        }
    }

    // --- Config validation tests ---

    #[test]
    fn config_validates_all_depths_present() {
        assert!(default_config().validate().is_ok());
    }

    #[test]
    fn config_rejects_missing_depth() {
        let mut config = default_config();
        config.scales.remove(&5);
        let err = config.validate().unwrap_err();
        assert!(err.contains("missing scale for depth 5"), "got: {err}");
    }

    #[test]
    fn config_rejects_zero_scale() {
        let mut config = default_config();
        config.scales.insert(3, 0.0);
        assert!(config.validate().unwrap_err().contains("positive"));
    }

    #[test]
    fn config_rejects_negative_scale() {
        let mut config = default_config();
        config.scales.insert(7, -0.5);
        assert!(config.validate().unwrap_err().contains("positive"));
    }

    #[test]
    fn config_rejects_nan_scale() {
        let mut config = default_config();
        config.scales.insert(2, f32::NAN);
        assert!(config.validate().unwrap_err().contains("not finite"));
    }

    // --- AuthoredDepths reference resolution tests ---

    #[test]
    fn resolve_reference_exact_match() {
        let authored = AuthoredDepths::new(vec![Depth::Three, Depth::Six]);
        assert_eq!(authored.resolve_reference(Depth::Three), Some(Depth::Three));
        assert_eq!(authored.resolve_reference(Depth::Six), Some(Depth::Six));
    }

    #[test]
    fn resolve_reference_prefers_nearest_shallower() {
        let authored = AuthoredDepths::new(vec![Depth::Two, Depth::Seven]);
        assert_eq!(authored.resolve_reference(Depth::Five), Some(Depth::Two));
    }

    #[test]
    fn resolve_reference_falls_back_to_deeper_when_no_shallower() {
        let authored = AuthoredDepths::new(vec![Depth::Five, Depth::Eight]);
        assert_eq!(authored.resolve_reference(Depth::Three), Some(Depth::Five));
    }

    #[test]
    fn resolve_reference_single_authored_depth() {
        let authored = AuthoredDepths::single(Depth::Three);
        assert_eq!(authored.resolve_reference(Depth::One), Some(Depth::Three));
        assert_eq!(authored.resolve_reference(Depth::Nine), Some(Depth::Three));
        assert_eq!(authored.resolve_reference(Depth::Three), Some(Depth::Three));
    }

    #[test]
    fn resolve_reference_empty_returns_none() {
        let authored = AuthoredDepths::new(vec![]);
        assert_eq!(authored.resolve_reference(Depth::Three), None);
    }

    #[test]
    fn resolve_reference_nearest_shallower_is_closest() {
        let authored = AuthoredDepths::new(vec![Depth::One, Depth::Three, Depth::Eight]);
        assert_eq!(authored.resolve_reference(Depth::Five), Some(Depth::Three));
    }

    #[test]
    fn resolve_reference_nearest_deeper_is_closest() {
        let authored = AuthoredDepths::new(vec![Depth::Five, Depth::Nine]);
        assert_eq!(authored.resolve_reference(Depth::Two), Some(Depth::Five));
    }

    // --- Fallback scale computation tests ---

    #[test]
    fn exact_authored_depth_no_fallback() {
        let config = default_config();
        let authored = AuthoredDepths::single(Depth::Three);
        assert!((config.resolve_fallback(Depth::Three, &authored) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn exact_authored_depth_multi_no_fallback() {
        let config = default_config();
        let authored = AuthoredDepths::new(vec![Depth::Three, Depth::Six]);
        assert!((config.resolve_fallback(Depth::Three, &authored) - 1.0).abs() < f32::EPSILON);
        assert!((config.resolve_fallback(Depth::Six, &authored) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn identity_at_authored_depth_for_any_authored_depth() {
        let config = default_config();
        // Verify identity for every possible authored depth — no depth is special.
        for d in VISIBLE_DEPTH_MIN..=VISIBLE_DEPTH_MAX {
            let depth = Depth::try_from(d).unwrap();
            let authored = AuthoredDepths::single(depth);
            let scale = config.resolve_fallback(depth, &authored);
            assert!(
                (scale - 1.0).abs() < f32::EPSILON,
                "authored at depth {d} should produce identity, got {scale}"
            );
        }
    }

    #[test]
    fn shallower_scales_up() {
        let config = default_config();
        let authored = AuthoredDepths::single(Depth::Five);
        let scale = config.resolve_fallback(Depth::Two, &authored);
        assert!(scale > 1.0, "shallower depth should scale up, got {scale}");
    }

    #[test]
    fn deeper_scales_down() {
        let config = default_config();
        let authored = AuthoredDepths::single(Depth::Five);
        let scale = config.resolve_fallback(Depth::Eight, &authored);
        assert!(scale < 1.0, "deeper depth should scale down, got {scale}");
    }

    #[test]
    fn multi_authored_picks_nearest_shallower() {
        let config = default_config();
        let authored = AuthoredDepths::new(vec![Depth::Two, Depth::Seven]);
        let expected = config.scales[&5] / config.scales[&2];
        let scale = config.resolve_fallback(Depth::Five, &authored);
        assert!(
            (scale - expected).abs() < 0.001,
            "expected ~{expected:.3}, got {scale}"
        );
    }

    #[test]
    fn multi_authored_picks_nearest_deeper_when_no_shallower() {
        let config = default_config();
        let authored = AuthoredDepths::new(vec![Depth::Five, Depth::Eight]);
        let expected = config.scales[&3] / config.scales[&5];
        let scale = config.resolve_fallback(Depth::Three, &authored);
        assert!(
            (scale - expected).abs() < 0.001,
            "expected ~{expected:.3}, got {scale}"
        );
    }

    #[test]
    fn fallback_monotonically_decreasing_single_authored() {
        let config = default_config();
        let authored = AuthoredDepths::single(Depth::Three);
        let mut prev = f32::MAX;
        for d in 1..=9_i8 {
            let depth = Depth::try_from(d).unwrap();
            let scale = config.resolve_fallback(depth, &authored);
            assert!(
                scale < prev,
                "depth {d} scale {scale} should be < previous {prev}"
            );
            prev = scale;
        }
    }

    #[test]
    fn depth_zero_scales_up_from_authored() {
        let config = default_config();
        let authored = AuthoredDepths::single(Depth::Three);
        let scale = config.resolve_fallback(Depth::Zero, &authored);
        assert!(
            scale > 1.0,
            "depth 0 should scale up from depth 3, got {scale}"
        );
    }

    #[test]
    fn empty_authored_returns_one() {
        let config = default_config();
        let authored = AuthoredDepths::new(vec![]);
        assert!((config.resolve_fallback(Depth::Five, &authored) - 1.0).abs() < f32::EPSILON);
    }

    // --- Low-level config tests ---

    #[test]
    fn scale_for_depth_zero_returns_value() {
        let config = default_config();
        let s = config.scale_for(Depth::Zero).unwrap();
        let s1 = config.scale_for(Depth::One).unwrap();
        assert!(
            s > s1,
            "depth 0 scale ({s}) should be > depth 1 scale ({s1})"
        );
    }

    #[test]
    fn scale_for_all_depths_returns_values() {
        let config = default_config();
        for d in 0..=9_i8 {
            let depth = Depth::try_from(d).unwrap();
            assert!(config.scale_for(depth).is_some(), "depth {d}");
        }
    }

    #[test]
    fn fallback_scale_same_depth_is_one() {
        let config = default_config();
        let scale = config.fallback_scale(Depth::Three, Depth::Three);
        assert!((scale.unwrap() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fallback_scale_depth_zero_scales_up() {
        let config = default_config();
        let scale = config.fallback_scale(Depth::Zero, Depth::Three).unwrap();
        assert!(
            scale > 1.0,
            "depth 0 from depth 3 should scale up, got {scale}"
        );
    }

    // --- AuthoredDepths construction tests ---

    #[test]
    fn authored_depths_deduplicates_and_sorts() {
        let ad = AuthoredDepths::new(vec![Depth::Five, Depth::Two, Depth::Five, Depth::One]);
        assert_eq!(ad.0, vec![Depth::One, Depth::Two, Depth::Five]);
    }

    #[test]
    fn authored_depths_contains() {
        let ad = AuthoredDepths::new(vec![Depth::Three, Depth::Six]);
        assert!(ad.contains(Depth::Three));
        assert!(ad.contains(Depth::Six));
        assert!(!ad.contains(Depth::Five));
    }

    // --- Spawn-defaulting integration test ---

    #[test]
    fn none_authored_depths_defaults_to_spawn_depth() {
        let config = default_config();
        let spawn_depth = Depth::Five;
        let authored = match &None::<Vec<Depth>> {
            Some(depths) => AuthoredDepths::new(depths.clone()),
            None => AuthoredDepths::single(spawn_depth),
        };
        // At spawn depth → no fallback.
        assert!((config.resolve_fallback(spawn_depth, &authored) - 1.0).abs() < f32::EPSILON);
        // At different depth → fallback computed vs spawn depth.
        let scale = config.resolve_fallback(Depth::Two, &authored);
        let expected = config.scales[&2] / config.scales[&5];
        assert!(
            (scale - expected).abs() < 0.001,
            "expected ~{expected:.3}, got {scale}"
        );
    }
}
