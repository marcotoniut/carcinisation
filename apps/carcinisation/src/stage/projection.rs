//! Stage projection: maps discrete depth lanes to floor Y positions.
//!
//! The [`ProjectionProfile`] struct parameterises a perspective curve with a
//! horizon Y (depth 9), a floor-base Y (depth 1), and a bias exponent that
//! controls how depth bands are distributed between them.
//!
//! [`build_perspective_grid`] generates the full set of line segments for
//! a depth-perspective grid (horizontal floor lines + converging guide rays)
//! from a projection profile and viewport bounds.  Both the runtime debug
//! overlay (`depth_debug.rs`) and the editor preview consume this shared
//! geometry, adapting only the rendering backend.
//!
//! # V2 extension points
//!
//! `ProjectionProfile` is a plain data struct for V1.  V2 may wrap it in a
//! driver enum:
//!
//! ```ignore
//! enum ProjectionSource {
//!     Static(ProjectionProfile),
//!     CameraFollow { base: ProjectionProfile, sensitivity: f32 },
//!     Keyframed(Vec<(f32, ProjectionProfile)>),
//! }
//! ```
//!
//! The evaluator signature stays the same — callers always receive a
//! `ProjectionProfile` regardless of source.

use std::time::Duration;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    components::{CinematicStageStep, StopStageStep, TweenStageStep},
    data::{StageData, StageStep},
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEPTH_MIN: i8 = 1;
const DEPTH_MAX: i8 = 9;
const DEPTH_RANGE: f32 = (DEPTH_MAX - DEPTH_MIN) as f32; // 8.0

fn default_bias_power() -> f32 {
    3.0
}

/// Default horizon Y: 50 % of screen height (72.0 for 144 px screen).
const DEFAULT_HORIZON_Y: f32 = 72.0;

/// Default floor-base Y: −10 % of screen height (−14.4 for 144 px screen).
const DEFAULT_FLOOR_BASE_Y: f32 = -14.4;

// ---------------------------------------------------------------------------
// ProjectionProfile
// ---------------------------------------------------------------------------

/// Stage projection profile.  Determines how depth maps to floor Y.
///
/// All Y fields are in **carapace world-space pixel coordinates** (bottom-left
/// origin, Y-up).
#[derive(Clone, Copy, Debug, PartialEq, Reflect, Serialize, Deserialize)]
pub struct ProjectionProfile {
    /// Y position of the horizon line (depth 9 floor).
    pub horizon_y: f32,
    /// Y position of the nearest playfield floor line (depth 1).
    pub floor_base_y: f32,
    /// Exponent controlling depth compression.
    /// `1.0` = linear spacing, `3.0` = cubic (strong perspective).
    #[serde(default = "default_bias_power")]
    pub bias_power: f32,
}

impl Default for ProjectionProfile {
    fn default() -> Self {
        Self {
            horizon_y: DEFAULT_HORIZON_Y,
            floor_base_y: DEFAULT_FLOOR_BASE_Y,
            bias_power: default_bias_power(),
        }
    }
}

impl ProjectionProfile {
    /// Returns `true` if the profile satisfies all structural invariants:
    /// `horizon_y > floor_base_y` and `bias_power > 0`.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    /// Validate all structural invariants, returning a descriptive error on
    /// failure.  Works in both debug and release builds.
    ///
    /// Checks:
    /// - `horizon_y` must be finite
    /// - `floor_base_y` must be finite
    /// - `horizon_y` must be strictly above `floor_base_y`
    /// - `bias_power` must be finite and positive
    pub fn validate(&self) -> Result<(), String> {
        if !self.horizon_y.is_finite() {
            return Err(format!("horizon_y ({}) is not finite", self.horizon_y));
        }
        if !self.floor_base_y.is_finite() {
            return Err(format!(
                "floor_base_y ({}) is not finite",
                self.floor_base_y
            ));
        }
        if self.horizon_y <= self.floor_base_y {
            return Err(format!(
                "horizon_y ({}) must be above floor_base_y ({})",
                self.horizon_y, self.floor_base_y,
            ));
        }
        if !self.bias_power.is_finite() || self.bias_power <= 0.0 {
            return Err(format!(
                "bias_power ({}) must be finite and positive",
                self.bias_power,
            ));
        }
        Ok(())
    }

    /// Floor Y for a discrete depth (1–9).
    ///
    /// Normalises `d` to `t ∈ [0, 1]` where `0` = depth 9 (horizon) and
    /// `1` = depth 1 (foreground), applies `t^bias_power`, then lerps
    /// between `horizon_y` and `floor_base_y`.
    ///
    /// Depths outside 1–9 are extrapolated (depth 0 extends one step past
    /// `floor_base_y`).
    #[must_use]
    pub fn floor_y_for_depth(&self, d: i8) -> f32 {
        debug_assert!(
            self.is_valid(),
            "ProjectionProfile::floor_y_for_depth called on invalid profile: {self:?}"
        );
        let t = f32::from(DEPTH_MAX - d) / DEPTH_RANGE;
        let biased = t.abs().powf(self.bias_power).copysign(t);
        self.horizon_y + biased * (self.floor_base_y - self.horizon_y)
    }

    /// Convenience: floor Y for all depths 1–9 as a fixed-size array.
    /// Index `i` corresponds to depth `i + 1`.
    #[must_use]
    pub fn floor_y_array(&self) -> [f32; 9] {
        let mut out = [0.0; 9];
        for d in DEPTH_MIN..=DEPTH_MAX {
            out[(d - 1) as usize] = self.floor_y_for_depth(d);
        }
        out
    }

    /// Componentwise linear interpolation between two profiles.
    ///
    /// # Bias interpolation limitation
    ///
    /// `bias_power` is interpolated linearly, which does NOT produce
    /// perceptually linear projection transitions — the floor distribution
    /// curve shape changes non-uniformly as the exponent varies.  This is
    /// acceptable for V1 where bias values are typically constant across
    /// neighbouring steps.  Future work may interpolate evaluated floor
    /// curves directly (sample-then-lerp) instead of lerping parameters.
    #[must_use]
    pub fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        let result = Self {
            horizon_y: a.horizon_y + (b.horizon_y - a.horizon_y) * t,
            floor_base_y: a.floor_base_y + (b.floor_base_y - a.floor_base_y) * t,
            bias_power: a.bias_power + (b.bias_power - a.bias_power) * t,
        };
        debug_assert!(
            result.is_valid(),
            "ProjectionProfile::lerp produced invalid profile: {result:?} (a={a:?}, b={b:?}, t={t})"
        );
        result
    }
}

// ---------------------------------------------------------------------------
// Step progress + projection evaluation
// ---------------------------------------------------------------------------

/// Snapshot of stage progression at a given elapsed time.
///
/// Returned by [`walk_steps_at_elapsed`] and consumed by both camera position
/// and projection evaluation — keeping the step-walking logic in one place.
#[derive(Clone, Debug)]
pub struct StepProgressInfo {
    /// Index of the active step in `StageData.steps`.
    pub step_index: usize,
    /// Camera position at this moment (interpolated during tweens).
    pub camera_position: Vec2,
    /// Tween progress `[0, 1]` within the active step.
    /// `0.0` at tween start, `1.0` at tween end or for stop/cinematic steps.
    pub tween_progress: f32,
    /// Camera position at the start of the active step.
    pub step_start_position: Vec2,
}

/// Walk through stage steps up to `elapsed`, returning progress info.
///
/// Duration model matches the editor timeline: tween steps contribute
/// `distance / base_speed`, stop steps contribute `max_duration` (or zero
/// for infinite / None), cinematic steps contribute cutscene spawn sums.
#[must_use]
pub fn walk_steps_at_elapsed(stage_data: &StageData, elapsed: Duration) -> StepProgressInfo {
    let mut pos = stage_data.start_coordinates;
    let mut acc = Duration::ZERO;

    for (index, step) in stage_data.steps.iter().enumerate() {
        let step_start = pos;
        match step {
            StageStep::Tween(s) => {
                let dur = tween_duration(pos, s);
                if acc + dur > elapsed {
                    let t = if dur.is_zero() {
                        1.0
                    } else {
                        elapsed.saturating_sub(acc).as_secs_f32() / dur.as_secs_f32()
                    };
                    return StepProgressInfo {
                        step_index: index,
                        camera_position: pos.lerp(s.coordinates, t),
                        tween_progress: t,
                        step_start_position: step_start,
                    };
                }
                pos = s.coordinates;
                acc += dur;
            }
            StageStep::Stop(s) => {
                let dur = stop_step_duration(s);
                if acc + dur > elapsed {
                    return StepProgressInfo {
                        step_index: index,
                        camera_position: pos,
                        tween_progress: 1.0,
                        step_start_position: step_start,
                    };
                }
                acc += dur;
            }
            StageStep::Cinematic(s) => {
                let dur = cinematic_step_duration(s);
                if acc + dur > elapsed {
                    return StepProgressInfo {
                        step_index: index,
                        camera_position: pos,
                        tween_progress: 1.0,
                        step_start_position: step_start,
                    };
                }
                acc += dur;
            }
        }
    }

    // Past the end — return last position.
    StepProgressInfo {
        step_index: stage_data.steps.len().saturating_sub(1),
        camera_position: pos,
        tween_progress: 1.0,
        step_start_position: pos,
    }
}

/// Resolve the effective projection at a given step index.
///
/// Walks backwards from `step_index` to find the nearest step with a
/// `projection` override.  Falls back to `stage_data.projection`, then
/// to [`ProjectionProfile::default()`].
///
/// Projection is "sticky" — once set, it persists until the next override.
#[must_use]
pub fn effective_projection(stage_data: &StageData, step_index: usize) -> ProjectionProfile {
    // Walk backwards through steps.
    for i in (0..=step_index.min(stage_data.steps.len().saturating_sub(1))).rev() {
        let proj = match &stage_data.steps[i] {
            StageStep::Tween(s) => s.projection.as_ref(),
            StageStep::Stop(s) => s.projection.as_ref(),
            StageStep::Cinematic(_) => None,
        };
        if let Some(p) = proj {
            debug_assert!(
                p.is_valid(),
                "effective_projection found invalid profile at step {i}: {p:?}"
            );
            return *p;
        }
    }
    // Fall back to stage-level, then global default.
    let result = stage_data.projection.unwrap_or_default();
    debug_assert!(
        result.is_valid(),
        "effective_projection returned invalid profile at step {step_index}: {result:?}"
    );
    result
}

/// Minimum tween duration (in seconds) below which interpolation is skipped
/// and the target projection is snapped to immediately.  Prevents jitter from
/// near-zero-length tweens where `t` swings wildly.
const MIN_INTERPOLATION_DURATION_SECS: f32 = 0.01;

/// Evaluate the interpolated projection at a given elapsed time.
///
/// During tween steps, linearly interpolates between two explicitly resolved
/// profiles:
/// - `prev` = `effective_projection(step_index - 1)` (or stage/global default
///   for the first step)
/// - `curr` = `effective_projection(step_index)`
///
/// During stop/cinematic steps, holds `curr` constant.
///
/// Tweens shorter than [`MIN_INTERPOLATION_DURATION_SECS`] snap directly to
/// `curr` to avoid jitter from near-zero denominators.
///
/// # V2 extension
///
/// TODO: Projection evaluation may later be derived directly from
/// [`StepProgressInfo`] to ensure strict alignment between camera position
/// and projection state.  This would allow callers to obtain both camera
/// position and projection from a single step-walk, avoiding redundant
/// traversal and guaranteeing consistency.
#[must_use]
pub fn evaluate_projection_at(stage_data: &StageData, elapsed: Duration) -> ProjectionProfile {
    let info = walk_steps_at_elapsed(stage_data, elapsed);

    // Resolve the projection at the current step (sticky carry-forward).
    let curr = effective_projection(stage_data, info.step_index);

    // Interpolate only during tween steps with meaningful progress.
    if info.tween_progress < 1.0
        && let Some(StageStep::Tween(tween)) = stage_data.steps.get(info.step_index)
    {
        // Skip interpolation for extremely short tweens.
        let dur = tween_duration(info.step_start_position, tween);
        if dur.as_secs_f32() >= MIN_INTERPOLATION_DURATION_SECS {
            // Resolve the projection that was active before this step.
            let prev = if info.step_index > 0 {
                effective_projection(stage_data, info.step_index - 1)
            } else {
                stage_data.projection.unwrap_or_default()
            };
            return ProjectionProfile::lerp(&prev, &curr, info.tween_progress);
        }
    }

    curr
}

// ---------------------------------------------------------------------------
// Stage-level projection validation
// ---------------------------------------------------------------------------

/// Validate all projection profiles in a [`StageData`].
///
/// Checks the stage-level default (if present) and every step-level override.
/// Returns `Ok(())` if all profiles are valid, or an error with the location
/// and reason of the first invalid profile found.
///
/// Call this at stage load time (e.g. `on_stage_startup`) to fail fast on
/// malformed authored data.  Works in release builds.
pub fn validate_stage_projections(stage_data: &StageData) -> Result<(), String> {
    if let Some(ref p) = stage_data.projection {
        p.validate()
            .map_err(|e| format!("Invalid stage-level projection: {e}"))?;
    }
    for (i, step) in stage_data.steps.iter().enumerate() {
        let proj = match step {
            StageStep::Tween(s) => s.projection.as_ref(),
            StageStep::Stop(s) => s.projection.as_ref(),
            StageStep::Cinematic(_) => None,
        };
        if let Some(p) = proj {
            p.validate()
                .map_err(|e| format!("Invalid projection at step {i}: {e}"))?;
        }
    }
    Ok(())
}

// --- Step duration helpers (shared between runtime and editor) ---

fn tween_duration(current_position: Vec2, step: &TweenStageStep) -> Duration {
    let distance = step.coordinates.distance(current_position);
    let speed = step.base_speed.max(0.0001);
    Duration::from_secs_f32(distance / speed)
}

fn stop_step_duration(step: &StopStageStep) -> Duration {
    step.max_duration.unwrap_or(Duration::ZERO)
}

fn cinematic_step_duration(step: &CinematicStageStep) -> Duration {
    match step {
        CinematicStageStep::CutsceneAnimationSpawn(s) => s
            .spawns
            .iter()
            .fold(Duration::ZERO, |acc, sp| acc + sp.duration),
    }
}

// ---------------------------------------------------------------------------
// Grid geometry (extracted from depth_debug.rs)
// ---------------------------------------------------------------------------

/// A line segment in the perspective grid.
///
/// Coordinates are in the **caller's** coordinate system — the shared builder
/// does not apply any coordinate transform.
#[derive(Clone, Debug)]
pub struct GridLineSegment {
    pub start: Vec2,
    pub end: Vec2,
    pub start_rgba: [f32; 4],
    pub end_rgba: [f32; 4],
}

/// Tuning parameters for [`build_perspective_grid`].
///
/// Defaults match the existing `depth_debug.rs` constants.
#[derive(Clone, Debug)]
pub struct GridParams {
    /// Number of guide rays (should be odd so the centre ray is exact).
    pub guide_ray_count: u32,
    /// Major-ray interval: every Nth ray drawn at full intensity.
    pub major_ray_interval: u32,
    /// Mu-law concentration parameter.
    pub mu: f32,
    /// Alpha for horizontal depth lines.
    pub horizontal_alpha: f32,
    /// Alpha for major guide rays.
    pub major_ray_alpha: f32,
    /// Alpha for minor guide rays.
    pub minor_ray_alpha: f32,
}

impl Default for GridParams {
    fn default() -> Self {
        Self {
            guide_ray_count: 35,
            major_ray_interval: 4,
            mu: 35.0,
            horizontal_alpha: 0.85,
            major_ray_alpha: 0.80,
            minor_ray_alpha: 0.45,
        }
    }
}

/// Complete perspective grid geometry for one projection state.
#[derive(Clone, Debug, Default)]
pub struct PerspectiveGrid {
    /// One horizontal line per depth (1–9).
    pub depth_lines: Vec<GridLineSegment>,
    /// Guide ray segments — multiple segments per ray, one per depth band,
    /// plus a final segment to the viewport boundary.
    pub guide_ray_segments: Vec<GridLineSegment>,
}

/// Depth brightness: `1.0` at depth 1 (brightest), `0.2` at depth 9 (dimmest).
///
/// Extracted from `depth_debug.rs::depth_brightness`.
#[must_use]
pub fn depth_brightness(d: i8) -> f32 {
    1.0 - f32::from(d - 1) / 8.0 * 0.8
}

/// Grid colour RGBA from brightness and alpha.
///
/// Purple base `(0.6, 0.15, 0.9)` modulated by brightness.
/// Alpha modulated by `brightness²` for perceptual fade toward the horizon.
///
/// Extracted from `depth_debug.rs::grid_color`.
#[must_use]
pub fn grid_color_rgba(brightness: f32, alpha: f32) -> [f32; 4] {
    [
        0.6 * brightness,
        0.15 * brightness,
        0.9 * brightness,
        alpha * brightness * brightness,
    ]
}

/// Build the full perspective grid geometry from floor positions and viewport.
///
/// # Arguments
///
/// * `floors` — `(depth, floor_y)` pairs in the **caller's coordinate system**,
///   sorted with depth 9 first (descending by depth).  Only depths 1–9 are used.
/// * `viewport` — visible rectangle in the caller's coordinate system.
/// * `vanish_x` — X of the vanishing point (typically viewport centre X).
/// * `params` — grid tuning parameters (see [`GridParams::default`]).
///
/// # Returns
///
/// [`PerspectiveGrid`] with all segments in the **same coordinate system** as
/// the inputs.  The caller is responsible for any coordinate transform and for
/// choosing a rendering backend (gizmos, lyon shapes, etc.).
///
/// # Algorithm
///
/// Direct extraction of `depth_debug.rs::draw_depth_grid_background` (the
/// mu-law guide ray algorithm).  See that module's doc comments for the full
/// mathematical derivation.
pub fn build_perspective_grid(
    floors: &[(i8, f32)],
    viewport: Rect,
    vanish_x: f32,
    params: &GridParams,
) -> PerspectiveGrid {
    let mut grid = PerspectiveGrid::default();

    if floors.is_empty() {
        return grid;
    }

    // --- Horizontal depth lines ---
    for &(d, floor_y) in floors {
        let color = grid_color_rgba(depth_brightness(d), params.horizontal_alpha);
        grid.depth_lines.push(GridLineSegment {
            start: Vec2::new(viewport.min.x, floor_y),
            end: Vec2::new(viewport.max.x, floor_y),
            start_rgba: color,
            end_rgba: color,
        });
    }

    if floors.len() < 2 {
        return grid;
    }

    // --- Converging guide rays ---
    let vanish_y = floors[0].1; // depth 9 = horizon

    let n = params.guide_ray_count;
    let centre_idx = n / 2;
    let theta_max = std::f32::consts::FRAC_PI_2 * n as f32 / (n as f32 + 1.0);
    let ln_1_plus_mu = (1.0 + params.mu).ln();

    for i in 0..n {
        // Mu-law remap: uniform u → edge-concentrated b.
        let u = 2.0 * i as f32 / (n - 1) as f32 - 1.0;
        let b = u.signum() * (1.0 + params.mu * u.abs()).ln() / ln_1_plus_mu;
        let theta = b * theta_max;

        // θ = 0 → straight down; positive → right; negative → left.
        let dx = theta.sin();
        let dy = -theta.cos();

        let is_major = i == centre_idx || i % params.major_ray_interval == 0;
        let alpha = if is_major {
            params.major_ray_alpha
        } else {
            params.minor_ray_alpha
        };

        // Clip to viewport bottom and nearer side edge.
        let t_bottom = if dy.abs() > f32::EPSILON {
            (viewport.min.y - vanish_y) / dy
        } else {
            f32::MAX
        };
        let t_side = if dx.abs() > f32::EPSILON {
            let edge = if dx > 0.0 {
                viewport.max.x
            } else {
                viewport.min.x
            };
            (edge - vanish_x) / dx
        } else {
            f32::MAX
        };
        let t_end = t_bottom.min(t_side);
        if t_end <= 0.0 {
            continue;
        }

        let endpoint = Vec2::new(vanish_x + t_end * dx, vanish_y + t_end * dy);

        // Segmented by depth bands.
        let mut prev = Vec2::new(vanish_x, vanish_y);
        let mut prev_d = floors[0].0;

        for &(d, floor_y) in floors.iter().skip(1) {
            if dy.abs() < f32::EPSILON {
                break;
            }
            let t = (floor_y - vanish_y) / dy;
            if t <= 0.0 || t > t_end {
                continue;
            }
            let here = Vec2::new(vanish_x + t * dx, floor_y);

            grid.guide_ray_segments.push(GridLineSegment {
                start: prev,
                end: here,
                start_rgba: grid_color_rgba(depth_brightness(prev_d), alpha),
                end_rgba: grid_color_rgba(depth_brightness(d), alpha),
            });

            prev = here;
            prev_d = d;
        }

        // Final segment to viewport boundary.
        let c_last = grid_color_rgba(depth_brightness(prev_d), alpha);
        grid.guide_ray_segments.push(GridLineSegment {
            start: prev,
            end: endpoint,
            start_rgba: c_last,
            end_rgba: c_last,
        });
    }

    grid
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Validation tests ---

    #[test]
    fn default_profile_is_valid() {
        assert!(ProjectionProfile::default().is_valid());
    }

    #[test]
    fn valid_profile() {
        let p = ProjectionProfile {
            horizon_y: 80.0,
            floor_base_y: 0.0,
            bias_power: 3.0,
        };
        assert!(p.is_valid());
    }

    #[test]
    fn invalid_inverted_horizon() {
        let p = ProjectionProfile {
            horizon_y: 0.0,
            floor_base_y: 80.0,
            bias_power: 3.0,
        };
        assert!(!p.is_valid());
    }

    #[test]
    fn invalid_zero_bias() {
        let p = ProjectionProfile {
            horizon_y: 80.0,
            floor_base_y: 0.0,
            bias_power: 0.0,
        };
        assert!(!p.is_valid());
    }

    #[test]
    fn invalid_nan_values() {
        let p = ProjectionProfile {
            horizon_y: f32::NAN,
            floor_base_y: 0.0,
            bias_power: 3.0,
        };
        assert!(!p.is_valid());
    }

    // --- validate() error messages ---

    #[test]
    fn validate_inverted_describes_fields() {
        let p = ProjectionProfile {
            horizon_y: 10.0,
            floor_base_y: 80.0,
            bias_power: 3.0,
        };
        let err = p.validate().unwrap_err();
        assert!(err.contains("horizon_y"), "should name horizon_y: {err}");
        assert!(
            err.contains("floor_base_y"),
            "should name floor_base_y: {err}"
        );
        assert!(err.contains("10"), "should include value: {err}");
        assert!(err.contains("80"), "should include value: {err}");
    }

    #[test]
    fn validate_zero_bias_describes_field() {
        let p = ProjectionProfile {
            horizon_y: 80.0,
            floor_base_y: 0.0,
            bias_power: 0.0,
        };
        let err = p.validate().unwrap_err();
        assert!(err.contains("bias_power"), "should name field: {err}");
    }

    #[test]
    fn validate_nan_horizon_describes_field() {
        let p = ProjectionProfile {
            horizon_y: f32::NAN,
            floor_base_y: 0.0,
            bias_power: 3.0,
        };
        let err = p.validate().unwrap_err();
        assert!(err.contains("horizon_y"), "should name field: {err}");
        assert!(err.contains("finite"), "should explain reason: {err}");
    }

    // --- validate_stage_projections ---

    use super::validate_stage_projections;

    #[test]
    fn validate_stage_no_projections_ok() {
        let stage = make_stage(vec![tween_step(100.0, 0.0, 1.0)]);
        assert!(validate_stage_projections(&stage).is_ok());
    }

    #[test]
    fn validate_stage_valid_projections_ok() {
        let mut stage = make_stage(vec![tween_step_with_projection(
            100.0,
            0.0,
            1.0,
            profile_a(),
        )]);
        stage.projection = Some(profile_b());
        assert!(validate_stage_projections(&stage).is_ok());
    }

    #[test]
    fn validate_stage_invalid_stage_level() {
        let mut stage = make_stage(vec![tween_step(100.0, 0.0, 1.0)]);
        stage.projection = Some(ProjectionProfile {
            horizon_y: 0.0,
            floor_base_y: 80.0,
            bias_power: 3.0,
        });
        let err = validate_stage_projections(&stage).unwrap_err();
        assert!(err.contains("stage-level"), "should locate error: {err}");
    }

    #[test]
    fn validate_stage_invalid_step_projection() {
        let bad = ProjectionProfile {
            horizon_y: 80.0,
            floor_base_y: 0.0,
            bias_power: -1.0,
        };
        let stage = make_stage(vec![
            tween_step(100.0, 0.0, 1.0),
            tween_step_with_projection(200.0, 0.0, 1.0, bad),
        ]);
        let err = validate_stage_projections(&stage).unwrap_err();
        assert!(err.contains("step 1"), "should identify step index: {err}");
        assert!(err.contains("bias_power"), "should name field: {err}");
    }

    // --- ProjectionProfile evaluator tests ---

    #[test]
    fn floor_y_boundary_depths() {
        let p = ProjectionProfile {
            horizon_y: 72.0,
            floor_base_y: -14.4,
            bias_power: 3.0,
        };
        // Depth 9 (t=0) → horizon_y.
        assert!((p.floor_y_for_depth(9) - 72.0).abs() < 1e-5);
        // Depth 1 (t=1) → floor_base_y.
        assert!(
            (p.floor_y_for_depth(1) - (-14.4)).abs() < 1e-5,
            "depth 1 got {}, expected -14.4",
            p.floor_y_for_depth(1)
        );
    }

    #[test]
    fn floor_y_linear_bias_evenly_spaced() {
        let p = ProjectionProfile {
            horizon_y: 80.0,
            floor_base_y: 0.0,
            bias_power: 1.0,
        };
        // With bias=1, depths should be evenly spaced.
        let y5 = p.floor_y_for_depth(5);
        let y3 = p.floor_y_for_depth(3);
        let y7 = p.floor_y_for_depth(7);
        // d5: t = (9-5)/8 = 0.5  → y = 80 + 0.5*(0-80) = 40
        assert!((y5 - 40.0).abs() < 0.001);
        // d3: t = (9-3)/8 = 0.75 → y = 80 + 0.75*(0-80) = 20
        assert!((y3 - 20.0).abs() < 0.001);
        // d7: t = (9-7)/8 = 0.25 → y = 80 + 0.25*(0-80) = 60
        assert!((y7 - 60.0).abs() < 0.001);
        // Equal spacing: y7-y5 == y5-y3.
        assert!(((y7 - y5) - (y5 - y3)).abs() < 0.001);
    }

    #[test]
    fn floor_y_cubic_compresses_horizon() {
        let p = ProjectionProfile {
            horizon_y: 80.0,
            floor_base_y: 0.0,
            bias_power: 3.0,
        };
        // Cubic bias should compress depths near horizon (high d)
        // and spread depths near foreground (low d).
        let y8 = p.floor_y_for_depth(8); // t=1/8=0.125, biased=0.00195
        let y2 = p.floor_y_for_depth(2); // t=7/8=0.875, biased=0.6699
        // y8 should be close to horizon (80), y2 should be much closer to 0.
        assert!(y8 > 70.0, "depth 8 should be near horizon, got {y8}");
        assert!(y2 < 30.0, "depth 2 should be near floor, got {y2}");
    }

    #[test]
    fn floor_y_depth_0_extrapolates() {
        let p = ProjectionProfile {
            horizon_y: 80.0,
            floor_base_y: 0.0,
            bias_power: 1.0,
        };
        // Depth 0: t = (9-0)/8 = 1.125, beyond [0,1].
        let y0 = p.floor_y_for_depth(0);
        let y1 = p.floor_y_for_depth(1);
        // Should extrapolate past floor_base_y.
        assert!(
            y0 < y1,
            "depth 0 should extrapolate below depth 1: y0={y0}, y1={y1}"
        );
    }

    #[test]
    fn floor_y_array_length_and_consistency() {
        let p = ProjectionProfile::default();
        let arr = p.floor_y_array();
        assert_eq!(arr.len(), 9);
        for d in 1..=9i8 {
            assert!(
                (arr[(d - 1) as usize] - p.floor_y_for_depth(d)).abs() < f32::EPSILON,
                "array index {} mismatch",
                d - 1,
            );
        }
    }

    #[test]
    fn lerp_boundaries() {
        let a = ProjectionProfile {
            horizon_y: 50.0,
            floor_base_y: 0.0,
            bias_power: 1.0,
        };
        let b = ProjectionProfile {
            horizon_y: 100.0,
            floor_base_y: 20.0,
            bias_power: 5.0,
        };
        let at0 = ProjectionProfile::lerp(&a, &b, 0.0);
        let at1 = ProjectionProfile::lerp(&a, &b, 1.0);
        let mid = ProjectionProfile::lerp(&a, &b, 0.5);

        assert_eq!(at0, a);
        assert_eq!(at1, b);
        assert!((mid.horizon_y - 75.0).abs() < f32::EPSILON);
        assert!((mid.floor_base_y - 10.0).abs() < f32::EPSILON);
        assert!((mid.bias_power - 3.0).abs() < f32::EPSILON);
    }

    // --- Grid helper tests ---

    #[test]
    fn depth_brightness_range() {
        assert!((depth_brightness(1) - 1.0).abs() < f32::EPSILON);
        assert!((depth_brightness(9) - 0.2).abs() < f32::EPSILON);
        // Monotonically decreasing.
        for d in 1..9i8 {
            assert!(depth_brightness(d) > depth_brightness(d + 1));
        }
    }

    #[test]
    fn grid_color_rgba_at_full_brightness() {
        let c = grid_color_rgba(1.0, 1.0);
        assert!((c[0] - 0.6).abs() < f32::EPSILON);
        assert!((c[1] - 0.15).abs() < f32::EPSILON);
        assert!((c[2] - 0.9).abs() < f32::EPSILON);
        assert!((c[3] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn grid_color_rgba_brightness_squared_alpha() {
        let brightness = 0.5;
        let alpha = 0.8;
        let c = grid_color_rgba(brightness, alpha);
        // Alpha should be alpha * brightness^2.
        let expected_alpha = alpha * brightness * brightness;
        assert!((c[3] - expected_alpha).abs() < f32::EPSILON);
    }

    // --- build_perspective_grid tests ---

    fn test_floors() -> Vec<(i8, f32)> {
        let profile = ProjectionProfile {
            horizon_y: 100.0,
            floor_base_y: 0.0,
            bias_power: 1.0, // linear for predictable values
        };
        let mut floors: Vec<(i8, f32)> =
            (1..=9).map(|d| (d, profile.floor_y_for_depth(d))).collect();
        floors.sort_by_key(|&(d, _)| std::cmp::Reverse(d));
        floors
    }

    fn test_viewport() -> Rect {
        Rect::new(0.0, -20.0, 160.0, 124.0)
    }

    #[test]
    fn grid_depth_line_count() {
        let floors = test_floors();
        let grid = build_perspective_grid(&floors, test_viewport(), 80.0, &GridParams::default());
        assert_eq!(grid.depth_lines.len(), 9, "one horizontal line per depth");
    }

    #[test]
    fn grid_depth_lines_match_floor_values() {
        let floors = test_floors();
        let grid = build_perspective_grid(&floors, test_viewport(), 80.0, &GridParams::default());
        for (seg, &(_, floor_y)) in grid.depth_lines.iter().zip(floors.iter()) {
            assert!(
                (seg.start.y - floor_y).abs() < f32::EPSILON,
                "depth line Y should match floor value"
            );
            assert!(
                (seg.end.y - floor_y).abs() < f32::EPSILON,
                "depth line Y should match floor value"
            );
        }
    }

    #[test]
    fn grid_depth_lines_span_viewport_width() {
        let vp = test_viewport();
        let floors = test_floors();
        let grid = build_perspective_grid(&floors, vp, 80.0, &GridParams::default());
        for seg in &grid.depth_lines {
            assert!(
                (seg.start.x - vp.min.x).abs() < f32::EPSILON,
                "line start X should be viewport left"
            );
            assert!(
                (seg.end.x - vp.max.x).abs() < f32::EPSILON,
                "line end X should be viewport right"
            );
        }
    }

    #[test]
    fn grid_guide_rays_present() {
        let floors = test_floors();
        let grid = build_perspective_grid(&floors, test_viewport(), 80.0, &GridParams::default());
        // With 9 depths and 35 rays, each ray has up to 9 segments (8 bands + final).
        // Some may be clipped. Must have at least one segment per non-clipped ray.
        assert!(
            !grid.guide_ray_segments.is_empty(),
            "guide rays should produce segments"
        );
    }

    #[test]
    fn grid_guide_ray_segments_within_viewport() {
        let vp = test_viewport();
        let floors = test_floors();
        let grid = build_perspective_grid(&floors, vp, 80.0, &GridParams::default());
        let tolerance = 0.01;
        for seg in &grid.guide_ray_segments {
            assert!(
                seg.end.x >= vp.min.x - tolerance && seg.end.x <= vp.max.x + tolerance,
                "segment end X ({}) outside viewport [{}, {}]",
                seg.end.x,
                vp.min.x,
                vp.max.x
            );
            assert!(
                seg.end.y >= vp.min.y - tolerance && seg.end.y <= vp.max.y + tolerance,
                "segment end Y ({}) outside viewport [{}, {}]",
                seg.end.y,
                vp.min.y,
                vp.max.y
            );
        }
    }

    #[test]
    fn grid_guide_rays_originate_at_vanishing_point() {
        let floors = test_floors();
        let vanish_x = 80.0;
        let grid =
            build_perspective_grid(&floors, test_viewport(), vanish_x, &GridParams::default());

        let vanish_y = floors[0].1; // depth 9 = horizon
        // The first segment of each ray should start at the vanishing point.
        // Rays are emitted sequentially — the first segment of each new ray starts at VP.
        let mut found_vp_starts = 0;
        for seg in &grid.guide_ray_segments {
            if (seg.start.x - vanish_x).abs() < f32::EPSILON
                && (seg.start.y - vanish_y).abs() < f32::EPSILON
            {
                found_vp_starts += 1;
            }
        }
        // Should have at least guide_ray_count rays starting at VP.
        // Some may be fully clipped (t_end <= 0), so allow fewer.
        assert!(
            found_vp_starts >= 30,
            "expected >=30 rays starting at VP, found {found_vp_starts}"
        );
    }

    #[test]
    fn grid_empty_floors() {
        let grid = build_perspective_grid(&[], test_viewport(), 80.0, &GridParams::default());
        assert!(grid.depth_lines.is_empty());
        assert!(grid.guide_ray_segments.is_empty());
    }

    #[test]
    fn grid_single_floor_no_rays() {
        let floors = vec![(5, 50.0)];
        let grid = build_perspective_grid(&floors, test_viewport(), 80.0, &GridParams::default());
        assert_eq!(grid.depth_lines.len(), 1);
        // Need at least 2 floors for guide rays.
        assert!(grid.guide_ray_segments.is_empty());
    }

    // --- Step progress + projection evaluation tests ---

    use super::{effective_projection, evaluate_projection_at, walk_steps_at_elapsed};
    use crate::stage::components::{StopStageStep, TweenStageStep};
    use crate::stage::data::{SkyboxData, StageData, StageStep};

    fn make_stage(steps: Vec<StageStep>) -> StageData {
        StageData {
            name: "test".into(),
            background_path: String::new(),
            music_path: String::new(),
            skybox: SkyboxData {
                path: String::new(),
                frames: 1,
            },
            start_coordinates: Vec2::ZERO,
            spawns: vec![],
            steps,
            on_start_transition_o: None,
            on_end_transition_o: None,
            gravity: None,
            projection: None,
            checkpoint: None,
        }
    }

    fn tween_step(x: f32, y: f32, speed: f32) -> StageStep {
        StageStep::Tween(TweenStageStep {
            coordinates: Vec2::new(x, y),
            base_speed: speed,
            spawns: vec![],
            floor_depths: None,
            projection: None,
        })
    }

    fn tween_step_with_projection(x: f32, y: f32, speed: f32, p: ProjectionProfile) -> StageStep {
        StageStep::Tween(TweenStageStep {
            coordinates: Vec2::new(x, y),
            base_speed: speed,
            spawns: vec![],
            floor_depths: None,
            projection: Some(p),
        })
    }

    fn stop_step(duration_secs: f32) -> StageStep {
        StageStep::Stop(StopStageStep::new().with_max_duration(duration_secs))
    }

    fn profile_a() -> ProjectionProfile {
        ProjectionProfile {
            horizon_y: 72.0,
            floor_base_y: -14.0,
            bias_power: 3.0,
        }
    }

    fn profile_b() -> ProjectionProfile {
        ProjectionProfile {
            horizon_y: 96.0,
            floor_base_y: 10.0,
            bias_power: 3.0,
        }
    }

    // --- walk_steps_at_elapsed ---

    #[test]
    fn walk_at_zero_returns_start() {
        let stage = make_stage(vec![tween_step(100.0, 0.0, 1.0)]);
        let info = walk_steps_at_elapsed(&stage, Duration::ZERO);
        assert_eq!(info.step_index, 0);
        assert!((info.camera_position - Vec2::ZERO).length() < 0.01);
        assert!(info.tween_progress < 0.01);
    }

    #[test]
    fn walk_mid_tween() {
        // Tween from (0,0) to (100,0) at speed 1.0 → 100s duration.
        let stage = make_stage(vec![tween_step(100.0, 0.0, 1.0)]);
        let info = walk_steps_at_elapsed(&stage, Duration::from_secs(50));
        assert_eq!(info.step_index, 0);
        assert!((info.camera_position.x - 50.0).abs() < 0.1);
        assert!((info.tween_progress - 0.5).abs() < 0.01);
    }

    #[test]
    fn walk_past_end_returns_last_position() {
        let stage = make_stage(vec![tween_step(100.0, 0.0, 1.0)]);
        let info = walk_steps_at_elapsed(&stage, Duration::from_secs(999));
        assert!((info.camera_position.x - 100.0).abs() < 0.01);
        assert!((info.tween_progress - 1.0).abs() < 0.01);
    }

    #[test]
    fn walk_stop_step_holds_position() {
        let stage = make_stage(vec![tween_step(100.0, 0.0, 1.0), stop_step(10.0)]);
        // 100s tween + 5s into stop.
        let info = walk_steps_at_elapsed(&stage, Duration::from_secs(105));
        assert_eq!(info.step_index, 1);
        assert!((info.camera_position.x - 100.0).abs() < 0.01);
        assert!((info.tween_progress - 1.0).abs() < 0.01);
    }

    // --- effective_projection ---

    #[test]
    fn effective_projection_falls_back_to_default() {
        let stage = make_stage(vec![tween_step(100.0, 0.0, 1.0)]);
        let p = effective_projection(&stage, 0);
        assert_eq!(p, ProjectionProfile::default());
    }

    #[test]
    fn effective_projection_uses_stage_default() {
        let mut stage = make_stage(vec![tween_step(100.0, 0.0, 1.0)]);
        stage.projection = Some(profile_a());
        let p = effective_projection(&stage, 0);
        assert_eq!(p, profile_a());
    }

    #[test]
    fn effective_projection_step_overrides_stage() {
        let mut stage = make_stage(vec![tween_step_with_projection(
            100.0,
            0.0,
            1.0,
            profile_b(),
        )]);
        stage.projection = Some(profile_a());
        let p = effective_projection(&stage, 0);
        assert_eq!(p, profile_b());
    }

    #[test]
    fn effective_projection_sticky_carry_forward() {
        let stage = make_stage(vec![
            tween_step_with_projection(100.0, 0.0, 1.0, profile_a()),
            tween_step(200.0, 0.0, 1.0), // no projection
            tween_step(300.0, 0.0, 1.0), // no projection
        ]);
        // Step 2 has no projection; should inherit from step 0.
        let p = effective_projection(&stage, 2);
        assert_eq!(p, profile_a());
    }

    #[test]
    fn effective_projection_later_override_wins() {
        let stage = make_stage(vec![
            tween_step_with_projection(100.0, 0.0, 1.0, profile_a()),
            tween_step(200.0, 0.0, 1.0),
            tween_step_with_projection(300.0, 0.0, 1.0, profile_b()),
        ]);
        assert_eq!(effective_projection(&stage, 0), profile_a());
        assert_eq!(effective_projection(&stage, 1), profile_a()); // inherited
        assert_eq!(effective_projection(&stage, 2), profile_b()); // overridden
    }

    // --- evaluate_projection_at ---

    #[test]
    fn evaluate_projection_at_constant_when_no_overrides() {
        let stage = make_stage(vec![
            tween_step(100.0, 0.0, 1.0),
            tween_step(200.0, 0.0, 1.0),
        ]);
        let p0 = evaluate_projection_at(&stage, Duration::ZERO);
        let p1 = evaluate_projection_at(&stage, Duration::from_secs(50));
        let p2 = evaluate_projection_at(&stage, Duration::from_secs(150));
        // All should be the global default.
        assert_eq!(p0, ProjectionProfile::default());
        assert_eq!(p1, ProjectionProfile::default());
        assert_eq!(p2, ProjectionProfile::default());
    }

    #[test]
    fn evaluate_projection_at_interpolates_mid_tween() {
        // Step 0: profile_a. Step 1: profile_b.
        // Tween from (0,0) to (100,0) at speed 1 = 100s.
        // Then tween from (100,0) to (200,0) at speed 1 = 100s.
        let stage = make_stage(vec![
            tween_step_with_projection(100.0, 0.0, 1.0, profile_a()),
            tween_step_with_projection(200.0, 0.0, 1.0, profile_b()),
        ]);
        // At 150s: midway through step 1, t=0.5.
        let p = evaluate_projection_at(&stage, Duration::from_secs(150));
        let expected = ProjectionProfile::lerp(&profile_a(), &profile_b(), 0.5);
        assert!((p.horizon_y - expected.horizon_y).abs() < 0.1);
        assert!((p.floor_base_y - expected.floor_base_y).abs() < 0.1);
    }

    #[test]
    fn evaluate_projection_at_holds_during_stop() {
        let stage = make_stage(vec![
            tween_step_with_projection(100.0, 0.0, 1.0, profile_a()),
            stop_step(10.0),
        ]);
        // 105s: 100s tween + 5s into stop.
        let p = evaluate_projection_at(&stage, Duration::from_secs(105));
        // Stop inherits from step 0 (sticky).
        assert_eq!(p, profile_a());
    }
}
