//! Lightweight depth-floor debug overlay, toggled with `Ctrl+L` (`Cmd+L` on macOS).
//!
//! When enabled, draws:
//! - **Purple horizontal lines** for visible depth floor positions (1..=9),
//!   with brightness increasing for shallower depths.
//! - **Purple diagonal lines** converging toward a vanishing point at the
//!   horizon, with the same brightness progression.
//! - **Green lines** at each composed entity's ground contact point.
//!
//! Grid lines fade toward the horizon using the same depth-brightness
//! progression (depth 1 = brightest, depth 9 = dimmest).
//!
//! # Usage with `PxPlugin`
//!
//! `PxPlugin` renders a fullscreen post-process quad that overwrites Bevy
//! gizmos. Spawn a `Camera2d` with [`PxOverlayCamera`] and `order: 1` so
//! gizmos render on top. See the `depth_traverse` binary for a working example.

use bevy::prelude::*;
use carapace::prelude::{PxCompositeSprite, PxSubPosition};
use carapace::presentation::PxPresentationTransform;

use crate::globals::{SCREEN_RESOLUTION, VIEWPORT_MULTIPLIER};
use crate::stage::components::placement::{Depth, Floor};

const SCREEN_X: f32 = SCREEN_RESOLUTION.x as f32;
const SCREEN_Y: f32 = SCREEN_RESOLUTION.y as f32;
const LINE_EXTENSION: f32 = 1000.;

const ANCHOR_LINE_COLOR: Color = Color::srgba(0.15, 0.6, 0.15, 0.7);

// --- Guide ray parameters ---

/// Total number of guide rays. Must be odd so the centre ray is exact.
const GUIDE_RAY_COUNT: u32 = 35;

/// Major-ray interval: every Nth ray is drawn at full intensity.
const GUIDE_RAY_MAJOR_EVERY: u32 = 4;

/// Alpha for horizontal depth lines.
const HORIZONTAL_ALPHA: f32 = 0.85;

/// Alpha for major guide rays.
const GUIDE_RAY_MAJOR_ALPHA: f32 = 0.80;

/// Alpha for minor guide rays.
const GUIDE_RAY_MINOR_ALPHA: f32 = 0.45;

// --- Resources ---

/// Resource controlling whether the depth debug overlay is drawn.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct DepthDebugOverlay {
    pub enabled: bool,
}

impl DepthDebugOverlay {
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

// --- Plugin ---

/// Lightweight plugin that draws a depth perspective grid and contact lines.
///
/// Toggle with `Ctrl+L` (`Cmd+L` on macOS). Default state: off.
///
/// Insert [`DepthDebugOverlay::new(true)`] as a resource before adding the
/// plugin to start with the overlay enabled (useful for examples).
pub struct DepthDebugPlugin;

impl Plugin for DepthDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DepthDebugOverlay>().add_systems(
            Update,
            (
                toggle_depth_debug_overlay,
                draw_depth_grid_background,
                draw_ground_anchors_foreground,
            )
                .chain(),
        );
    }
}

// --- Systems ---

/// Toggle overlay with Ctrl+L (Cmd+L on macOS).
fn toggle_depth_debug_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    mut overlay: ResMut<DepthDebugOverlay>,
) {
    let modifier_held = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    if modifier_held && keys.just_pressed(KeyCode::KeyL) {
        overlay.enabled = !overlay.enabled;
        info!(
            "Depth debug overlay {}",
            if overlay.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
    }
}

/// Draw the background perspective grid: horizontal depth lines and diagonal
/// guide lines converging toward a vanishing point at the horizon centre.
///
/// Both horizontal and diagonal lines use the same depth-based brightness
/// progression: depth 1 = brightest (1.0), depth 9 = dimmest (0.5).
/// Alpha is kept low so the sprite visually dominates the grid.
fn draw_depth_grid_background(
    overlay: Res<DepthDebugOverlay>,
    mut gizmos: Gizmos,
    query: Query<(&Depth, &Floor)>,
) {
    if !overlay.enabled {
        return;
    }

    let mut floors: Vec<(i8, f32)> = Vec::new();

    for (depth, floor) in query.iter() {
        let d = depth.to_i8();
        if !(1..=9).contains(&d) {
            continue;
        }

        let wy = to_viewport_y(floor.0);
        let color = grid_color(depth_brightness(d), HORIZONTAL_ALPHA);

        // Horizontal depth line.
        gizmos.line_2d(
            Vec2::new(-LINE_EXTENSION, wy),
            Vec2::new(LINE_EXTENSION, wy),
            color,
        );

        floors.push((d, floor.0));
    }

    // Single-point angular perspective guide rays with mu-law
    // edge-concentration.
    //
    // Rays span the open angular interval (−π/2, +π/2) relative to
    // the downward vertical from the VP. A mu-law remap concentrates
    // rays toward the edges (near-horizontal) to compensate for the
    // sec²(θ) screen-space divergence of tan-projected angles:
    //
    //   u  = 2i/(N−1) − 1                    linear ∈ [−1, 1]
    //   b  = sign(u) × ln(1 + μ|u|) / ln(1 + μ)   mu-law
    //   θ  = b × π/2 × N/(N+1)               open interval
    //
    // The mu-law derivative b'(u) = μ / ((1+μ|u|)·ln(1+μ)) is large
    // at the centre (wide angular steps) and small at the edges
    // (tight angular steps). The edge-to-centre density ratio is
    // exactly 1 + μ.
    //
    // With μ = 35, edges are 36× denser than centre — enough to
    // substantially compensate for sec²(θ) across the visible range.
    //
    // Each ray is clipped to the visible viewport (bottom edge and
    // nearer side edge).
    floors.sort_by_key(|&(d, _)| std::cmp::Reverse(d)); // depth 9 first

    if floors.len() < 2 {
        return;
    }

    let vanish_x = to_viewport_x(SCREEN_X * 0.5);
    let vanish_y = to_viewport_y(floors[0].1);

    // Viewport bounds for ray clipping.
    let screen_left = to_viewport_x(0.0);
    let screen_right = to_viewport_x(SCREEN_X);
    let screen_bottom = to_viewport_y(0.0);

    let n = GUIDE_RAY_COUNT;
    let centre_idx = n / 2;

    // Open-interval max angle: outermost rays approach but never reach ±π/2.
    let theta_max = std::f32::consts::FRAC_PI_2 * n as f32 / (n as f32 + 1.0);
    let mu = 35.0_f32;
    let ln_1_plus_mu = (1.0 + mu).ln();

    for i in 0..n {
        // Mu-law remap: uniform u → edge-concentrated b.
        let u = 2.0 * i as f32 / (n - 1) as f32 - 1.0;
        let b = u.signum() * (1.0 + mu * u.abs()).ln() / ln_1_plus_mu;
        let theta = b * theta_max;

        // θ = 0 → straight down; positive → right; negative → left.
        let dx = theta.sin();
        let dy = -theta.cos();

        let is_major = i == centre_idx || i % GUIDE_RAY_MAJOR_EVERY == 0;
        let alpha = if is_major {
            GUIDE_RAY_MAJOR_ALPHA
        } else {
            GUIDE_RAY_MINOR_ALPHA
        };

        // Clip: intersect with the screen bottom and the nearer side edge.
        let t_bottom = if dy.abs() > f32::EPSILON {
            (screen_bottom - vanish_y) / dy
        } else {
            f32::MAX
        };
        let t_side = if dx.abs() > f32::EPSILON {
            let edge = if dx > 0.0 { screen_right } else { screen_left };
            (edge - vanish_x) / dx
        } else {
            f32::MAX
        };
        let t_end = t_bottom.min(t_side);
        if t_end <= 0.0 {
            continue;
        }

        let endpoint = Vec2::new(vanish_x + t_end * dx, vanish_y + t_end * dy);

        // Draw gradient segments between consecutive depth floors.
        let mut prev = Vec2::new(vanish_x, vanish_y);
        let mut prev_d = floors[0].0;

        for &(d, floor_carapace) in floors.iter().skip(1) {
            if dy.abs() < f32::EPSILON {
                break;
            }
            let wy = to_viewport_y(floor_carapace);
            let t = (wy - vanish_y) / dy;
            if t <= 0.0 || t > t_end {
                continue;
            }
            let here = Vec2::new(vanish_x + t * dx, wy);

            let c_prev = grid_color(depth_brightness(prev_d), alpha);
            let c_here = grid_color(depth_brightness(d), alpha);
            gizmos.line_gradient_2d(prev, here, c_prev, c_here);

            prev = here;
            prev_d = d;
        }

        // Final segment to the screen boundary.
        let c_last = grid_color(depth_brightness(prev_d), alpha);
        gizmos.line_gradient_2d(prev, endpoint, c_last, c_last);
    }
}

/// Draw a green horizontal line at each composed entity's ground contact,
/// spanning its scaled width.
fn draw_ground_anchors_foreground(
    overlay: Res<DepthDebugOverlay>,
    mut gizmos: Gizmos,
    query: Query<(
        &PxSubPosition,
        &PxCompositeSprite,
        Option<&PxPresentationTransform>,
    )>,
) {
    if !overlay.enabled {
        return;
    }

    for (position, composite, presentation) in query.iter() {
        if composite.size.x == 0 {
            continue;
        }

        let scale_x = presentation.map_or(1.0, |pt| pt.scale.x.abs());
        let half_w = composite.size.x as f32 * 0.5 * scale_x * VIEWPORT_MULTIPLIER;
        let cx = to_viewport_x(position.0.x);
        let wy = to_viewport_y(position.0.y);

        gizmos.line_2d(
            Vec2::new(cx - half_w, wy),
            Vec2::new(cx + half_w, wy),
            ANCHOR_LINE_COLOR,
        );
    }
}

// --- Helpers ---

/// Depth brightness: 1.0 at depth 1 (brightest), 0.2 at depth 9 (dimmest).
fn depth_brightness(d: i8) -> f32 {
    1.0 - f32::from(d - 1) / 8.0 * 0.8
}

/// Purple grid colour at the given brightness and alpha.
/// RGB scales with brightness; alpha scales with brightness² so the
/// fade toward the horizon is more pronounced and perceptually linear.
fn grid_color(brightness: f32, alpha: f32) -> Color {
    Color::srgba(
        0.6 * brightness,
        0.15 * brightness,
        0.9 * brightness,
        alpha * brightness * brightness,
    )
}

/// Convert a carapace pixel X coordinate to Bevy world X for gizmo drawing.
///
/// `PxPlugin` renders a fullscreen quad — the pixel-art centre always maps to
/// viewport origin (0, 0). No additional offset is needed.
fn to_viewport_x(x: f32) -> f32 {
    VIEWPORT_MULTIPLIER * (x - SCREEN_X * 0.5)
}

/// Convert a carapace pixel Y coordinate to Bevy world Y for gizmo drawing.
fn to_viewport_y(y: f32) -> f32 {
    VIEWPORT_MULTIPLIER * (y - SCREEN_Y * 0.5)
}
