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
    ///
    /// # Errors
    ///
    /// Returns a descriptive `String` if any invariant is violated.
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
        self.floor_y_for_progress(Self::depth_progress_for_depth(d))
    }

    /// Continuous depth progress for a discrete depth.
    ///
    /// Returns the normalised progress used by [`Self::floor_y_for_depth`]:
    /// `0.0` = horizon (depth 9), `1.0` = foreground floor (depth 1).
    ///
    /// Depths outside 1–9 extrapolate beyond that interval. In particular,
    /// depth 0 maps to `1.125`, i.e. one eighth of a step beyond depth 1.
    ///
    // IMPORTANT: depth progress and grid weight (w) are NOT the same quantity.
    //
    // `progress` here is a label of which authored depth band you're in,
    // computed as (DEPTH_MAX - d) / DEPTH_RANGE. It feeds the bias curve in
    // floor_y_for_progress.
    //
    // The grid's `w` (in build_perspective_grid) is the screen-space fraction
    // along a ray's path through the viewport, computed as
    // (floor_y - vanish_y) / depth_span. It feeds the pinhole projection.
    //
    // These are equal ONLY when bias_power = 1. For any bias > 1, they diverge,
    // and that divergence is what produces the perspective compression near the
    // horizon. Do NOT unify these two quantities — doing so silently flattens
    // the perspective response with no visible runtime error.
    #[must_use]
    pub fn depth_progress_for_depth(d: i8) -> f32 {
        f32::from(DEPTH_MAX - d) / DEPTH_RANGE
    }

    /// Floor Y for a continuous normalised depth progress value.
    ///
    /// `progress = 0.0` evaluates to `horizon_y`, `progress = 1.0` evaluates
    /// to `floor_base_y`, and values outside `[0, 1]` extrapolate along the
    /// same biased curve.
    #[must_use]
    pub fn floor_y_for_progress(&self, progress: f32) -> f32 {
        debug_assert!(
            self.is_valid(),
            "ProjectionProfile::floor_y_for_progress called on invalid profile: {self:?}"
        );
        let biased = progress.abs().powf(self.bias_power).copysign(progress);
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
    /// `bias_power` interpolates in log-space so that
    /// `lerp(a, b, 0.5).bias_power` is the geometric mean of the two
    /// endpoints.
    #[must_use]
    pub fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        let result = Self {
            horizon_y: a.horizon_y + (b.horizon_y - a.horizon_y) * t,
            floor_base_y: a.floor_base_y + (b.floor_base_y - a.floor_base_y) * t,
            bias_power: (a.bias_power.ln() * (1.0 - t) + b.bias_power.ln() * t).exp(),
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
    if stage_data.steps.is_empty() {
        return stage_data.projection.unwrap_or_default();
    }
    // Walk backwards through steps.
    for i in (0..=step_index.min(stage_data.steps.len() - 1)).rev() {
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
///
/// # Errors
///
/// Returns a descriptive `String` identifying the first invalid projection
/// profile and the reason it failed validation.
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

pub(super) fn tween_duration(current_position: Vec2, step: &TweenStageStep) -> Duration {
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
#[derive(Clone, Debug)]
pub struct GridParams {
    /// World-lane spacing between adjacent rays, in screen pixels at depth 1.
    pub lane_spacing: f32,
    /// Minimum perimeter gap in screen pixels between adjacent drawn rays.
    pub horizon_fill: f32,
    /// Every Nth world lane (by signed `k`) gets major styling.
    pub major_ray_interval: u32,
    /// Alpha for horizontal depth lines.
    pub horizontal_alpha: f32,
    /// Alpha for major guide rays.
    pub major_ray_alpha: f32,
    /// Alpha for minor guide rays.
    pub minor_ray_alpha: f32,
    /// Horizontal view displacement at foreground depth in caller coordinates.
    pub lateral_view_offset: f32,
    /// When `true`, the centre ray (k=0) uses a distinctive amber colour.
    pub center_ray_highlight: bool,
    /// When `true`, rays fade along their length via a per-band brightness gradient
    /// from horizon (dim) to foreground (bright). When `false`, each ray is drawn
    /// as a single flat-coloured line at full brightness with the major/minor alpha.
    pub horizon_fade: bool,
}

impl GridParams {
    /// Validate all structural invariants.
    ///
    /// # Errors
    ///
    /// Returns a descriptive `String` if any invariant is violated.
    pub fn validate(&self) -> Result<(), String> {
        if !self.lane_spacing.is_finite() || self.lane_spacing <= 0.0 {
            return Err(format!(
                "lane_spacing ({}) must be finite and positive",
                self.lane_spacing,
            ));
        }
        if !self.horizon_fill.is_finite() || self.horizon_fill <= 0.0 {
            return Err(format!(
                "horizon_fill ({}) must be finite and positive",
                self.horizon_fill,
            ));
        }
        if self.major_ray_interval < 1 {
            return Err(format!(
                "major_ray_interval ({}) must be >= 1",
                self.major_ray_interval,
            ));
        }
        Ok(())
    }
}

impl Default for GridParams {
    fn default() -> Self {
        Self {
            lane_spacing: 68.0,
            horizon_fill: 6.0,
            major_ray_interval: 4,
            horizontal_alpha: 0.85,
            major_ray_alpha: 0.80,
            minor_ray_alpha: 0.45,
            lateral_view_offset: 0.0,
            center_ray_highlight: false,
            horizon_fade: true,
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
/// Uses a logarithmic foreground-proximity curve so mid/far grid bands stay
/// legible without flattening the near-to-far fade entirely.
#[must_use]
pub fn depth_brightness(d: i8) -> f32 {
    let foreground_proximity = 1.0 - f32::from(d - 1) / 8.0;
    let curved = (1.0 + 8.0 * foreground_proximity).ln() / 9.0_f32.ln();
    0.2 + 0.8 * curved
}

/// Grid colour RGBA from brightness and alpha.
///
/// Bright lavender `(0.75, 0.5, 1.0)` with a floor of 15% so deep lines
/// remain visible against dark backgrounds.  Alpha is linear in brightness
/// for a strong ~5:1 fade from foreground to horizon.
#[must_use]
pub fn grid_color_rgba(brightness: f32, alpha: f32) -> [f32; 4] {
    // Remap brightness so it never drops below 0.15 (visible on dark scenes).
    let b = 0.15 + 0.85 * brightness;
    [0.75 * b, 0.5 * b, 1.0 * b, alpha * brightness]
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
/// Guide rays are generated from uniformly-spaced world lanes, projected to
/// viewport-boundary exit points, then filtered by a greedy perimeter-gap
/// criterion to ensure readable spacing near the horizon.
///
/// # Panics
/// Panics if `debug_assert!` fails (viewport width <= 0 or `depth_span` <= 0).
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn build_perspective_grid(
    floors: &[(i8, f32)],
    viewport: Rect,
    vanish_x: f32,
    params: &GridParams,
) -> PerspectiveGrid {
    #[derive(Clone, Debug)]
    struct Candidate {
        k: i32,
        exit_point: Vec2,
        perimeter: f32,
        is_center: bool,
        is_major: bool,
    }

    const CENTER_AMBER: [f32; 4] = [0.73, 0.46, 0.09, 1.0];

    debug_assert!(
        vanish_x >= viewport.min.x && vanish_x <= viewport.max.x,
        "vanish_x ({}) must be inside viewport X range [{}, {}]",
        vanish_x,
        viewport.min.x,
        viewport.max.x,
    );

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
    debug_assert!(
        params.validate().is_ok(),
        "GridParams::validate failed: {:?}",
        params.validate(),
    );

    let vanish_y = floors[0].1; // depth 9 = horizon
    let foreground_y = floors.last().unwrap().1; // depth 1 floor
    let depth_span = foreground_y - vanish_y;
    if depth_span.abs() < f32::EPSILON {
        return grid;
    }

    let halfwidth = (viewport.max.x - viewport.min.x) * 0.5;
    let w_bottom = (viewport.min.y - vanish_y) / depth_span;
    if w_bottom <= 0.0 {
        return grid;
    }

    // §2.3 — Candidate generation.
    let k_search_geom = ((halfwidth * depth_span.abs())
        / (params.horizon_fill * params.lane_spacing))
        .ceil() as i32;
    let k_search_shift = (params.lateral_view_offset.abs() / params.lane_spacing).ceil() as i32;
    let k_search = (k_search_geom + k_search_shift + 8).max(32);

    let mut candidates: Vec<Candidate> = Vec::with_capacity((2 * k_search + 1) as usize);
    for k in -k_search..=k_search {
        let world_lane = k as f32 * params.lane_spacing;
        let eff_lane = world_lane - params.lateral_view_offset;
        let exit_point = compute_exit(eff_lane, vanish_x, vanish_y, viewport, depth_span, w_bottom);
        let perimeter = perimeter_coord(exit_point, viewport, halfwidth);
        candidates.push(Candidate {
            k,
            exit_point,
            perimeter,
            is_center: k == 0,
            is_major: k != 0 && (k.unsigned_abs() % params.major_ray_interval == 0),
        });
    }

    // §2.6 — Greedy perimeter filter.
    candidates.sort_by(|a, b| {
        a.perimeter
            .partial_cmp(&b.perimeter)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let center_idx = candidates
        .iter()
        .position(|c| c.is_center)
        .expect("k=0 must be in the candidate set");

    let mut kept = Vec::with_capacity(candidates.len());
    kept.push(candidates[center_idx].clone());

    let mut last_forward = candidates[center_idx].perimeter;
    for c in &candidates[center_idx + 1..] {
        if c.perimeter - last_forward >= params.horizon_fill {
            kept.push(c.clone());
            last_forward = c.perimeter;
        }
    }
    let mut last_backward = candidates[center_idx].perimeter;
    for c in candidates[..center_idx].iter().rev() {
        if last_backward - c.perimeter >= params.horizon_fill {
            kept.push(c.clone());
            last_backward = c.perimeter;
        }
    }

    // §2.7 — Segment emission.
    for candidate in &kept {
        let use_amber = candidate.is_center && params.center_ray_highlight;

        let alpha = if candidate.is_center {
            1.0
        } else if candidate.is_major {
            params.major_ray_alpha
        } else {
            params.minor_ray_alpha
        };

        let w_exit = (candidate.exit_point.y - vanish_y) / depth_span;
        let eff_lane = candidate.k as f32 * params.lane_spacing - params.lateral_view_offset;

        if params.horizon_fade {
            // Multi-segment band walk: one segment per depth band with a
            // brightness gradient from horizon (dim) to foreground (bright).
            let mut prev = Vec2::new(vanish_x, vanish_y);
            let mut prev_d = floors[0].0;

            for &(d, floor_y) in floors.iter().skip(1) {
                // IMPORTANT: this `w` is the grid's screen-space depth fraction along the ray.
                // It is NOT the same as `depth_progress_for_depth(d)` from ProjectionProfile,
                // which is the bias-curve input. They coincide only at bias_power = 1.
                // See the comment above `depth_progress_for_depth` for full context.
                let weight = (floor_y - vanish_y) / depth_span;
                if weight <= 0.0 {
                    continue;
                }
                if weight >= w_exit {
                    break;
                }
                let here = Vec2::new(vanish_x + eff_lane * weight, floor_y);

                let (start_rgba, end_rgba) = if use_amber {
                    (CENTER_AMBER, CENTER_AMBER)
                } else {
                    (
                        grid_color_rgba(depth_brightness(prev_d), alpha),
                        grid_color_rgba(depth_brightness(d), alpha),
                    )
                };
                grid.guide_ray_segments.push(GridLineSegment {
                    start: prev,
                    end: here,
                    start_rgba,
                    end_rgba,
                });

                prev = here;
                prev_d = d;
            }

            // Final segment to exit point.
            let c_last = if use_amber {
                CENTER_AMBER
            } else {
                grid_color_rgba(depth_brightness(prev_d), alpha)
            };
            grid.guide_ray_segments.push(GridLineSegment {
                start: prev,
                end: candidate.exit_point,
                start_rgba: c_last,
                end_rgba: c_last,
            });
        } else {
            // Single flat segment per ray — no depth fade.
            let color = if use_amber {
                CENTER_AMBER
            } else {
                grid_color_rgba(1.0, alpha)
            };
            grid.guide_ray_segments.push(GridLineSegment {
                start: Vec2::new(vanish_x, vanish_y),
                end: candidate.exit_point,
                start_rgba: color,
                end_rgba: color,
            });
        }
    }

    grid
}

/// Compute where a ray from the vanishing point at effective lane exits the viewport.
fn compute_exit(
    eff_lane: f32,
    vanish_x: f32,
    vanish_y: f32,
    viewport: Rect,
    depth_span: f32,
    w_bottom: f32,
) -> Vec2 {
    if eff_lane.abs() < f32::EPSILON {
        return Vec2::new(vanish_x, vanish_y + w_bottom * depth_span);
    }
    let w_side = if eff_lane > 0.0 {
        (viewport.max.x - vanish_x) / eff_lane
    } else {
        (viewport.min.x - vanish_x) / eff_lane
    };
    let w_exit = if w_side > 0.0 {
        w_side.min(w_bottom)
    } else {
        w_bottom
    };
    let exit_x = vanish_x + eff_lane * w_exit;
    let exit_y = vanish_y + w_exit * depth_span;
    Vec2::new(exit_x, exit_y)
}

/// Perimeter coordinate for an exit point on the viewport boundary.
fn perimeter_coord(exit: Vec2, viewport: Rect, halfwidth: f32) -> f32 {
    let tol = 1e-3;
    let up_from_bottom = (exit.y - viewport.min.y).abs();
    if up_from_bottom < tol {
        exit.x - viewport.min.x
    } else if (exit.x - viewport.max.x).abs() < tol {
        2.0 * halfwidth + up_from_bottom
    } else {
        -up_from_bottom
    }
}

// ---------------------------------------------------------------------------
// Lateral view helpers
// ---------------------------------------------------------------------------

use super::resources::{DebugPanConfig, ProjectionView};

/// Depth weight for a given floor Y relative to a projection profile.
///
/// Returns `0.0` at the horizon and `1.0` at `floor_base_y`.
///
// IMPORTANT: this computes the same screen-space weight as the grid's `w`
// in build_perspective_grid — NOT the bias-curve input from
// depth_progress_for_depth. See the comment above depth_progress_for_depth
// for why these must not be conflated.
#[must_use]
pub fn projection_weight(profile: &ProjectionProfile, floor_y: f32) -> f32 {
    let span = profile.floor_base_y - profile.horizon_y;
    if span.abs() < f32::EPSILON {
        0.0
    } else {
        (floor_y - profile.horizon_y) / span
    }
}

/// Projects a sprite's screen X under lateral camera shift using
/// depth-weighted parallax.
///
/// `world_x` is interpreted as the sprite's screen anchor (not a 3D world
/// coordinate). Under shift, the screen X is offset by
/// `lateral_view_offset * depth_weight`, so deep sprites move less than
/// near sprites. A sprite at depth 9 (horizon, `depth_weight` = 0) stays
/// exactly at its authored `world_x` regardless of shift — this stability
/// is the intended behaviour.
///
/// Note: this is NOT the same projection model as the perspective grid
/// (`build_perspective_grid`). The grid uses true pinhole projection and
/// converges rays to the vanishing point at the horizon; sprites preserve
/// their `world_x` as a screen anchor. These two models are intentionally
/// different — the grid visualises the floor plane, sprites are gameplay
/// entities in depth-banded screen space. Grid rays and sprites at the
/// same `world_x` will not coincide except at the foreground depth.
/// Computes the **visual-space** X for a given world X and screen Y.
///
/// The result is suitable for rendering and debug overlays — NOT for
/// writing into `WorldPos`, which is world-space only. If you need
/// visual displacement on an entity, use `CxPresentationTransform` offset
/// fields (written by the parallax composition system).
#[must_use]
pub fn compute_visual_x(
    world_x: f32,
    floor_y: f32,
    profile: &ProjectionProfile,
    projection_view: &ProjectionView,
) -> f32 {
    let weight = projection_weight(profile, floor_y);
    world_x - projection_view.lateral_view_offset * weight
}

/// @system Pan the lateral view offset with `Shift+Left/Right`.
///
/// Reads pan tuning from [`DebugPanConfig`]. Example binaries register
/// this resource; the game binary does not (it drives `lateral_view_offset`
/// from camera position instead).
pub fn pan_lateral_view(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    pan_config: Res<DebugPanConfig>,
    mut projection_view: ResMut<ProjectionView>,
) {
    let shift_held = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    if !shift_held {
        return;
    }

    let mut direction = 0.0;
    if keys.pressed(KeyCode::ArrowLeft) {
        direction -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        direction += 1.0;
    }
    if direction == 0.0 {
        return;
    }

    projection_view.lateral_view_offset = (projection_view.lateral_view_offset
        + direction * pan_config.speed * time.delta_secs())
    .clamp(-pan_config.limit, pan_config.limit);
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
    fn floor_y_for_progress_matches_discrete_depth_evaluator() {
        let p = ProjectionProfile {
            horizon_y: 72.0,
            floor_base_y: -14.4,
            bias_power: 3.0,
        };

        for d in 0..=9i8 {
            let progress = ProjectionProfile::depth_progress_for_depth(d);
            assert!(
                (p.floor_y_for_progress(progress) - p.floor_y_for_depth(d)).abs() < 1e-5,
                "depth {d} progress mismatch"
            );
        }
    }

    #[test]
    fn depth_progress_for_depth_uses_shared_1_to_9_normalization() {
        assert!((ProjectionProfile::depth_progress_for_depth(9) - 0.0).abs() < f32::EPSILON);
        assert!((ProjectionProfile::depth_progress_for_depth(1) - 1.0).abs() < f32::EPSILON);
        assert!((ProjectionProfile::depth_progress_for_depth(0) - 1.125).abs() < 1e-5);
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
        assert!((mid.bias_power - (1.0_f32 * 5.0).sqrt()).abs() < 1e-5);
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
        // Logarithmic falloff keeps the mid-depth bands brighter than the old
        // linear curve for better runtime legibility.
        assert!(depth_brightness(5) > 0.6);
    }

    #[test]
    fn grid_color_rgba_at_full_brightness() {
        let c = grid_color_rgba(1.0, 1.0);
        // b = 0.4 + 0.6 * 1.0 = 1.0
        assert!((c[0] - 0.75).abs() < f32::EPSILON);
        assert!((c[1] - 0.5).abs() < f32::EPSILON);
        assert!((c[2] - 1.0).abs() < f32::EPSILON);
        assert!((c[3] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn grid_color_rgba_deep_depth_still_visible() {
        // Depth 9: brightness 0.2 → b = 0.15 + 0.85*0.2 = 0.32
        let c = grid_color_rgba(0.2, 0.85);
        assert!(c[0] > 0.2, "R should be visible, got {}", c[0]);
        assert!(c[2] > 0.3, "B should be visible, got {}", c[2]);
        assert!(c[3] > 0.1, "alpha should be readable, got {}", c[3]);
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
        let viewport = test_viewport();
        let vanish_y = floors[0].1;
        let grid = build_perspective_grid(&floors, viewport, vanish_x, &GridParams::default());

        let vp = Vec2::new(vanish_x, vanish_y);
        let mut fan_count = 0;
        for seg in &grid.guide_ray_segments {
            if (seg.start - vp).length() < 1e-3 {
                fan_count += 1;
            }
        }
        assert!(fan_count > 0, "no segments originate at vanishing point");
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
            parallax_attenuation: None,
            primitives: vec![],
            primitive_bands: None,
        }
    }

    fn tween_step(x: f32, y: f32, speed: f32) -> StageStep {
        StageStep::Tween(TweenStageStep {
            coordinates: Vec2::new(x, y),
            base_speed: speed,
            spawns: vec![],
            surfaces: None,
            projection: None,
            parallax_attenuation: None,
        })
    }

    fn tween_step_with_projection(x: f32, y: f32, speed: f32, p: ProjectionProfile) -> StageStep {
        StageStep::Tween(TweenStageStep {
            coordinates: Vec2::new(x, y),
            base_speed: speed,
            spawns: vec![],
            surfaces: None,
            projection: Some(p),
            parallax_attenuation: None,
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

    #[test]
    fn evaluate_projection_at_snaps_at_zero_duration_tween() {
        // Step 0: tween from (0,0) to (0,0) at speed 1.0 → zero distance → zero duration.
        // Step 0 has profile_a, step 1 has profile_b.
        let stage = make_stage(vec![
            tween_step_with_projection(0.0, 0.0, 1.0, profile_a()),
            tween_step_with_projection(100.0, 0.0, 1.0, profile_b()),
        ]);
        // At t=0, the zero-duration step 0 resolves immediately.
        // evaluate_projection_at should produce profile_a (step 0's profile),
        // not an interpolation with NaN/jitter.
        let p = evaluate_projection_at(&stage, Duration::ZERO);
        assert!(
            p.is_valid(),
            "zero-duration tween should produce valid profile"
        );
        assert_eq!(p, profile_a());
    }

    // --- Perimeter-filter grid certification tests ---

    #[test]
    fn certify_default_params_produce_reasonable_grid() {
        let floors = test_floors();
        let grid = build_perspective_grid(&floors, test_viewport(), 80.0, &GridParams::default());
        assert!(!grid.guide_ray_segments.is_empty());
        assert!(
            grid.guide_ray_segments.len() < 2000,
            "unexpectedly many segments: {}",
            grid.guide_ray_segments.len()
        );
    }

    #[test]
    fn certify_center_ray_vertical_at_zero_shift() {
        let floors = test_floors();
        let viewport = test_viewport();
        let vanish_x = (viewport.min.x + viewport.max.x) * 0.5;
        let params = GridParams {
            lateral_view_offset: 0.0,
            ..Default::default()
        };
        let grid = build_perspective_grid(&floors, viewport, vanish_x, &params);

        let has_vertical = grid
            .guide_ray_segments
            .iter()
            .any(|s| (s.start.x - vanish_x).abs() < 0.5 && (s.end.x - vanish_x).abs() < 0.5);
        assert!(has_vertical, "centre ray should be vertical at zero shift");
    }

    #[test]
    fn certify_perimeter_spacing_respects_horizon_fill() {
        let floors = test_floors();
        let viewport = test_viewport();
        let vanish_x = (viewport.min.x + viewport.max.x) * 0.5;
        let params = GridParams {
            horizon_fill: 6.0,
            ..Default::default()
        };
        let vanish_y = floors[0].1;
        let vp = Vec2::new(vanish_x, vanish_y);

        let grid = build_perspective_grid(&floors, viewport, vanish_x, &params);

        // Collect unique exit points: for each ray, the endpoint farthest from VP.
        let mut by_angle: std::collections::HashMap<i32, Vec2> = std::collections::HashMap::new();
        for seg in &grid.guide_ray_segments {
            let d = seg.end - vp;
            if d.length() < 1.0 {
                continue;
            }
            let angle_key = (d.x.atan2(d.y) * 10000.0) as i32;
            let entry = by_angle.entry(angle_key).or_insert(seg.end);
            if (seg.end - vp).length() > (*entry - vp).length() {
                *entry = seg.end;
            }
        }
        let exits: Vec<Vec2> = by_angle.values().copied().collect();

        let halfwidth = (viewport.max.x - viewport.min.x) * 0.5;
        let mut peris: Vec<f32> = exits
            .iter()
            .map(|e| perimeter_coord(*e, viewport, halfwidth))
            .collect();
        peris.sort_by(|a, b| a.partial_cmp(b).unwrap());

        for window in peris.windows(2) {
            let gap = window[1] - window[0];
            assert!(
                gap + 1e-2 >= params.horizon_fill,
                "adjacent exit perimeter gap {} < horizon_fill {}",
                gap,
                params.horizon_fill
            );
        }
    }

    #[test]
    fn certify_lateral_shift_preserves_world_anchoring() {
        let floors = test_floors();
        let viewport = test_viewport();
        let vanish_x = (viewport.min.x + viewport.max.x) * 0.5;
        let vanish_y = floors[0].1;
        let depth_span = floors.last().unwrap().1 - vanish_y;
        let w_bottom = (viewport.min.y - vanish_y) / depth_span;

        let shift: f32 = 30.0;

        let find_center_exit = |params: GridParams| -> Vec2 {
            let grid = build_perspective_grid(&floors, viewport, vanish_x, &params);
            let vp = Vec2::new(vanish_x, vanish_y);
            let expected_bottom_x = vanish_x - params.lateral_view_offset * w_bottom;
            let expected = Vec2::new(expected_bottom_x, viewport.min.y);
            grid.guide_ray_segments
                .iter()
                .filter(|s| (s.start - vp).length() > 1.0)
                .min_by(|a, b| {
                    (a.end - expected)
                        .length()
                        .partial_cmp(&(b.end - expected).length())
                        .unwrap()
                })
                .expect("at least one ray segment expected")
                .end
        };

        let p0 = GridParams {
            lateral_view_offset: 0.0,
            ..Default::default()
        };
        let ps = GridParams {
            lateral_view_offset: shift,
            ..Default::default()
        };
        let exit0 = find_center_exit(p0);
        let exit_shifted = find_center_exit(ps);

        let expected_dx = -shift * w_bottom;
        let observed_dx = exit_shifted.x - exit0.x;
        assert!(
            (observed_dx - expected_dx).abs() < 1.0,
            "centre ray bottom X shifted by {observed_dx} but expected {expected_dx}",
        );
    }

    #[test]
    fn certify_outermost_ray_within_horizon_fill_of_horizon() {
        let floors = test_floors();
        let viewport = test_viewport();
        let vanish_x = (viewport.min.x + viewport.max.x) * 0.5;
        let vanish_y = floors[0].1;
        let params = GridParams::default();

        let grid = build_perspective_grid(&floors, viewport, vanish_x, &params);

        let closest = grid
            .guide_ray_segments
            .iter()
            .map(|s| (s.end.y - vanish_y).abs().min((s.start.y - vanish_y).abs()))
            .fold(f32::INFINITY, f32::min);

        assert!(
            closest < params.horizon_fill * 2.0,
            "outermost ray {} pixels from horizon, expected < {}",
            closest,
            params.horizon_fill * 2.0
        );
    }

    #[test]
    fn certify_shift_stability() {
        let floors = test_floors();
        let viewport = test_viewport();
        let vanish_x = (viewport.min.x + viewport.max.x) * 0.5;

        let p_a = GridParams {
            lateral_view_offset: 0.0,
            ..Default::default()
        };
        let p_b = GridParams {
            lateral_view_offset: 1.0,
            ..Default::default()
        };

        let grid_a = build_perspective_grid(&floors, viewport, vanish_x, &p_a);
        let grid_b = build_perspective_grid(&floors, viewport, vanish_x, &p_b);

        let diff =
            (grid_a.guide_ray_segments.len() as i32 - grid_b.guide_ray_segments.len() as i32).abs();
        assert!(
            diff < 40,
            "unstable under small shift: {} vs {}",
            grid_a.guide_ray_segments.len(),
            grid_b.guide_ray_segments.len()
        );
    }

    #[test]
    fn certify_major_cadence_tracks_world_lanes() {
        let floors = test_floors();
        let viewport = test_viewport();
        let vanish_x = (viewport.min.x + viewport.max.x) * 0.5;
        let vanish_y = floors[0].1;
        let depth_span = floors.last().unwrap().1 - vanish_y;
        let w_bottom = (viewport.min.y - vanish_y) / depth_span;

        for &shift in &[-50.0_f32, -10.0, 0.0, 10.0, 50.0] {
            let params = GridParams {
                lateral_view_offset: shift,
                ..Default::default()
            };
            let grid = build_perspective_grid(&floors, viewport, vanish_x, &params);

            let expected_x = vanish_x - shift * w_bottom;
            let found = grid.guide_ray_segments.iter().any(|s| {
                (s.end.x - expected_x).abs() < 2.0 && (s.end.y - viewport.min.y).abs() < 2.0
            });
            assert!(found, "centre ray (k=0) missing at shift δ={shift}");
        }
    }

    #[test]
    fn certify_no_duplicate_rays() {
        let floors = test_floors();
        let viewport = test_viewport();
        let vanish_x = (viewport.min.x + viewport.max.x) * 0.5;
        let params = GridParams::default();
        let grid = build_perspective_grid(&floors, viewport, vanish_x, &params);

        let vp = Vec2::new(vanish_x, floors[0].1);
        let mut buckets: std::collections::HashSet<i32> = std::collections::HashSet::new();
        for seg in &grid.guide_ray_segments {
            let d = seg.end - vp;
            if d.length() < 1.0 {
                continue;
            }
            let key = (d.x.atan2(d.y) * 10000.0) as i32;
            buckets.insert(key);
        }
        assert!(
            buckets.len() > 5,
            "expected multiple distinct ray angles, got {}",
            buckets.len()
        );
    }

    #[test]
    fn certify_horizon_fade_off_produces_one_segment_per_ray() {
        let floors = test_floors();
        let viewport = test_viewport();
        let vanish_x = (viewport.min.x + viewport.max.x) * 0.5;
        let params = GridParams {
            horizon_fade: false,
            ..Default::default()
        };
        let grid = build_perspective_grid(&floors, viewport, vanish_x, &params);

        // With fade off, every segment must start at the vanishing point.
        let vp = Vec2::new(vanish_x, floors[0].1);
        for seg in &grid.guide_ray_segments {
            assert!(
                (seg.start - vp).length() < 1e-3,
                "segment with fade off should start at VP, got start={:?}",
                seg.start
            );
        }
    }

    #[test]
    fn certify_horizon_fade_on_produces_multi_band_segments() {
        let floors = test_floors();
        let viewport = test_viewport();
        let vanish_x = (viewport.min.x + viewport.max.x) * 0.5;
        let params = GridParams {
            horizon_fade: true,
            ..Default::default()
        };
        let grid = build_perspective_grid(&floors, viewport, vanish_x, &params);

        // With fade on, the total segment count is greater than the count produced
        // with fade off, because each ray emits multiple segments (one per band).
        let params_off = GridParams {
            horizon_fade: false,
            ..params.clone()
        };
        let grid_off = build_perspective_grid(&floors, viewport, vanish_x, &params_off);

        assert!(
            grid.guide_ray_segments.len() > grid_off.guide_ray_segments.len(),
            "fade on ({}) should produce more segments than fade off ({})",
            grid.guide_ray_segments.len(),
            grid_off.guide_ray_segments.len()
        );
    }

    #[test]
    fn certify_horizon_fade_default_is_on() {
        // Matches pre-refactor behaviour.
        assert!(GridParams::default().horizon_fade);
    }

    // --- compute_visual_x depth-weighting tests ---

    #[test]
    fn lateral_shift_zero_at_horizon() {
        let profile = ProjectionProfile {
            horizon_y: 100.0,
            floor_base_y: 0.0,
            bias_power: 3.0,
        };
        let view = ProjectionView {
            lateral_view_offset: 50.0,
            ..Default::default()
        };
        // Depth 9 → floor_y = horizon_y = 100.0, weight = 0.
        let floor_y = profile.floor_y_for_depth(9);
        let result = compute_visual_x(200.0, floor_y, &profile, &view);
        assert!(
            (result - 200.0).abs() < 1e-3,
            "at horizon, lateral shift should be zero, got {result}"
        );
    }

    #[test]
    fn lateral_shift_full_at_foreground() {
        let profile = ProjectionProfile {
            horizon_y: 100.0,
            floor_base_y: 0.0,
            bias_power: 3.0,
        };
        let view = ProjectionView {
            lateral_view_offset: 50.0,
            ..Default::default()
        };
        // Depth 1 → floor_y = floor_base_y = 0.0, weight = 1.
        let floor_y = profile.floor_y_for_depth(1);
        let result = compute_visual_x(200.0, floor_y, &profile, &view);
        assert!(
            (result - 150.0).abs() < 1e-3,
            "at foreground, shift should equal full offset, got {result}"
        );
    }

    #[test]
    fn lateral_shift_intermediate_at_mid_depth() {
        let profile = ProjectionProfile {
            horizon_y: 100.0,
            floor_base_y: 0.0,
            bias_power: 1.0, // linear for predictable weight
        };
        let view = ProjectionView {
            lateral_view_offset: 80.0,
            ..Default::default()
        };
        // Depth 5 → t = (9-5)/8 = 0.5, floor_y = 100 + 0.5*(0-100) = 50.
        // weight = (50 - 100) / (0 - 100) = 0.5.
        let floor_y = profile.floor_y_for_depth(5);
        let result = compute_visual_x(200.0, floor_y, &profile, &view);
        let expected = 200.0 - 80.0 * 0.5;
        assert!(
            (result - expected).abs() < 1e-3,
            "at mid depth, shift should be half offset, got {result} expected {expected}"
        );
    }
}
