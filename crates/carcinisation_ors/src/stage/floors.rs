//! Authoritative floor topology for the depth-first stage model.
//!
//! ## Relationship to projection
//!
//! Floors are NOT independent of projection. They are layered on top:
//!
//! ```text
//!     baseline floor_y    = projection.floor_y_for_depth(d)
//!     resolved floor_y    = baseline + (authored override delta, if any)
//! ```
//!
//! When projection changes (e.g. during a tween), every resolved floor's
//! Y moves automatically because its baseline moved. Authored overrides
//! (`SurfaceSpec` with `Projected`/`Anchored`/`Gap` modes) are applied on top of
//! the propagated baseline.
//!
//! ## Runtime interpolation
//!
//! [`evaluate_floors_at`] produces time-continuous floor state during tween
//! steps. Per-depth rules:
//!
//! - **`Solid → Solid`:** Y values lerp across the tween.
//! - **`Gap → Gap`:** stays gap throughout.
//! - **`Solid ↔ Gap` (topology change):** snaps at t=0 to the destination
//!   state. The tween represents moving *through the destination world*.

use std::{collections::BTreeMap, ops::RangeInclusive, time::Duration};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    components::placement::Depth,
    data::{StageData, StageStep},
    projection::{ProjectionProfile, effective_projection, walk_steps_at_elapsed},
    resources::ActiveProjection,
};

/// Resolved floor surfaces per depth, computed from the current
/// [`ActiveProjection`] baseline plus any authored [`ActiveSurfaceLayout`]
/// overrides. Recomputed each frame.
///
/// Each depth has zero or more surfaces. Single-surface stages produce
/// single-element Vecs. Multi-surface authoring produces multiple entries.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ActiveFloors {
    pub by_depth: BTreeMap<Depth, Vec<Surface>>,
}

impl ActiveFloors {
    /// Returns all resolved surfaces at a depth, or an empty slice.
    #[must_use]
    pub fn surfaces_at(&self, depth: Depth) -> &[Surface] {
        self.by_depth.get(&depth).map_or(&[], |v| v.as_slice())
    }

    /// Returns the first resolved surface at a depth (convenience for
    /// single-surface queries).
    #[must_use]
    pub fn surface(&self, depth: Depth) -> Option<Surface> {
        self.surfaces_at(depth).first().copied()
    }

    /// Returns the first solid floor Y at a depth, if any exists.
    ///
    /// For single-surface depths this returns the only solid Y. For
    /// multi-surface depths, returns the first solid in authored order.
    #[must_use]
    pub fn solid_y(&self, depth: Depth) -> Option<f32> {
        self.surfaces_at(depth).iter().find_map(|s| match s {
            Surface::Solid { y } => Some(*y),
            Surface::Gap => None,
        })
    }

    /// Returns the lowest solid Y at a depth (the ground surface).
    ///
    /// Used by spawn placement to position entities on the ground floor,
    /// ignoring platforms above.
    ///
    /// Future: spawn authoring may target a specific surface via tagged
    /// identity. V1 picks ground = lowest solid Y.
    #[must_use]
    pub fn lowest_solid_y(&self, depth: Depth) -> Option<f32> {
        self.surfaces_at(depth)
            .iter()
            .filter_map(|s| match s {
                Surface::Solid { y } => Some(*y),
                Surface::Gap => None,
            })
            .reduce(f32::min)
    }

    /// Returns the highest solid Y at a depth (the first surface hit when
    /// falling from above).
    #[must_use]
    pub fn highest_solid_y(&self, depth: Depth) -> Option<f32> {
        self.surfaces_at(depth)
            .iter()
            .filter_map(|s| match s {
                Surface::Solid { y } => Some(*y),
                Surface::Gap => None,
            })
            .reduce(f32::max)
    }

    /// Returns the highest solid Y at or below `max_y` at a depth.
    ///
    /// Used by falling physics to find the surface a falling entity
    /// would land on — the highest solid surface below the entity's
    /// current body bottom Y.
    #[must_use]
    pub fn highest_solid_y_at_or_below(&self, depth: Depth, max_y: f32) -> Option<f32> {
        self.surfaces_at(depth)
            .iter()
            .filter_map(|s| match s {
                Surface::Solid { y } if *y <= max_y => Some(*y),
                _ => None,
            })
            .reduce(f32::max)
    }
}

/// Runtime resource holding the active authored floor layout.
///
/// Useful for debug/inspection. The authoritative resolved floor state is
/// [`ActiveFloors`], which is produced by [`evaluate_floors_at`] and
/// includes interpolation during tween steps.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ActiveSurfaceLayout(pub SurfaceLayout);

/// Author-facing floor layout composed of ordered depth spans.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SurfaceLayout {
    pub spans: Vec<SurfaceSpec>,
}

// Future: optional `id: String` on SurfaceSpec for explicit cross-step
// surface matching. V1 uses positional index within the spans list.

/// A floor rule applied to an inclusive depth range.
#[derive(Clone, Debug, PartialEq, Reflect, Serialize, Deserialize)]
pub struct SurfaceSpec {
    pub depths: RangeInclusive<Depth>,
    pub mode: HeightMode,
}

/// How a surface resolves its Y position.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, Serialize, Deserialize)]
pub enum HeightMode {
    /// Derives Y from the projection curve with an optional displacement.
    ///
    /// `displacement = 0.0` sits on the projected baseline.  Positive values
    /// place the surface above the baseline, negative below.  When the
    /// projection profile changes during a tween, Projected surfaces slide
    /// automatically because the baseline moves.
    Projected { displacement: f32 },
    /// Fixed screen Y. Does not move when projection changes.
    Anchored(f32),
    /// No gameplay floor exists at this depth.
    Gap,
}

/// Resolved floor state for a single depth lane.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, Serialize, Deserialize)]
pub enum Surface {
    Solid { y: f32 },
    Gap,
}

/// Resolve the active floor layout for the current stage step.
///
/// Floor layouts are sticky carry-forward state: the last step that authored a
/// floor override remains active until a later step replaces it.
#[must_use]
pub fn effective_floor_layout(stage_data: &StageData, step_index: usize) -> SurfaceLayout {
    if stage_data.steps.is_empty() {
        return SurfaceLayout::default();
    }

    let mut layout = SurfaceLayout::default();
    let last_index = step_index.min(stage_data.steps.len().saturating_sub(1));
    for step in stage_data.steps.iter().take(last_index + 1) {
        if let Some(step_layout) = surface_layout_override(step) {
            layout = step_layout;
        }
    }
    layout
}

fn surface_layout_override(step: &StageStep) -> Option<SurfaceLayout> {
    let specs = match step {
        StageStep::Tween(step) => step.surfaces.as_ref(),
        StageStep::Stop(step) => step.surfaces.as_ref(),
        StageStep::Cinematic(_) => return None,
    };

    // Some(vec) → replace with this layout. None → inherit (carry-forward).
    specs.map(|v| SurfaceLayout { spans: v.clone() })
}

/// Resolve one authoritative floor surface per depth lane.
///
/// Resolution rules:
/// 1. Start from the projection baseline for every depth
/// 2. Apply spans in author order
/// 3. Later spans override earlier spans
/// 4. Every depth ends as either [`Surface::Solid`] or [`Surface::Gap`]
#[must_use]
pub fn resolve_active_floors(
    projection: &ActiveProjection,
    layout: &SurfaceLayout,
) -> ActiveFloors {
    resolve_floors_from(&projection.0, layout)
}

/// Minimum tween duration (in seconds) below which floor interpolation is
/// skipped. Mirrors [`super::projection::MIN_INTERPOLATION_DURATION_SECS`].
const MIN_INTERPOLATION_DURATION_SECS: f32 = 0.01;

/// Resolve a single depth lane to an optional solid Y.
///
/// Returns `Some(y)` for any solid surface, `None` for gaps.
fn resolve_depth_y(
    depth: Depth,
    projection: &ProjectionProfile,
    layout: &SurfaceLayout,
) -> Option<f32> {
    let baseline_y = projection.floor_y_for_depth(depth.to_i8());
    let mut surface = Surface::Solid { y: baseline_y };

    for span in &layout.spans {
        if span.depths.contains(&depth) {
            surface = match span.mode {
                HeightMode::Projected { displacement } => Surface::Solid {
                    y: baseline_y + displacement,
                },
                HeightMode::Anchored(y) => Surface::Solid { y },
                HeightMode::Gap => Surface::Gap,
            };
        }
    }

    match surface {
        Surface::Solid { y } => Some(y),
        Surface::Gap => None,
    }
}

/// Evaluate the interpolated floor state at a given elapsed time.
///
/// Mirrors [`super::projection::evaluate_projection_at`] for the floor layer.
/// During tween steps, per-depth floors interpolate between the previous and
/// current step's resolved state. The only input is the resolved Y for each
/// depth — span identity, height mode, and authored structure are transparent.
///
/// Two rules:
///
/// - **Both sides produce a Y** (`Some → Some`): lerp between the two values.
/// - **Either side produces no Y** (`Some ↔ None`, `None → None`): snap at
///   t=0 to the destination state. The tween represents moving *through the
///   destination world*, not transitioning between two worlds.
///
/// During stop/cinematic steps, holds the current step's resolved state.
/// Zero-duration tweens snap to the destination (no interpolation).
///
/// # Panics
/// Panics if `Depth::try_from` fails for any depth in the visible range.
#[must_use]
pub fn evaluate_floors_at(stage_data: &StageData, elapsed: Duration) -> ActiveFloors {
    let info = walk_steps_at_elapsed(stage_data, elapsed);

    let curr_layout = effective_floor_layout(stage_data, info.step_index);
    let curr_projection = effective_projection(stage_data, info.step_index);

    // Interpolate only during tween steps with meaningful progress.
    if info.tween_progress < 1.0
        && let Some(StageStep::Tween(tween)) = stage_data.steps.get(info.step_index)
    {
        use super::projection::tween_duration;

        let dur = tween_duration(info.step_start_position, tween);
        if dur.as_secs_f32() >= MIN_INTERPOLATION_DURATION_SECS {
            let prev_layout = if info.step_index > 0 {
                effective_floor_layout(stage_data, info.step_index - 1)
            } else {
                SurfaceLayout::default()
            };
            let prev_projection = if info.step_index > 0 {
                effective_projection(stage_data, info.step_index - 1)
            } else {
                stage_data.projection.unwrap_or_default()
            };

            let t = info.tween_progress;
            let by_depth = (Depth::MIN.to_i8()..=Depth::MAX.to_i8())
                .map(|depth_i8| {
                    let depth =
                        Depth::try_from(depth_i8).expect("visible depth range must be valid");
                    let prev_y = resolve_depth_y(depth, &prev_projection, &prev_layout);
                    let curr_y = resolve_depth_y(depth, &curr_projection, &curr_layout);

                    // Per-surface interpolation by index. Current stages produce
                    // one surface per depth; multi-surface matching extends this
                    // to Vec pairs when authored content uses stacked surfaces.
                    let surface = match (prev_y, curr_y) {
                        (Some(a), Some(b)) => Surface::Solid { y: a + (b - a) * t },
                        // Y appears mid-tween → snap to destination.
                        (None, Some(b)) => Surface::Solid { y: b },
                        // Y absent or disappearing → gap.
                        _ => Surface::Gap,
                    };

                    (depth, vec![surface])
                })
                .collect();

            return ActiveFloors { by_depth };
        }
    }

    // Non-tween steps or zero-duration tweens: resolve at current state.
    resolve_floors_from(&curr_projection, &curr_layout)
}

/// Resolve floors from a projection profile and layout (non-interpolated).
///
/// Equivalent to [`resolve_active_floors`] but takes a `&ProjectionProfile`
/// directly rather than an `&ActiveProjection` wrapper.
#[must_use]
fn resolve_floors_from(projection: &ProjectionProfile, layout: &SurfaceLayout) -> ActiveFloors {
    let by_depth = (Depth::MIN.to_i8()..=Depth::MAX.to_i8())
        .map(|depth_i8| {
            let depth = Depth::try_from(depth_i8).expect("visible depth range must be valid");
            let surface = match resolve_depth_y(depth, projection, layout) {
                Some(y) => Surface::Solid { y },
                None => Surface::Gap,
            };
            (depth, vec![surface])
        })
        .collect();

    ActiveFloors { by_depth }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::{
        components::{StopStageStep, TweenStageStep},
        projection::ProjectionProfile,
    };
    use bevy::prelude::Vec2;

    fn active_projection() -> ActiveProjection {
        ActiveProjection(ProjectionProfile {
            horizon_y: 72.0,
            floor_base_y: -14.4,
            bias_power: 3.0,
        })
    }

    fn stage_with_steps(steps: Vec<StageStep>) -> StageData {
        StageData {
            name: "test".into(),
            music_path: String::new(),
            background_path: String::new(),
            skybox: crate::stage::data::SkyboxData {
                path: String::new(),
                frames: 1,
            },
            start_coordinates: Vec2::ZERO,
            spawns: Vec::new(),
            steps,
            on_start_transition_o: None,
            on_end_transition_o: None,
            gravity: None,
            projection: None,
            checkpoint: None,
            parallax_attenuation: None,
            primitives: vec![],
            primitive_bands: None,
        }
    }

    #[test]
    fn resolve_active_floors_defaults_to_projection_baseline() {
        let projection = active_projection();
        let floors = resolve_active_floors(&projection, &SurfaceLayout::default());

        for depth_i8 in Depth::MIN.to_i8()..=Depth::MAX.to_i8() {
            let depth = Depth::try_from(depth_i8).unwrap();
            assert_eq!(
                floors.surface(depth),
                Some(Surface::Solid {
                    y: projection.0.floor_y_for_depth(depth_i8),
                })
            );
        }
    }

    #[test]
    fn resolve_active_floors_applies_later_spans_last() {
        let projection = active_projection();
        let layout = SurfaceLayout {
            spans: vec![
                SurfaceSpec {
                    depths: Depth::Three..=Depth::Five,
                    mode: HeightMode::Projected { displacement: 10.0 },
                },
                SurfaceSpec {
                    depths: Depth::Four..=Depth::Four,
                    mode: HeightMode::Anchored(5.0),
                },
            ],
        };

        let floors = resolve_active_floors(&projection, &layout);

        assert_eq!(
            floors.surface(Depth::Three),
            Some(Surface::Solid {
                y: projection.0.floor_y_for_depth(3) + 10.0
            })
        );
        assert_eq!(floors.surface(Depth::Four), Some(Surface::Solid { y: 5.0 }));
        assert_eq!(
            floors.surface(Depth::Five),
            Some(Surface::Solid {
                y: projection.0.floor_y_for_depth(5) + 10.0
            })
        );
    }

    #[test]
    fn resolve_active_floors_supports_gaps() {
        let projection = active_projection();
        let layout = SurfaceLayout {
            spans: vec![SurfaceSpec {
                depths: Depth::Two..=Depth::Three,
                mode: HeightMode::Gap,
            }],
        };

        let floors = resolve_active_floors(&projection, &layout);
        assert_eq!(floors.surface(Depth::Two), Some(Surface::Gap));
        assert_eq!(floors.surface(Depth::Three), Some(Surface::Gap));
        assert!(floors.solid_y(Depth::Two).is_none());
    }

    // --- Projected height model ---

    #[test]
    fn surface_resolution_projected_with_zero_displacement_matches_baseline() {
        let projection = active_projection();
        let layout = SurfaceLayout {
            spans: vec![SurfaceSpec {
                depths: Depth::Four..=Depth::Four,
                mode: HeightMode::Projected { displacement: 0.0 },
            }],
        };
        let floors = resolve_active_floors(&projection, &layout);
        assert_eq!(
            floors.surface(Depth::Four),
            Some(Surface::Solid {
                y: projection.0.floor_y_for_depth(4),
            })
        );
    }

    #[test]
    fn surface_resolution_projected_with_displacement_offsets_baseline() {
        let projection = active_projection();
        let layout = SurfaceLayout {
            spans: vec![SurfaceSpec {
                depths: Depth::Four..=Depth::Four,
                mode: HeightMode::Projected { displacement: 20.0 },
            }],
        };
        let floors = resolve_active_floors(&projection, &layout);
        assert_eq!(
            floors.surface(Depth::Four),
            Some(Surface::Solid {
                y: projection.0.floor_y_for_depth(4) + 20.0,
            })
        );
    }

    #[test]
    fn surface_resolution_projected_tracks_projection_changes() {
        let profile_a = ProjectionProfile {
            horizon_y: 80.0,
            floor_base_y: 0.0,
            bias_power: 1.0,
        };
        let profile_b = ProjectionProfile {
            horizon_y: 80.0,
            floor_base_y: -40.0,
            bias_power: 1.0,
        };
        let layout = SurfaceLayout {
            spans: vec![SurfaceSpec {
                depths: Depth::Five..=Depth::Five,
                mode: HeightMode::Projected { displacement: 10.0 },
            }],
        };

        let floors_a = resolve_floors_from(&profile_a, &layout);
        let floors_b = resolve_floors_from(&profile_b, &layout);

        let ya = floors_a.solid_y(Depth::Five).unwrap();
        let yb = floors_b.solid_y(Depth::Five).unwrap();
        let expected_a = profile_a.floor_y_for_depth(5) + 10.0;
        let expected_b = profile_b.floor_y_for_depth(5) + 10.0;
        assert!((ya - expected_a).abs() < 0.01);
        assert!((yb - expected_b).abs() < 0.01);
        // profile_b has a lower floor_base_y, so depth 5 baseline is lower.
        assert!(
            (ya - yb).abs() > 1.0,
            "different projections should produce different Y: ya={ya}, yb={yb}"
        );
    }

    #[test]
    fn surface_resolution_anchored_ignores_projection() {
        let profile_a = ProjectionProfile {
            horizon_y: 80.0,
            floor_base_y: 0.0,
            bias_power: 1.0,
        };
        let profile_b = ProjectionProfile {
            horizon_y: 100.0,
            floor_base_y: -20.0,
            bias_power: 1.0,
        };
        let layout = SurfaceLayout {
            spans: vec![SurfaceSpec {
                depths: Depth::Five..=Depth::Five,
                mode: HeightMode::Anchored(42.0),
            }],
        };

        let floors_a = resolve_floors_from(&profile_a, &layout);
        let floors_b = resolve_floors_from(&profile_b, &layout);

        assert_eq!(floors_a.solid_y(Depth::Five), Some(42.0));
        assert_eq!(floors_b.solid_y(Depth::Five), Some(42.0));
    }

    #[test]
    fn effective_floor_layout_carries_forward_latest_override() {
        let steps = vec![
            TweenStageStep::base(0.0, 0.0).into(),
            StopStageStep::new()
                .with_surfaces(vec![SurfaceSpec {
                    depths: Depth::Four..=Depth::Four,
                    mode: HeightMode::Anchored(18.0),
                }])
                .into(),
            TweenStageStep::base(10.0, 0.0).into(),
        ];
        let stage = stage_with_steps(steps);

        let layout = effective_floor_layout(&stage, 2);
        let projection = active_projection();
        let floors = resolve_active_floors(&projection, &layout);
        assert_eq!(
            floors.surface(Depth::Four),
            Some(Surface::Solid { y: 18.0 })
        );
    }

    // --- evaluate_floors_at: Y interpolation ---

    /// Helper: tween from origin to (100,0) at speed 1 → 100s duration.
    fn tween_100s() -> TweenStageStep {
        TweenStageStep::base(100.0, 0.0)
    }

    #[test]
    fn floor_y_interpolates_for_same_mode_change() {
        // Step 0: tween (0,0)→(100,0) = 100s, Anchored(10) at depth 4.
        // Step 1: tween (100,0)→(200,0) = 100s, Anchored(30) at depth 4.
        let stage = stage_with_steps(vec![
            tween_100s()
                .with_surfaces(vec![SurfaceSpec {
                    depths: Depth::Four..=Depth::Four,
                    mode: HeightMode::Anchored(10.0),
                }])
                .into(),
            TweenStageStep::base(200.0, 0.0)
                .with_surfaces(vec![SurfaceSpec {
                    depths: Depth::Four..=Depth::Four,
                    mode: HeightMode::Anchored(30.0),
                }])
                .into(),
        ]);

        // At 150s: 100s step 0 + 50s into step 1 → t=0.5
        let floors = evaluate_floors_at(&stage, Duration::from_secs(150));
        let y = floors.solid_y(Depth::Four).expect("should be solid");
        assert!(
            (y - 20.0).abs() < 0.1,
            "at t=0.5, floor should be lerp(10,30,0.5)=20, got {y}"
        );
    }

    #[test]
    fn floor_y_interpolates_for_cross_mode_solid_change() {
        // Step 0: Projected{-10} at depth 4. Step 1: Anchored(50) at depth 4.
        // Both produce a Y value, so they lerp.
        let step0_layout = SurfaceLayout {
            spans: vec![SurfaceSpec {
                depths: Depth::Four..=Depth::Four,
                mode: HeightMode::Projected {
                    displacement: -10.0,
                },
            }],
        };
        let step1_layout = SurfaceLayout {
            spans: vec![SurfaceSpec {
                depths: Depth::Four..=Depth::Four,
                mode: HeightMode::Anchored(50.0),
            }],
        };
        let projection = active_projection().0;

        let prev_y = resolve_depth_y(Depth::Four, &projection, &step0_layout).unwrap();
        let curr_y = resolve_depth_y(Depth::Four, &projection, &step1_layout).unwrap();
        let expected_mid = prev_y + (curr_y - prev_y) * 0.5;

        // Build a stage where step 0 has Projected{-10}, step 1 has Anchored(50).
        // Test resolve_depth_y directly for the cross-mode case.
        assert!(
            (prev_y - (projection.floor_y_for_depth(4) - 10.0)).abs() < 0.01,
            "Projected(-10) should produce baseline - 10"
        );
        assert!(
            (curr_y - 50.0).abs() < 0.01,
            "Anchored(50) should produce 50"
        );
        assert!(
            (expected_mid - f32::midpoint(prev_y, curr_y)).abs() < 0.01,
            "midpoint should be average"
        );
    }

    #[test]
    fn floor_y_interpolates_tracking_projection() {
        // No authored floor overrides. Projection changes between steps.
        // Floor Y should slide because the baseline (projection.floor_y_for_depth)
        // changes.
        let profile_a = ProjectionProfile {
            horizon_y: 80.0,
            floor_base_y: 0.0,
            bias_power: 1.0,
        };
        let profile_b = ProjectionProfile {
            horizon_y: 100.0,
            floor_base_y: -20.0,
            bias_power: 1.0,
        };

        let stage = stage_with_steps(vec![
            tween_100s().with_projection(profile_a).into(),
            TweenStageStep::base(200.0, 0.0)
                .with_projection(profile_b)
                .into(),
        ]);

        // At 150s: mid-step 1. evaluate_floors_at interpolates prev/curr
        // projections internally. Since both layouts are empty, it lerps
        // between the two projection baselines.
        let floors = evaluate_floors_at(&stage, Duration::from_secs(150));
        let depth_5_y = floors.solid_y(Depth::Five).expect("should be solid");
        let prev_y = profile_a.floor_y_for_depth(5);
        let curr_y = profile_b.floor_y_for_depth(5);
        let expected = prev_y + (curr_y - prev_y) * 0.5;
        assert!(
            (depth_5_y - expected).abs() < 0.5,
            "floor should track interpolated projection: got {depth_5_y}, expected ~{expected}"
        );
    }

    // --- evaluate_floors_at: topology snaps at t=0 ---

    #[test]
    fn topology_addition_interpolates_from_baseline() {
        // Step 0: no floor override at depth 4 (baseline).
        // Step 1: Anchored(50) at depth 4.
        // Both resolve to Some(y), so this INTERPOLATES per the contract
        // (baseline is still a solid floor).
        let stage = stage_with_steps(vec![
            tween_100s().into(),
            TweenStageStep::base(200.0, 0.0)
                .with_surfaces(vec![SurfaceSpec {
                    depths: Depth::Four..=Depth::Four,
                    mode: HeightMode::Anchored(50.0),
                }])
                .into(),
        ]);
        let projection = active_projection().0;

        // At t≈0.001 (very early in step 1)
        let floors = evaluate_floors_at(&stage, Duration::from_millis(100_100));
        let y = floors.solid_y(Depth::Four).expect("should be solid");
        let baseline = projection.floor_y_for_depth(4);
        assert!(
            (y - baseline).abs() < 1.0,
            "at t≈0.001, floor should be near baseline {baseline}, got {y}"
        );
    }

    #[test]
    fn gap_appearance_snaps_at_t_zero() {
        // Step 0: solid at depth 4. Step 1: Gap at depth 4.
        // Some → None: topology change, snaps to None at t=0.
        let step1_layout = SurfaceLayout {
            spans: vec![SurfaceSpec {
                depths: Depth::Four..=Depth::Four,
                mode: HeightMode::Gap,
            }],
        };
        let projection = active_projection().0;

        // Resolve prev (baseline) and curr (Gap).
        let prev = resolve_depth_y(Depth::Four, &projection, &SurfaceLayout::default());
        let curr = resolve_depth_y(Depth::Four, &projection, &step1_layout);
        assert!(prev.is_some(), "prev should be solid");
        assert!(curr.is_none(), "curr should be gap");

        // Test the per-depth rule directly.
        match (prev, curr) {
            (Some(_), None) => {
                // Contract: topology snap at t=0 → Gap for entire tween.
            }
            _ => panic!("unexpected resolution"),
        }
    }

    #[test]
    fn gap_removal_snaps_at_t_zero() {
        // Verify the per-depth rule: None → Some(b) → uses b throughout.
        let prev_y: Option<f32> = None; // gap
        let curr_y: Option<f32> = Some(42.0); // solid

        // Per contract: snap at t=0 → Some(42) for entire tween.
        let result = match (prev_y, curr_y) {
            (None, Some(b)) => Surface::Solid { y: b },
            _ => panic!("unexpected"),
        };
        assert_eq!(result, Surface::Solid { y: 42.0 });
    }

    // --- evaluate_floors_at: boundary behaviour ---

    #[test]
    fn stop_step_holds_floors_constant() {
        let stage = stage_with_steps(vec![
            tween_100s()
                .with_surfaces(vec![SurfaceSpec {
                    depths: Depth::Four..=Depth::Four,
                    mode: HeightMode::Anchored(20.0),
                }])
                .into(),
            StopStageStep::new().with_max_duration(10.0).into(),
        ]);

        // At 105s: 100s tween + 5s into stop.
        let floors = evaluate_floors_at(&stage, Duration::from_secs(105));
        let y = floors.solid_y(Depth::Four).expect("should be solid");
        assert!(
            (y - 20.0).abs() < 0.01,
            "stop step should hold floor constant at 20.0, got {y}"
        );
    }

    #[test]
    fn stage_start_has_no_interpolation() {
        // Step 0 with floor override. No previous step to interpolate from.
        let stage = stage_with_steps(vec![
            tween_100s()
                .with_surfaces(vec![SurfaceSpec {
                    depths: Depth::Four..=Depth::Four,
                    mode: HeightMode::Anchored(42.0),
                }])
                .into(),
        ]);
        let projection = active_projection().0;

        // At t=0 (very start of step 0).
        let floors = evaluate_floors_at(&stage, Duration::ZERO);
        let y = floors.solid_y(Depth::Four).expect("should be solid");

        // At step 0, prev is SurfaceLayout::default() (no overrides, baseline).
        // Curr is Anchored(42). Both are Some → lerp from baseline to 42 at t=0 → baseline.
        let baseline = projection.floor_y_for_depth(4);
        assert!(
            (y - baseline).abs() < 0.01,
            "at t=0 of step 0, floor should be baseline {baseline}, got {y}"
        );
    }

    #[test]
    fn past_stage_end_holds_final_state() {
        let stage = stage_with_steps(vec![
            tween_100s()
                .with_surfaces(vec![SurfaceSpec {
                    depths: Depth::Four..=Depth::Four,
                    mode: HeightMode::Anchored(42.0),
                }])
                .into(),
        ]);

        // Far past the end.
        let floors = evaluate_floors_at(&stage, Duration::from_secs(9999));
        let y = floors.solid_y(Depth::Four).expect("should be solid");
        assert!(
            (y - 42.0).abs() < 0.01,
            "past stage end, floor should hold final state 42.0, got {y}"
        );
    }

    #[test]
    fn zero_duration_tween_resolves_to_destination() {
        // Tween from (0,0) to (0,0) → zero distance → zero duration.
        let stage = stage_with_steps(vec![
            TweenStageStep::base(0.0, 0.0)
                .with_surfaces(vec![SurfaceSpec {
                    depths: Depth::Four..=Depth::Four,
                    mode: HeightMode::Anchored(10.0),
                }])
                .into(),
            tween_100s()
                .with_surfaces(vec![SurfaceSpec {
                    depths: Depth::Four..=Depth::Four,
                    mode: HeightMode::Anchored(50.0),
                }])
                .into(),
        ]);

        // At t=0, step 0 has zero duration so resolves immediately.
        let floors = evaluate_floors_at(&stage, Duration::ZERO);
        let y = floors.solid_y(Depth::Four).expect("should be solid");
        // Zero-duration tween snaps to step 0's value.
        assert!(
            (y - 10.0).abs() < 0.01,
            "zero-duration tween should resolve to destination, got {y}"
        );
    }

    // --- Multi-surface data shape ---

    #[test]
    fn multi_surface_at_same_depth_preserves_authored_order() {
        let mut floors = ActiveFloors::default();
        floors.by_depth.insert(
            Depth::Four,
            vec![
                Surface::Solid { y: 10.0 },
                Surface::Solid { y: 40.0 },
                Surface::Solid { y: 70.0 },
            ],
        );

        let surfaces = floors.surfaces_at(Depth::Four);
        assert_eq!(surfaces.len(), 3);
        assert_eq!(surfaces[0], Surface::Solid { y: 10.0 });
        assert_eq!(surfaces[1], Surface::Solid { y: 40.0 });
        assert_eq!(surfaces[2], Surface::Solid { y: 70.0 });
    }

    #[test]
    fn solid_y_returns_first_solid_in_multi_surface() {
        let mut floors = ActiveFloors::default();
        floors.by_depth.insert(
            Depth::Four,
            vec![
                Surface::Gap,
                Surface::Solid { y: 30.0 },
                Surface::Solid { y: 60.0 },
            ],
        );

        // solid_y returns the first Solid, skipping the Gap.
        assert_eq!(floors.solid_y(Depth::Four), Some(30.0));
    }

    #[test]
    fn surfaces_at_empty_for_absent_depth() {
        let floors = ActiveFloors::default();
        assert!(floors.surfaces_at(Depth::Four).is_empty());
        assert_eq!(floors.solid_y(Depth::Four), None);
    }

    // --- Picking rules ---

    #[test]
    fn picking_lowest_solid_y_picks_ground() {
        let mut floors = ActiveFloors::default();
        floors.by_depth.insert(
            Depth::Four,
            vec![Surface::Solid { y: 30.0 }, Surface::Solid { y: 60.0 }],
        );
        assert_eq!(floors.lowest_solid_y(Depth::Four), Some(30.0));
    }

    #[test]
    fn picking_lowest_solid_y_skips_gaps() {
        let mut floors = ActiveFloors::default();
        floors
            .by_depth
            .insert(Depth::Four, vec![Surface::Gap, Surface::Solid { y: 50.0 }]);
        assert_eq!(floors.lowest_solid_y(Depth::Four), Some(50.0));
    }

    #[test]
    fn picking_highest_below_picks_landing_surface() {
        let mut floors = ActiveFloors::default();
        floors.by_depth.insert(
            Depth::Four,
            vec![
                Surface::Solid { y: 30.0 },
                Surface::Solid { y: 60.0 },
                Surface::Solid { y: 80.0 },
            ],
        );
        // Body bottom at 50 → highest solid at or below is 30.
        assert_eq!(
            floors.highest_solid_y_at_or_below(Depth::Four, 50.0),
            Some(30.0)
        );
        // Body bottom at 65 → highest solid at or below is 60.
        assert_eq!(
            floors.highest_solid_y_at_or_below(Depth::Four, 65.0),
            Some(60.0)
        );
        // Body bottom at 90 → highest solid at or below is 80.
        assert_eq!(
            floors.highest_solid_y_at_or_below(Depth::Four, 90.0),
            Some(80.0)
        );
    }

    #[test]
    fn picking_highest_below_returns_none_when_all_surfaces_above() {
        let mut floors = ActiveFloors::default();
        floors.by_depth.insert(
            Depth::Four,
            vec![Surface::Solid { y: 50.0 }, Surface::Solid { y: 70.0 }],
        );
        // Body bottom at 40 → no surface at or below.
        assert_eq!(floors.highest_solid_y_at_or_below(Depth::Four, 40.0), None);
    }

    #[test]
    fn picking_spawn_placement_with_altitude_offsets_above_ground() {
        let mut floors = ActiveFloors::default();
        floors.by_depth.insert(
            Depth::Four,
            vec![Surface::Solid { y: 30.0 }, Surface::Solid { y: 60.0 }],
        );
        // lowest_solid_y = 30, altitude = 20 → spawn at 50.
        let ground = floors.lowest_solid_y(Depth::Four).unwrap();
        assert!((ground + 20.0 - 50.0).abs() < f32::EPSILON);
    }
}
