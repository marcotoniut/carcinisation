//! Lightweight depth-floor debug overlays.
//!
//! Two independent toggles:
//!
//! - **`Ctrl+P` / `Cmd+P`** — Perspective grid: purple horizontal floor lines
//!   (depths 1..=9) plus converging guide rays toward a vanishing point.
//! - **`Ctrl+O` / `Cmd+O`** — Entity anchors: green horizontal line at the
//!   active placement anchor, light-blue crosshair at the pivot / composition
//!   origin.
//!
//! # Terminology
//!
//! - **Pivot / Origin**: `WorldPos` — the entity's world position and the
//!   composition's authored reference point.
//! - **Active Anchor**: ground contact (`entity_y − ground × scale`) when
//!   grounded; body-centre pivot (`entity_y − air × scale`) when [`Airborne`].
//! - **Placement Anchor**: the point aligned with the world.  Grounded states
//!   align ground contact with the floor; airborne states use the body pivot.
//!
//! Grid geometry (horizontal lines + guide rays) is computed from the active
//! projection profile via the shared [`build_perspective_grid`] function in
//! [`super::projection`] and rendered here as Bevy gizmos using the active
//! onscreen viewport scale.
//!
//! # Usage with `CxPlugin`
//!
//! `CxPlugin` renders a fullscreen post-process quad that overwrites Bevy
//! gizmos. Spawn a `Camera2d` with [`CxOverlayCamera`] and `order: 1` so
//! gizmos render on top. See the `depth_traverse` binary for a working example.

use bevy::{prelude::*, window::PrimaryWindow};
use carapace::prelude::{CxCompositeSprite, CxOverlayViewportTransform, CxScreen, WorldPos};
use carapace::presentation::CxPresentationTransform;

use crate::stage::components::placement::{Airborne, AnchorOffsets};
use crate::stage::projection::{GridParams, build_perspective_grid};
use crate::stage::resources::{ActiveProjection, ProjectionView};

// --- Marker colours ---

/// Active placement anchor (ground contact when grounded, body pivot when airborne).
const ANCHOR_COLOR: Color = Color::srgba(0.15, 0.7, 0.15, 0.7);

/// Pivot / composition origin crosshair.
const PIVOT_COLOR: Color = Color::srgba(0.4, 0.7, 1.0, 0.6);

/// Crosshair arm length as a fraction of the scaled composite width/height.
const CROSSHAIR_FRACTION: f32 = 0.15;
// --- Resources ---

/// Controls whether the perspective depth grid is drawn (`P` to toggle).
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

/// Controls whether per-entity anchor markers are drawn (`Cmd+O` to toggle).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct EntityAnchorOverlay {
    pub enabled: bool,
}

impl EntityAnchorOverlay {
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

// --- Plugin ---

/// Lightweight plugin that draws a depth perspective grid and entity anchors.
///
/// - `Ctrl+P` / `Cmd+P` toggles the perspective grid. Set
///   `CARCINISATION_SHOW_PERSPECTIVE=true` to start enabled.
/// - `Ctrl+O` / `Cmd+O` toggles per-entity anchor markers.
pub struct DepthDebugPlugin;

impl Plugin for DepthDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DepthDebugOverlay>()
            .init_resource::<EntityAnchorOverlay>()
            .add_systems(
                Update,
                (
                    toggle_depth_debug_overlay,
                    toggle_entity_anchor_overlay,
                    draw_depth_grid_background,
                    draw_entity_anchors,
                ),
            );
    }
}

// --- Systems ---

/// Toggle perspective grid with `Cmd+P` / `Ctrl+P`.
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
    if modifier_held && keys.just_pressed(KeyCode::KeyP) {
        overlay.enabled = !overlay.enabled;
        info!(
            "Perspective grid {}",
            if overlay.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
    }
}

/// Toggle per-entity anchor markers with `Cmd+O` / `Ctrl+O`.
fn toggle_entity_anchor_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    mut overlay: ResMut<EntityAnchorOverlay>,
) {
    let modifier_held = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    if modifier_held && keys.just_pressed(KeyCode::KeyO) {
        overlay.enabled = !overlay.enabled;
        info!(
            "Entity anchor overlay {}",
            if overlay.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
    }
}

/// Draw the background projection grid using the active projection profile.
///
/// The grid is a pure projection overlay. It is derived from
/// [`ActiveProjection`] and does not consume gameplay floor state.
fn draw_depth_grid_background(
    overlay: Res<DepthDebugOverlay>,
    projection: Option<Res<ActiveProjection>>,
    projection_view: Option<Res<ProjectionView>>,
    screen: Res<CxScreen>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut gizmos: Gizmos,
) {
    if !overlay.enabled {
        return;
    }
    let Some(projection) = projection else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let overlay_transform = CxOverlayViewportTransform::from_screen(
        &screen,
        Vec2::new(window.width(), window.height()),
    );
    let logical_screen = screen.size().as_vec2();

    // Build the grid entirely in logical game pixels. Only the final gizmo
    // draw step knows about the fitted overlay viewport transform.
    let floors: Vec<(i8, f32)> = (1..=9_i8)
        .rev()
        .map(|depth| (depth, projection.0.floor_y_for_depth(depth)))
        .collect();

    let viewport = Rect::new(0.0, 0.0, logical_screen.x, logical_screen.y);
    let vanish_x = logical_screen.x * 0.5;
    let mut grid_params = GridParams::default();
    if let Some(view) = projection_view {
        grid_params.lateral_view_offset = view.lateral_view_offset;
    }

    let grid = build_perspective_grid(&floors, viewport, vanish_x, &grid_params);

    // Render horizontal depth lines.
    for seg in &grid.depth_lines {
        gizmos.line_2d(
            overlay_transform.point(seg.start),
            overlay_transform.point(seg.end),
            seg_color(&seg.start_rgba),
        );
    }

    // Render guide ray segments with gradient.
    for seg in &grid.guide_ray_segments {
        gizmos.line_gradient_2d(
            overlay_transform.point(seg.start),
            overlay_transform.point(seg.end),
            seg_color(&seg.start_rgba),
            seg_color(&seg.end_rgba),
        );
    }
}

/// Draw per-entity debug markers:
///
/// - **Green horizontal line** — active placement anchor.  Uses
///   [`Airborne`] presence to select the right offset from
///   [`AnchorOffsets`]: ground when grounded, air when airborne.
///   Falls back to the per-frame bounding-box bottom when no anchor
///   data is present.
///
/// - **Light-blue crosshair** — pivot / composition origin (`WorldPos`).
///   Two perpendicular lines centred on the entity position, sized to ~15 % of
///   the scaled composite (with a minimum so it stays visible at far depths).
#[allow(clippy::float_cmp, clippy::similar_names)]
fn draw_entity_anchors(
    overlay: Res<EntityAnchorOverlay>,
    mut gizmos: Gizmos,
    screen: Res<CxScreen>,
    windows: Query<&Window, With<PrimaryWindow>>,
    query: Query<(
        &WorldPos,
        &CxCompositeSprite,
        Option<&CxPresentationTransform>,
        Option<&AnchorOffsets>,
        Has<Airborne>,
    )>,
) {
    if !overlay.enabled {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let overlay_transform = CxOverlayViewportTransform::from_screen(
        &screen,
        Vec2::new(window.width(), window.height()),
    );

    for (position, composite, presentation, anchor_offsets, is_airborne) in query.iter() {
        if composite.size.x == 0 {
            continue;
        }

        let scale_x = presentation.map_or(1.0, |pt| pt.scale.x.abs());
        let scale_y = presentation.map_or(1.0, |pt| pt.scale.y.abs());

        let half_w = overlay_transform.delta_x(composite.size.x as f32 * 0.5 * scale_x);
        let pivot = overlay_transform.point(position.0);

        // --- Active anchor (green horizontal line) ---
        let anchor_y = if let Some(offsets) = anchor_offsets {
            position.0.y - offsets.active_offset(is_airborne) * scale_y
        } else {
            // Fallback: per-frame bounding-box bottom.
            position.0.y + composite.origin.y as f32 * scale_y
        };
        let anchor_wy = overlay_transform.point_y(anchor_y);
        gizmos.line_2d(
            Vec2::new(pivot.x - half_w, anchor_wy),
            Vec2::new(pivot.x + half_w, anchor_wy),
            ANCHOR_COLOR,
        );

        // --- Pivot crosshair (light blue) ---
        //
        // Arm length = 15 % of scaled composite dimension, floored to a
        // minimum so the crosshair stays visible at far depths.
        // Keep the crosshair readable even when far-depth sprites get tiny.
        let min_crosshair_x = overlay_transform.delta_x(3.0);
        let min_crosshair_y = overlay_transform.delta_y(3.0);
        let arm_x = overlay_transform
            .delta_x(composite.size.x as f32 * scale_x * CROSSHAIR_FRACTION)
            .max(min_crosshair_x);
        let arm_y = overlay_transform
            .delta_y(composite.size.y as f32 * scale_y * CROSSHAIR_FRACTION)
            .max(min_crosshair_y);

        // Horizontal arm.
        gizmos.line_2d(
            Vec2::new(pivot.x - arm_x, pivot.y),
            Vec2::new(pivot.x + arm_x, pivot.y),
            PIVOT_COLOR,
        );
        // Vertical arm.
        gizmos.line_2d(
            Vec2::new(pivot.x, pivot.y - arm_y),
            Vec2::new(pivot.x, pivot.y + arm_y),
            PIVOT_COLOR,
        );
    }
}

// --- Helpers ---

/// Convert an RGBA array from the shared grid builder into a Bevy [`Color`].
fn seg_color(rgba: &[f32; 4]) -> Color {
    Color::srgba(rgba[0], rgba[1], rgba[2], rgba[3])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grid_toggle_app() -> App {
        let mut app = App::new();
        app.init_resource::<DepthDebugOverlay>();
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.add_systems(Update, toggle_depth_debug_overlay);
        app
    }

    fn make_anchor_toggle_app() -> App {
        let mut app = App::new();
        app.init_resource::<EntityAnchorOverlay>();
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.add_systems(Update, toggle_entity_anchor_overlay);
        app
    }

    #[test]
    fn cmd_p_toggles_perspective_grid() {
        let mut app = make_grid_toggle_app();

        assert!(!app.world().resource::<DepthDebugOverlay>().enabled);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::SuperLeft);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyP);
        app.update();

        assert!(app.world().resource::<DepthDebugOverlay>().enabled);
    }

    #[test]
    fn plain_p_does_not_toggle_perspective_grid() {
        let mut app = make_grid_toggle_app();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyP);
        app.update();

        assert!(!app.world().resource::<DepthDebugOverlay>().enabled);
    }

    #[test]
    fn cmd_o_toggles_entity_anchors() {
        let mut app = make_anchor_toggle_app();

        assert!(!app.world().resource::<EntityAnchorOverlay>().enabled);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::SuperLeft);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyO);
        app.update();

        assert!(app.world().resource::<EntityAnchorOverlay>().enabled);
    }

    #[test]
    fn plain_o_does_not_toggle_entity_anchors() {
        let mut app = make_anchor_toggle_app();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyO);
        app.update();

        assert!(!app.world().resource::<EntityAnchorOverlay>().enabled);
    }
}
