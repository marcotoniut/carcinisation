//! Lightweight depth-floor debug overlay, toggled with `Ctrl+L` (`Cmd+L` on macOS).
//!
//! When enabled, draws:
//! - **Purple horizontal + diagonal lines**: perspective depth grid (floor
//!   positions 1..=9 plus converging guide rays).
//! - **Green horizontal line**: ground contact — the rendered sprite bottom for
//!   each composed entity.  Should align with the floor when grounded.
//! - **Light-blue crosshair**: entity pivot / composition origin — the point
//!   that game logic uses for position, collision, and tween targets.
//!
//! # Terminology
//!
//! - **Pivot / Origin**: `PxSubPosition` — the entity's world position and the
//!   composition's authored reference point.
//! - **Ground Contact**: the bottom of the rendered composite, accounting for
//!   presentation scale.  Equals pivot for `BottomOrigin` entities; below pivot
//!   for `Origin` entities.
//! - **Placement Anchor**: the point aligned with the world.  Grounded states
//!   align ground contact with the floor; airborne states use the pivot.
//!
//! Grid geometry (horizontal lines + guide rays) is computed by the shared
//! [`build_perspective_grid`] function in [`super::projection`] and rendered
//! here as Bevy gizmos with a `VIEWPORT_MULTIPLIER` coordinate transform.
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
use crate::stage::projection::{GridParams, build_perspective_grid};

const SCREEN_X: f32 = SCREEN_RESOLUTION.x as f32;
const SCREEN_Y: f32 = SCREEN_RESOLUTION.y as f32;

// --- Marker colours ---

/// Ground contact: rendered sprite bottom.
const GROUND_CONTACT_COLOR: Color = Color::srgba(0.15, 0.7, 0.15, 0.7);

/// Pivot / composition origin crosshair.
const PIVOT_COLOR: Color = Color::srgba(0.4, 0.7, 1.0, 0.6);

/// Crosshair arm length as a fraction of the scaled composite width/height.
const CROSSHAIR_FRACTION: f32 = 0.15;

/// Minimum crosshair arm length in viewport pixels so it stays visible at
/// horizon depths where the sprite is very small.
const CROSSHAIR_MIN_PX: f32 = 3.0 * VIEWPORT_MULTIPLIER;

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
                draw_entity_anchors,
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

/// Draw the background perspective grid using the shared geometry builder.
///
/// Collects Floor entities, converts to viewport coordinates, then delegates
/// to [`build_perspective_grid`] for all geometry.  The returned segments are
/// rendered as Bevy gizmo lines.
fn draw_depth_grid_background(
    overlay: Res<DepthDebugOverlay>,
    mut gizmos: Gizmos,
    query: Query<(&Depth, &Floor)>,
) {
    if !overlay.enabled {
        return;
    }

    // Collect floors and convert to viewport space.
    let mut floors: Vec<(i8, f32)> = Vec::new();
    for (depth, floor) in query.iter() {
        let d = depth.to_i8();
        if (1..=9).contains(&d) {
            floors.push((d, to_viewport_y(floor.0)));
        }
    }
    floors.sort_by_key(|&(d, _)| std::cmp::Reverse(d));

    // Viewport bounds in gizmo space.
    let viewport = Rect::new(
        to_viewport_x(0.0),
        to_viewport_y(0.0),
        to_viewport_x(SCREEN_X),
        to_viewport_y(SCREEN_Y),
    );
    let vanish_x = to_viewport_x(SCREEN_X * 0.5);

    let grid = build_perspective_grid(&floors, viewport, vanish_x, &GridParams::default());

    // Render horizontal depth lines.
    for seg in &grid.depth_lines {
        gizmos.line_2d(seg.start, seg.end, seg_color(&seg.start_rgba));
    }

    // Render guide ray segments with gradient.
    for seg in &grid.guide_ray_segments {
        gizmos.line_gradient_2d(
            seg.start,
            seg.end,
            seg_color(&seg.start_rgba),
            seg_color(&seg.end_rgba),
        );
    }
}

/// Draw per-entity debug markers:
///
/// - **Green horizontal line** — ground contact (rendered sprite bottom).
///   `position.y + composite.origin.y * scale_y`, converted to viewport space.
///   For `BottomOrigin` entities, coincides with the pivot.
///
/// - **Light-blue crosshair** — pivot / composition origin (`PxSubPosition`).
///   Two perpendicular lines centred on the entity position, sized to ~15 % of
///   the scaled composite (with a minimum so it stays visible at far depths).
fn draw_entity_anchors(
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
        let scale_y = presentation.map_or(1.0, |pt| pt.scale.y.abs());

        let half_w = composite.size.x as f32 * 0.5 * scale_x * VIEWPORT_MULTIPLIER;
        let cx = to_viewport_x(position.0.x);
        let pivot_wy = to_viewport_y(position.0.y);

        // --- Ground contact (green horizontal line) ---
        //
        // composite.origin.y is the bottom of the per-frame bounding box in
        // Y-up carapace space, relative to the composition origin (negative
        // when the sprite extends below the origin).  Multiplying by scale_y
        // gives the rendered distance from entity position to sprite bottom.
        let contact_y = position.0.y + composite.origin.y as f32 * scale_y;
        let contact_wy = to_viewport_y(contact_y);
        gizmos.line_2d(
            Vec2::new(cx - half_w, contact_wy),
            Vec2::new(cx + half_w, contact_wy),
            GROUND_CONTACT_COLOR,
        );

        // --- Pivot crosshair (light blue) ---
        //
        // Arm length = 15 % of scaled composite dimension, floored to a
        // minimum so the crosshair stays visible at far depths.
        let arm_x = (composite.size.x as f32 * scale_x * CROSSHAIR_FRACTION * VIEWPORT_MULTIPLIER)
            .max(CROSSHAIR_MIN_PX);
        let arm_y = (composite.size.y as f32 * scale_y * CROSSHAIR_FRACTION * VIEWPORT_MULTIPLIER)
            .max(CROSSHAIR_MIN_PX);

        // Horizontal arm.
        gizmos.line_2d(
            Vec2::new(cx - arm_x, pivot_wy),
            Vec2::new(cx + arm_x, pivot_wy),
            PIVOT_COLOR,
        );
        // Vertical arm.
        gizmos.line_2d(
            Vec2::new(cx, pivot_wy - arm_y),
            Vec2::new(cx, pivot_wy + arm_y),
            PIVOT_COLOR,
        );
    }
}

// --- Helpers ---

/// Convert an RGBA array from the shared grid builder into a Bevy [`Color`].
fn seg_color(rgba: &[f32; 4]) -> Color {
    Color::srgba(rgba[0], rgba[1], rgba[2], rgba[3])
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
