use crate::components::{
    Draggable, EditorCamera, ProjectionGizmo, SceneData, SelectedItem, SelectionOutline,
    StageSpawnRef, StartCoordinatesNode, TweenPathNode,
};
use crate::constants::{CAMERA_MOVE_BOUNDARY, CAMERA_ZOOM_MAX, CAMERA_ZOOM_MIN};
use crate::history::SpawnLocation;
use crate::placement::PlacementMode;
use bevy::ecs::system::SystemParam;
use bevy::image::TextureAtlasLayout;
use bevy::input::gestures::PinchGesture;
use bevy::input::mouse::{MouseButton, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;
use bevy_inspector_egui::bevy_egui::input::EguiWantsInput;
use bevy_prototype_lyon::prelude::*;
use carcinisation::stage::components::placement::Depth;
use std::time::{Duration, Instant};

// ─── Tuning constants ───────────────────────────────────────────────────────

const ALT_ZOOM_SENSITIVITY: f32 = 0.003;

/// Zoom speed for scroll-with-modifier (trackpad) and mouse wheel.
const SCROLL_ZOOM_SENSITIVITY: f32 = 0.0015;
/// Mouse wheel zoom is typically coarser (discrete ticks), so scale up.
const MOUSE_WHEEL_ZOOM_SENSITIVITY: f32 = 0.08;
/// Trackpad pinch-to-zoom sensitivity.
const PINCH_ZOOM_SENSITIVITY: f32 = 0.5;

/// Trackpad pan: pixels of scroll → pixels of camera movement (at scale 1.0).
const SCROLL_PAN_SENSITIVITY: f32 = 1.0;

/// Middle-mouse drag pan sensitivity.
const MIDDLE_DRAG_PAN_SENSITIVITY: f32 = 1.0;

/// How long after the last scroll event before we consider the gesture ended.
const SCROLL_GESTURE_TIMEOUT: Duration = Duration::from_millis(120);

// ─── Gesture ownership ──────────────────────────────────────────────────────
//
// Determines who "owns" the current pointer interaction. Ownership is decided
// at gesture start (button press or first scroll event) and held until the
// gesture ends (button release or scroll timeout). This prevents ownership
// from flapping mid-gesture when the cursor crosses a UI/viewport boundary.

/// Who currently owns pointer interaction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GestureTarget {
    /// No active gesture — ownership will be decided on next input.
    #[default]
    None,
    /// The viewport/camera owns the current gesture.
    Viewport,
    /// An egui panel/menu owns the current gesture.
    Ui,
    /// A tool (entity drag, placement) owns the current gesture.
    Tool,
}

/// Tracks the current gesture owner and scroll gesture timing.
#[derive(Resource, Debug)]
pub struct GestureState {
    pub owner: GestureTarget,
    /// Timestamp of the last scroll event, for gesture-end detection.
    pub last_scroll_time: Option<Instant>,
}

impl Default for GestureState {
    fn default() -> Self {
        Self {
            owner: GestureTarget::None,
            last_scroll_time: None,
        }
    }
}

impl GestureState {
    /// Check if a scroll gesture has timed out and reset if so.
    fn tick_scroll_timeout(&mut self) {
        if let Some(last) = self.last_scroll_time
            && last.elapsed() > SCROLL_GESTURE_TIMEOUT
        {
            self.last_scroll_time = None;
            if self.owner != GestureTarget::Tool {
                self.owner = GestureTarget::None;
            }
        }
    }

    /// Claim ownership for a new scroll gesture if not already owned.
    fn claim_scroll(&mut self, egui_active: bool) -> GestureTarget {
        let now = Instant::now();
        let is_new_gesture = self.last_scroll_time.is_none() || self.owner == GestureTarget::None;
        self.last_scroll_time = Some(now);

        if is_new_gesture {
            self.owner = if egui_active {
                GestureTarget::Ui
            } else {
                GestureTarget::Viewport
            };
        }
        self.owner
    }
}

// ─── Shared queries and state ───────────────────────────────────────────────

type SelectedTransformQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Transform,
    (
        With<SelectedItem>,
        With<Draggable>,
        Without<EditorCamera>,
        Without<ProjectionGizmo>,
    ),
>;
type SelectedDragQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, Option<&'static StageSpawnRef>),
    (
        With<Draggable>,
        Without<EditorCamera>,
        Without<TweenPathNode>,
        Without<StartCoordinatesNode>,
        Without<ProjectionGizmo>,
    ),
>;

#[derive(Resource, Default)]
pub struct DragState {
    pub active: Option<DragInfo>,
}

/// What kind of entity is being dragged.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum DragKind {
    /// A spawn entity (enemy, object, pickup, destructible).
    #[default]
    Spawn,
    /// A camera-path tween node.
    PathNode,
    /// The start_coordinates node.
    StartNode,
}

impl DragKind {
    /// Whether this drag kind targets path overlay geometry (needs rebuild on release).
    pub fn is_path(self) -> bool {
        matches!(self, DragKind::PathNode | DragKind::StartNode)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DragInfo {
    pub entity: Entity,
    pub offset: Vec2,
    pub kind: DragKind,
}

/// Currently displayed coordinate overlay entity.
#[derive(Component, Debug)]
pub struct CoordinateOverlay;

#[derive(SystemParam)]
pub struct MousePressParams<'w, 's> {
    pub window_query: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    pub images: Res<'w, Assets<Image>>,
    pub texture_atlas_layouts: Res<'w, Assets<TextureAtlasLayout>>,
    pub drag_state: ResMut<'w, DragState>,
    pub selected_query: Query<'w, 's, Entity, With<SelectedItem>>,
    pub outline_query: Query<'w, 's, Entity, With<SelectionOutline>>,
    pub draggable_query: Query<
        'w,
        's,
        (
            Entity,
            &'static Sprite,
            &'static GlobalTransform,
            Option<&'static Anchor>,
        ),
        With<Draggable>,
    >,
    pub path_node_query: Query<'w, 's, (Entity, &'static GlobalTransform, &'static TweenPathNode)>,
    pub start_node_query:
        Query<'w, 's, (Entity, &'static GlobalTransform), With<StartCoordinatesNode>>,
    pub camera_query: Query<'w, 's, (&'static Camera, &'static Transform), With<EditorCamera>>,
}

// ─── Gesture timeout system ─────────────────────────────────────────────────

/// @system Resets scroll gesture ownership after a timeout gap.
pub fn tick_gesture_timeout(mut gesture: ResMut<GestureState>) {
    gesture.tick_scroll_timeout();
}

// ─── Camera systems ─────────────────────────────────────────────────────────

/// @system Alt + mouse motion zooms the camera around the cursor.
#[allow(clippy::needless_pass_by_value)]
pub fn on_alt_mouse_motion(
    mut cursor_moved_events: MessageReader<CursorMoved>,
    keyboard_buttons: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<EditorCamera>>,
    gesture: Res<GestureState>,
    mut last_cursor: Local<Option<Vec2>>,
) {
    let alt_pressed =
        keyboard_buttons.pressed(KeyCode::AltLeft) || keyboard_buttons.pressed(KeyCode::AltRight);

    let Ok(mut transform) = camera_query.single_mut() else {
        return;
    };

    for event in cursor_moved_events.read() {
        let delta = event
            .delta
            .or_else(|| last_cursor.map(|last| event.position - last));
        *last_cursor = Some(event.position);

        if !alt_pressed || gesture.owner == GestureTarget::Ui {
            continue;
        }

        if let Some(delta) = delta {
            let zoom_delta = -delta.y * ALT_ZOOM_SENSITIVITY;
            apply_zoom(&mut transform, zoom_delta);
        }
    }
}

/// @system Scroll to pan (trackpad/wheel), or Cmd/Ctrl + scroll to zoom.
/// Mouse wheel (discrete ticks) always zooms regardless of modifier.
pub fn on_scroll(
    mut mouse_wheel_events: MessageReader<MouseWheel>,
    keyboard: Res<ButtonInput<KeyCode>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut query: Query<(&Camera, &mut Transform), With<EditorCamera>>,
    egui_wants: Option<Res<EguiWantsInput>>,
    mut gesture: ResMut<GestureState>,
) {
    let egui_active = egui_wants.is_some_and(|e| e.wants_any_pointer_input());

    let Ok((camera, mut camera_transform)) = query.single_mut() else {
        return;
    };

    let Ok(window) = window_query.single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    let cmd_held = keyboard.pressed(KeyCode::SuperLeft)
        || keyboard.pressed(KeyCode::SuperRight)
        || keyboard.pressed(KeyCode::ControlLeft)
        || keyboard.pressed(KeyCode::ControlRight);

    for event in mouse_wheel_events.read() {
        // Determine gesture ownership for this scroll sequence.
        let owner = gesture.claim_scroll(egui_active);
        if owner == GestureTarget::Ui {
            continue;
        }

        let is_discrete = event.unit == MouseScrollUnit::Line;

        if is_discrete || cmd_held {
            // Mouse wheel (discrete ticks) → zoom.
            // Trackpad with Cmd/Ctrl → zoom.
            let sensitivity = if is_discrete {
                MOUSE_WHEEL_ZOOM_SENSITIVITY
            } else {
                SCROLL_ZOOM_SENSITIVITY
            };
            let zoom_delta = event.y * sensitivity;
            apply_zoom_at_cursor(camera, &mut camera_transform, cursor_position, zoom_delta);
        } else {
            // Trackpad two-finger scroll (pixel units, no modifier) → pan.
            let scale = camera_transform.scale.x;
            camera_transform.translation.x -= event.x * SCROLL_PAN_SENSITIVITY * scale;
            camera_transform.translation.y += event.y * SCROLL_PAN_SENSITIVITY * scale;

            camera_transform.translation.x = camera_transform
                .translation
                .x
                .clamp(-CAMERA_MOVE_BOUNDARY, CAMERA_MOVE_BOUNDARY);
            camera_transform.translation.y = camera_transform
                .translation
                .y
                .clamp(-CAMERA_MOVE_BOUNDARY, CAMERA_MOVE_BOUNDARY);
        }
    }
}

/// @system Trackpad pinch-to-zoom at cursor position.
#[allow(clippy::needless_pass_by_value)]
pub fn on_pinch_zoom(
    mut pinch_events: MessageReader<PinchGesture>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut query: Query<(&Camera, &mut Transform), With<EditorCamera>>,
    gesture: Res<GestureState>,
) {
    if gesture.owner == GestureTarget::Ui {
        return;
    }

    let Ok((camera, mut camera_transform)) = query.single_mut() else {
        return;
    };
    let Ok(window) = window_query.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    for event in pinch_events.read() {
        // Negate: pinch out (positive) should zoom in (decrease scale).
        let zoom_delta = -event.0 * PINCH_ZOOM_SENSITIVITY;
        apply_zoom_at_cursor(camera, &mut camera_transform, cursor_position, zoom_delta);
    }
}

/// @system Middle mouse button drag pans the camera.
#[allow(clippy::needless_pass_by_value)]
pub fn on_middle_mouse_pan(
    mut cursor_moved_events: MessageReader<CursorMoved>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut camera_query: Query<&mut Transform, With<EditorCamera>>,
    gesture: Res<GestureState>,
    mut last_cursor: Local<Option<Vec2>>,
) {
    if !mouse_buttons.pressed(MouseButton::Middle) {
        *last_cursor = None;
        return;
    }
    if gesture.owner == GestureTarget::Ui {
        return;
    }

    let Ok(mut transform) = camera_query.single_mut() else {
        return;
    };

    for event in cursor_moved_events.read() {
        let delta = event
            .delta
            .or_else(|| last_cursor.map(|last| event.position - last));
        *last_cursor = Some(event.position);

        if let Some(delta) = delta {
            let scale = transform.scale.x;
            transform.translation.x -= delta.x * MIDDLE_DRAG_PAN_SENSITIVITY * scale;
            transform.translation.y += delta.y * MIDDLE_DRAG_PAN_SENSITIVITY * scale;

            transform.translation.x = transform
                .translation
                .x
                .clamp(-CAMERA_MOVE_BOUNDARY, CAMERA_MOVE_BOUNDARY);
            transform.translation.y = transform
                .translation
                .y
                .clamp(-CAMERA_MOVE_BOUNDARY, CAMERA_MOVE_BOUNDARY);
        }
    }
}

// ─── Entity interaction systems ─────────────────────────────────────────────

/// @system Right-click drags selected entities to cursor position.
#[allow(clippy::needless_pass_by_value, clippy::type_complexity)]
pub fn on_right_click_drag(
    mut cursor_moved_events: MessageReader<CursorMoved>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut selected_query: SelectedTransformQuery,
    camera_query: Query<(&Camera, &Transform), (With<EditorCamera>, Without<SelectedItem>)>,
    window_query: Query<Entity, With<PrimaryWindow>>,
    gesture: Res<GestureState>,
) {
    if !mouse_buttons.pressed(MouseButton::Right) {
        return;
    }
    if gesture.owner == GestureTarget::Ui {
        return;
    }
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let Ok(window_entity) = window_query.single() else {
        return;
    };

    for event in cursor_moved_events.read() {
        if event.window != window_entity {
            continue;
        }
        if let Some(world_position) = screen_to_world(camera, camera_transform, event.position) {
            let world_position = world_position.extend(0.0);
            for mut transform in &mut selected_query {
                transform.translation = world_position;
            }
        }
    }
}

/// @system Left click selects the top-most draggable entity under the cursor,
/// or places a spawn if placement mode is active.
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
pub fn on_mouse_press(
    buttons: Res<ButtonInput<MouseButton>>,
    keyboard_buttons: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut params: MousePressParams,
    mut placement_mode: ResMut<PlacementMode>,
    mut scene_data: Option<ResMut<SceneData>>,
    mut gesture: ResMut<GestureState>,
    egui_wants: Option<Res<EguiWantsInput>>,
) {
    // ESC cancels placement mode
    if keyboard_buttons.just_pressed(KeyCode::Escape) {
        placement_mode.active = None;
    }

    let egui_active = egui_wants.is_some_and(|e| e.wants_any_pointer_input());

    // Claim gesture on any button press
    if buttons.just_pressed(MouseButton::Left)
        || buttons.just_pressed(MouseButton::Middle)
        || buttons.just_pressed(MouseButton::Right)
    {
        gesture.owner = if egui_active {
            GestureTarget::Ui
        } else {
            GestureTarget::Viewport
        };
    }

    if buttons.just_pressed(MouseButton::Left) && gesture.owner == GestureTarget::Viewport {
        params.drag_state.active = None;

        let Ok(window) = params.window_query.single() else {
            return;
        };
        let Some(cursor_position) = window.cursor_position() else {
            return;
        };
        let Ok((camera, camera_transform)) = params.camera_query.single() else {
            return;
        };
        let Some(world_position) = screen_to_world(camera, camera_transform, cursor_position)
        else {
            return;
        };

        // Placement mode: create new spawn. Hold Shift to keep placing.
        if placement_mode.active.is_some() {
            if let Some(scene_data) = scene_data.as_mut()
                && let SceneData::Stage(stage_data) = scene_data.as_mut()
            {
                let keep = keyboard_buttons.pressed(KeyCode::ShiftLeft)
                    || keyboard_buttons.pressed(KeyCode::ShiftRight);
                let state = if keep {
                    placement_mode.active.clone().unwrap()
                } else {
                    placement_mode.active.take().unwrap()
                };
                let spawn = state.template.instantiate(world_position, state.depth);
                stage_data.spawns.push(spawn);
            }
            gesture.owner = GestureTarget::Tool;
            return;
        }

        // Normal selection mode
        let mut candidates: Vec<_> = params.draggable_query.iter().collect();
        candidates.sort_by(|a, b| {
            b.2.translation()
                .z
                .partial_cmp(&a.2.translation().z)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut picked = None;
        for (entity, sprite, global_transform, anchor) in candidates {
            if sprite_hit_test(
                sprite,
                anchor.copied().unwrap_or_default(),
                global_transform,
                world_position,
                &params.images,
                &params.texture_atlas_layouts,
            ) {
                let position = global_transform.translation().truncate();
                picked = Some((entity, sprite, anchor, position));
                break;
            }
        }

        if let Some((entity, sprite, anchor, position)) = picked {
            for entity in params.selected_query.iter() {
                commands.entity(entity).remove::<SelectedItem>();
            }
            for entity in params.outline_query.iter() {
                commands.entity(entity).despawn();
            }
            commands.entity(entity).insert(SelectedItem);
            spawn_selection_outline(
                &mut commands,
                entity,
                sprite,
                anchor.copied().unwrap_or_default(),
                &params.images,
                &params.texture_atlas_layouts,
            );
            params.drag_state.active = Some(DragInfo {
                entity,
                offset: world_position - position,
                kind: DragKind::Spawn,
            });
            gesture.owner = GestureTarget::Tool;
        } else {
            // Check path node handles (circle distance test).
            // Use the hover-scaled radius so the clickable area matches the visual.
            let hit_radius = crate::builders::stage::PATH_NODE_RADIUS
                * crate::builders::stage::PATH_NODE_HOVER_SCALE
                * camera_transform.scale.x.max(1.0);
            let mut node_picked = false;
            for (entity, global_transform, _node) in params.path_node_query.iter() {
                let node_pos = global_transform.translation().truncate();
                if node_pos.distance(world_position) <= hit_radius {
                    for entity in params.selected_query.iter() {
                        commands.entity(entity).remove::<SelectedItem>();
                    }
                    for entity in params.outline_query.iter() {
                        commands.entity(entity).despawn();
                    }
                    commands.entity(entity).insert(SelectedItem);
                    params.drag_state.active = Some(DragInfo {
                        entity,
                        offset: world_position - node_pos,
                        kind: DragKind::PathNode,
                    });
                    gesture.owner = GestureTarget::Tool;
                    node_picked = true;
                    break;
                }
            }

            // Check start_coordinates node.
            if !node_picked {
                for (entity, global_transform) in params.start_node_query.iter() {
                    let node_pos = global_transform.translation().truncate();
                    if node_pos.distance(world_position) <= hit_radius {
                        for entity in params.selected_query.iter() {
                            commands.entity(entity).remove::<SelectedItem>();
                        }
                        for entity in params.outline_query.iter() {
                            commands.entity(entity).despawn();
                        }
                        commands.entity(entity).insert(SelectedItem);
                        params.drag_state.active = Some(DragInfo {
                            entity,
                            offset: world_position - node_pos,
                            kind: DragKind::StartNode,
                        });
                        gesture.owner = GestureTarget::Tool;
                        node_picked = true;
                        break;
                    }
                }
            }

            if !node_picked {
                // Alt+click on empty canvas: insert a Tween step at the cursor position
                // into the nearest path segment.
                let alt_pressed = keyboard_buttons.pressed(KeyCode::AltLeft)
                    || keyboard_buttons.pressed(KeyCode::AltRight);
                if alt_pressed {
                    if let Some(scene_data) = scene_data.as_mut()
                        && let SceneData::Stage(stage_data) = scene_data.as_mut()
                    {
                        let h_screen = carcinisation::globals::SCREEN_RESOLUTION.as_vec2() / 2.0;
                        let data_coords = world_position - h_screen;
                        let insert_idx = path_insert_index(stage_data, data_coords);
                        let mut tween = carcinisation::stage::components::TweenStageStep::new();
                        tween.coordinates = data_coords;
                        stage_data.steps.insert(
                            insert_idx,
                            carcinisation::stage::data::StageStep::Tween(tween),
                        );
                    }
                    gesture.owner = GestureTarget::Tool;
                } else {
                    for entity in params.selected_query.iter() {
                        commands.entity(entity).remove::<SelectedItem>();
                    }
                    for entity in params.outline_query.iter() {
                        commands.entity(entity).despawn();
                    }
                    params.drag_state.active = None;
                }
            }
        }
    }
}

/// @system Clears drag state, gesture ownership, and coordinate overlay on mouse release.
/// If a path node was being dragged, marks SceneData changed to trigger a clean rebuild.
#[allow(clippy::needless_pass_by_value)]
pub fn on_mouse_release(
    buttons: Res<ButtonInput<MouseButton>>,
    mut drag_state: ResMut<DragState>,
    mut gesture: ResMut<GestureState>,
    mut commands: Commands,
    overlay_query: Query<Entity, With<CoordinateOverlay>>,
    mut pending_rebuild: ResMut<crate::resources::PendingSceneRebuild>,
) {
    let any_released = buttons.just_released(MouseButton::Left)
        || buttons.just_released(MouseButton::Middle)
        || buttons.just_released(MouseButton::Right);

    if any_released {
        // Only release gesture if no buttons are still held
        let any_still_held = buttons.pressed(MouseButton::Left)
            || buttons.pressed(MouseButton::Middle)
            || buttons.pressed(MouseButton::Right);
        if !any_still_held {
            gesture.owner = GestureTarget::None;
        }
    }

    if buttons.just_released(MouseButton::Left) {
        // If a path node was dragged, request a full scene rebuild for the next frame.
        if let Some(ref info) = drag_state.active
            && info.kind.is_path()
        {
            pending_rebuild.0 = true;
        }
        drag_state.active = None;
        for entity in overlay_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

/// @system Drag selected entities with the left mouse button, showing coordinate overlay.
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn on_mouse_drag(
    mut cursor_moved_events: MessageReader<CursorMoved>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut selected_query: SelectedDragQuery,
    mut path_node_query: Query<
        (&mut Transform, &TweenPathNode),
        (
            Without<EditorCamera>,
            Without<StartCoordinatesNode>,
            Without<ProjectionGizmo>,
        ),
    >,
    mut start_node_query: Query<
        &mut Transform,
        (
            With<StartCoordinatesNode>,
            Without<EditorCamera>,
            Without<TweenPathNode>,
            Without<ProjectionGizmo>,
        ),
    >,
    mut gizmo_query: Query<
        (&mut Transform, &ProjectionGizmo),
        (
            Without<EditorCamera>,
            Without<TweenPathNode>,
            Without<StartCoordinatesNode>,
            Without<StageSpawnRef>,
            Without<SelectedItem>,
        ),
    >,
    mut scene_data: Option<ResMut<SceneData>>,
    controls: Option<Res<crate::resources::StageControlsUI>>,
    mut drag_state: ResMut<DragState>,
    camera_query: Query<(&Camera, &Transform), With<EditorCamera>>,
    window_query: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
    overlay_query: Query<Entity, With<CoordinateOverlay>>,
    asset_server: Option<Res<AssetServer>>,
) {
    let Some(drag_info) = drag_state.active else {
        return;
    };
    if !mouse_buttons.pressed(MouseButton::Left) {
        drag_state.active = None;
        return;
    }

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let Ok(window_entity) = window_query.single() else {
        return;
    };

    let h_screen = carcinisation::globals::SCREEN_RESOLUTION.as_vec2() / 2.0;

    for event in cursor_moved_events.read() {
        if event.window != window_entity {
            continue;
        }

        let Some(world_position) = screen_to_world(camera, camera_transform, event.position) else {
            continue;
        };
        let target_position = world_position - drag_info.offset;

        // Path node drag: update tween step coordinates.
        if let Ok((mut transform, node)) = path_node_query.get_mut(drag_info.entity) {
            transform.translation = target_position.extend(transform.translation.z);
            // Node is rendered at coordinates + h_screen, so subtract half-screen to get data coords.
            let data_coords = target_position - h_screen;
            // bypass_change_detection: keep the dragged entity alive across frames.
            // SceneData will be marked changed on mouse release to trigger a clean rebuild.
            if let Some(scene_data) = scene_data.as_mut()
                && let SceneData::Stage(stage_data) = scene_data.bypass_change_detection()
                && let Some(carcinisation::stage::data::StageStep::Tween(tween)) =
                    stage_data.steps.get_mut(node.step_index)
            {
                tween.coordinates = data_coords;
            }
            if let Some(ref asset_server) = asset_server {
                update_coordinate_overlay(
                    &mut commands,
                    &overlay_query,
                    data_coords,
                    None,
                    asset_server,
                );
            }
            continue;
        }

        // Start coordinates node drag.
        if let Ok(mut transform) = start_node_query.get_mut(drag_info.entity) {
            transform.translation = target_position.extend(transform.translation.z);
            let data_coords = target_position - h_screen;
            if let Some(scene_data) = scene_data.as_mut()
                && let SceneData::Stage(stage_data) = scene_data.bypass_change_detection()
            {
                stage_data.start_coordinates = data_coords;
            }
            if let Some(ref asset_server) = asset_server {
                update_coordinate_overlay(
                    &mut commands,
                    &overlay_query,
                    data_coords,
                    None,
                    asset_server,
                );
            }
            continue;
        }

        // Projection gizmo drag: update horizon_y or floor_base_y.
        if let Ok((mut transform, gizmo)) = gizmo_query.get_mut(drag_info.entity) {
            // Only move vertically — keep the gizmo's X position.
            transform.translation.y = target_position.y;

            // Determine which step to edit: find the active step at the current
            // scrub position and ensure it has a projection override.
            if let Some(scene_data) = scene_data.as_mut()
                && let Some(ref controls) = controls
                && let SceneData::Stage(stage_data) = scene_data.bypass_change_detection()
            {
                let info = carcinisation::stage::projection::walk_steps_at_elapsed(
                    stage_data,
                    controls.elapsed_duration,
                );
                let new_y = target_position.y;

                // Resolve effective projection BEFORE taking a mutable reference
                // to the step, to avoid borrow conflicts.
                let eff = carcinisation::stage::projection::effective_projection(
                    stage_data,
                    info.step_index,
                );

                // Get or create the projection override on the active step.
                if let Some(step) = stage_data.steps.get_mut(info.step_index) {
                    let proj = match step {
                        carcinisation::stage::data::StageStep::Tween(s) => {
                            s.projection.get_or_insert(eff)
                        }
                        carcinisation::stage::data::StageStep::Stop(s) => {
                            s.projection.get_or_insert(eff)
                        }
                        carcinisation::stage::data::StageStep::Cinematic(_) => {
                            continue;
                        }
                    };

                    // Apply clamped value.
                    const MIN_GAP: f32 = 1.0;
                    match gizmo {
                        ProjectionGizmo::Horizon => {
                            proj.horizon_y = new_y.max(proj.floor_base_y + MIN_GAP);
                        }
                        ProjectionGizmo::FloorBase => {
                            proj.floor_base_y = new_y.min(proj.horizon_y - MIN_GAP);
                        }
                    }
                }
            }
            continue;
        }

        // Spawn entity drag.
        if let Ok((mut transform, spawn_ref)) = selected_query.get_mut(drag_info.entity) {
            transform.translation = target_position.extend(transform.translation.z);
            let Some(spawn_ref) = spawn_ref else {
                continue;
            };
            if let Some(scene_data) = scene_data.as_mut()
                && let SceneData::Stage(stage_data) = scene_data.bypass_change_detection()
            {
                update_spawn_from_drag(spawn_ref, target_position, stage_data);

                // Update coordinate overlay
                if let Some(ref asset_server) = asset_server {
                    let depth = {
                        let loc = SpawnLocation::from_ref(spawn_ref);
                        crate::history::resolve_spawn(stage_data, &loc).map(|s| s.get_depth())
                    };
                    update_coordinate_overlay(
                        &mut commands,
                        &overlay_query,
                        target_position,
                        depth,
                        asset_server,
                    );
                }
            }
        } else {
            // Entity was despawned (e.g. by a concurrent scene rebuild). If it was a
            // path drag, mark SceneData changed so stale decorative geometry gets cleaned up.
            if drag_info.kind.is_path()
                && let Some(ref mut sd) = scene_data
            {
                sd.set_changed();
            }
            drag_state.active = None;
        }
    }
}

/// @system Delete/Backspace removes the selected spawn or path node.
#[allow(clippy::needless_pass_by_value, clippy::type_complexity)]
pub fn on_delete_selected(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    selected_query: Query<(Entity, &StageSpawnRef), (With<SelectedItem>, Without<TweenPathNode>)>,
    path_node_query: Query<(Entity, &TweenPathNode), With<SelectedItem>>,
    outline_query: Query<Entity, With<SelectionOutline>>,
    mut scene_data: Option<ResMut<SceneData>>,
) {
    if !keyboard.just_pressed(KeyCode::Delete) && !keyboard.just_pressed(KeyCode::Backspace) {
        return;
    }

    // Delete a selected path node (removes the whole step).
    if let Ok((entity, node)) = path_node_query.single() {
        if let Some(scene_data) = scene_data.as_mut()
            && let SceneData::Stage(stage_data) = scene_data.as_mut()
            && node.step_index < stage_data.steps.len()
        {
            stage_data.steps.remove(node.step_index);
        }
        commands.entity(entity).remove::<SelectedItem>();
        for entity in outline_query.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    let Ok((entity, spawn_ref)) = selected_query.single() else {
        return;
    };

    let Some(scene_data) = scene_data.as_mut() else {
        return;
    };
    let SceneData::Stage(stage_data) = scene_data.as_mut() else {
        return;
    };

    let location = SpawnLocation::from_ref(spawn_ref);
    if crate::history::remove_spawn(stage_data, &location).is_none() {
        return;
    }

    // Clear selection (scene will rebuild due to SceneData change)
    commands.entity(entity).remove::<SelectedItem>();
    for entity in outline_query.iter() {
        commands.entity(entity).despawn();
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Converts a screen position to world coordinates using the editor camera.
/// Finds the step index at which a new tween should be inserted so that it
/// sits on the path segment nearest to `point`. Returns `steps.len()` (append)
/// if appending after the last segment is the best fit.
fn path_insert_index(stage_data: &carcinisation::stage::data::StageData, point: Vec2) -> usize {
    use carcinisation::stage::data::StageStep;

    // Walk the path collecting (segment_start, segment_end, insert_before_index).
    // "insert_before_index" = the index of the tween that ENDS this segment + 1,
    // so that inserting there puts the new tween between the two endpoints.
    let mut segments: Vec<(Vec2, Vec2, usize)> = Vec::new();
    let mut current_pos = stage_data.start_coordinates;
    for (i, step) in stage_data.steps.iter().enumerate() {
        if let StageStep::Tween(t) = step {
            // Inserting at index i would place the new step before this tween.
            segments.push((current_pos, t.coordinates, i));
            current_pos = t.coordinates;
        }
    }

    if segments.is_empty() {
        return stage_data.steps.len();
    }

    let mut best_idx = stage_data.steps.len(); // default: append
    let mut best_dist = f32::MAX;

    for &(a, b, before_idx) in &segments {
        let dist = point_to_segment_dist_sq(point, a, b);
        if dist < best_dist {
            best_dist = dist;
            // Insert *before* the tween that ends this segment, so the new node
            // sits between the segment's start and end.
            best_idx = before_idx;
        }
    }

    // If the point is at least as close to the last endpoint as to any segment,
    // prefer appending (extending the path) over splitting an existing segment.
    let append_dist = (point - current_pos).length_squared();
    if append_dist <= best_dist {
        best_idx = stage_data.steps.len();
    }

    best_idx
}

/// Squared distance from `point` to the line segment `a`–`b`.
fn point_to_segment_dist_sq(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let ap = point - a;
    let len_sq = ab.length_squared();
    if len_sq < f32::EPSILON {
        return ap.length_squared();
    }
    let t = (ap.dot(ab) / len_sq).clamp(0.0, 1.0);
    let proj = a + ab * t;
    (point - proj).length_squared()
}

fn screen_to_world(camera: &Camera, transform: &Transform, cursor_position: Vec2) -> Option<Vec2> {
    camera
        .viewport_to_world_2d(&GlobalTransform::from(*transform), cursor_position)
        .ok()
}

/// Applies a uniform zoom delta with clamp and returns whether it changed.
fn apply_zoom(transform: &mut Transform, delta: f32) -> bool {
    if delta.abs() < f32::EPSILON {
        return false;
    }
    let current = transform.scale.x;
    let target = (current + delta).clamp(CAMERA_ZOOM_MIN, CAMERA_ZOOM_MAX);
    if (target - current).abs() < f32::EPSILON {
        return false;
    }
    transform.scale = Vec3::new(target, target, transform.scale.z);
    true
}

/// Zooms toward a cursor position by offsetting camera translation after scaling.
fn apply_zoom_at_cursor(
    camera: &Camera,
    transform: &mut Transform,
    cursor_position: Vec2,
    delta: f32,
) {
    let Some(before) = screen_to_world(camera, transform, cursor_position) else {
        return;
    };
    if !apply_zoom(transform, delta) {
        return;
    }
    let Some(after) = screen_to_world(camera, transform, cursor_position) else {
        return;
    };
    let offset = before - after;
    transform.translation.x += offset.x;
    transform.translation.y += offset.y;
}

fn sprite_hit_test(
    sprite: &Sprite,
    anchor: Anchor,
    global_transform: &GlobalTransform,
    world_position: Vec2,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
) -> bool {
    let Some(image) = images.get(&sprite.image) else {
        return false;
    };
    let local_position = global_transform
        .to_matrix()
        .inverse()
        .transform_point3(world_position.extend(0.0))
        .truncate();

    let Ok(texture_point) =
        sprite.compute_pixel_space_point(local_position, anchor, images, texture_atlas_layouts)
    else {
        return false;
    };

    let size = image.size();
    if size.x == 0 || size.y == 0 {
        return false;
    }
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let x = texture_point.x.floor().clamp(0.0, size.x as f32 - 1.0) as usize;
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let y = texture_point.y.floor().clamp(0.0, size.y as f32 - 1.0) as usize;
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let idx = (y * size.x as usize + x) * 4;
    let Some(data) = image.data.as_deref() else {
        return false;
    };
    if idx + 3 >= data.len() {
        return false;
    }
    data[idx + 3] > 0
}

fn spawn_selection_outline(
    commands: &mut Commands,
    entity: Entity,
    sprite: &Sprite,
    anchor: Anchor,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
) {
    let size = sprite_size(sprite, images, texture_atlas_layouts).unwrap_or(Vec2::ZERO);
    if size.x <= 0.0 || size.y <= 0.0 {
        return;
    }

    let sprite_center = -anchor.as_vec() * size;
    let shape = shapes::Rectangle {
        extents: size,
        origin: RectangleOrigin::Center,
        radii: None,
    };
    commands.entity(entity).with_children(|builder| {
        builder.spawn((
            SelectionOutline,
            Name::new("Selection Outline"),
            ShapeBuilder::with(&shape)
                .stroke((Color::WHITE, 1.0))
                .build(),
            Transform::from_translation(sprite_center.extend(0.1)),
            GlobalTransform::default(),
        ));
    });
}

fn sprite_size(
    sprite: &Sprite,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
) -> Option<Vec2> {
    sprite
        .custom_size
        .or_else(|| sprite_texture_rect(sprite, images, texture_atlas_layouts).map(|r| r.size()))
}

fn sprite_texture_rect(
    sprite: &Sprite,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
) -> Option<Rect> {
    let image_size = images.get(&sprite.image).map(Image::size)?;
    let atlas_rect = sprite
        .texture_atlas
        .as_ref()
        .and_then(|s| s.texture_rect(texture_atlas_layouts))
        .map(|r| r.as_rect());
    #[allow(clippy::cast_precision_loss)]
    let base_rect = Rect::new(0.0, 0.0, image_size.x as f32, image_size.y as f32);

    let rect = match (atlas_rect, sprite.rect) {
        (None, None) => base_rect,
        (None, Some(sprite_rect)) => sprite_rect,
        (Some(atlas_rect), None) => atlas_rect,
        (Some(atlas_rect), Some(mut sprite_rect)) => {
            sprite_rect.min += atlas_rect.min;
            sprite_rect.max += atlas_rect.min;
            sprite_rect
        }
    };
    Some(rect)
}

fn update_spawn_from_drag(
    spawn_ref: &StageSpawnRef,
    world_position: Vec2,
    stage_data: &mut carcinisation::stage::data::StageData,
) {
    let location = SpawnLocation::from_ref(spawn_ref);
    if let Some(spawn) = crate::history::resolve_spawn_mut(stage_data, &location) {
        let coords = match *spawn_ref {
            StageSpawnRef::Static { .. } => world_position,
            StageSpawnRef::Step { step_origin, .. } => world_position - step_origin,
        };
        spawn.set_coordinates(coords);
    }
}

fn update_coordinate_overlay(
    commands: &mut Commands,
    overlay_query: &Query<Entity, With<CoordinateOverlay>>,
    position: Vec2,
    depth: Option<carcinisation::stage::components::placement::Depth>,
    asset_server: &AssetServer,
) {
    for entity in overlay_query.iter() {
        commands.entity(entity).despawn();
    }

    let depth_str = depth.map_or("?".to_string(), |d| d.to_i8().to_string());
    let text = format!("({:.0}, {:.0}) D:{}", position.x, position.y, depth_str);

    commands.spawn((
        CoordinateOverlay,
        Text2d::new(text),
        TextFont {
            font: asset_server.load(crate::constants::FONT_PATH),
            font_size: 11.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 0.0, 0.9)),
        Transform::from_translation((position + Vec2::new(0.0, -12.0)).extend(100.0)),
        Anchor::TOP_CENTER,
    ));
}

/// @system Highlights path node handles by scaling them up when the cursor is nearby.
#[allow(clippy::needless_pass_by_value, clippy::type_complexity)]
pub fn highlight_hovered_path_nodes(
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &Transform), With<EditorCamera>>,
    mut node_query: Query<
        (&GlobalTransform, &mut Transform),
        (
            With<crate::components::PathOverlay>,
            With<Draggable>,
            Without<EditorCamera>,
        ),
    >,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let Some(world_cursor) = screen_to_world(camera, camera_transform, cursor_position) else {
        return;
    };

    let hover_radius = crate::builders::stage::PATH_NODE_RADIUS
        * crate::builders::stage::PATH_NODE_HOVER_SCALE
        * camera_transform.scale.x.max(1.0);

    for (global_transform, mut transform) in &mut node_query {
        let node_pos = global_transform.translation().truncate();
        let hovered = node_pos.distance(world_cursor) <= hover_radius;
        let target_scale = if hovered {
            crate::builders::stage::PATH_NODE_HOVER_SCALE
        } else {
            1.0
        };
        transform.scale = Vec3::splat(target_scale);
    }
}

/// @system Spawns, updates, or despawns the translucent placement ghost under the cursor.
#[allow(
    clippy::needless_pass_by_value,
    clippy::too_many_arguments,
    clippy::type_complexity
)]
pub fn update_placement_ghost(
    mut commands: Commands,
    placement_mode: Res<PlacementMode>,
    ghost_query: Query<Entity, With<crate::components::PlacementGhost>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &Transform), (With<EditorCamera>, Without<SelectedItem>)>,
    asset_server: Res<AssetServer>,
    mut image_assets: ResMut<Assets<Image>>,
    mut thumbnail_cache: ResMut<crate::resources::ThumbnailCache>,
    depth_scale_config: Res<carcinisation::stage::depth_scale::DepthScaleConfig>,
) {
    use crate::builders::thumbnail::resolve_stage_spawn_thumbnail;
    use crate::components::PlacementGhost;

    let has_placement = placement_mode.active.is_some();

    // Despawn ghosts if placement mode is off.
    if !has_placement {
        for entity in ghost_query.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    let Ok(window) = window_query.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let Some(world_position) = screen_to_world(camera, camera_transform, cursor_position) else {
        return;
    };

    let state = placement_mode.active.as_ref().unwrap();
    let temp_spawn = state.template.instantiate(world_position, state.depth);

    // Despawn stale ghost entities (we recreate every frame for simplicity).
    for entity in ghost_query.iter() {
        commands.entity(entity).despawn();
    }

    let thumbnail = resolve_stage_spawn_thumbnail(
        &temp_spawn,
        &asset_server,
        &mut image_assets,
        &mut thumbnail_cache,
        &depth_scale_config,
        state.animation_tag.as_deref(),
    );

    let mut sprite = thumbnail.sprite;
    sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.5);

    commands.spawn((
        PlacementGhost,
        sprite,
        thumbnail.anchor,
        Transform::from_translation(world_position.extend(200.0))
            .with_scale(Vec3::splat(thumbnail.fallback_scale)),
    ));
}

/// @system Keys 1-9 set depth while in placement mode.
pub fn placement_depth_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut placement_mode: ResMut<PlacementMode>,
) {
    let Some(state) = placement_mode.active.as_mut() else {
        return;
    };
    let pressed = [
        (KeyCode::Digit1, Depth::One),
        (KeyCode::Digit2, Depth::Two),
        (KeyCode::Digit3, Depth::Three),
        (KeyCode::Digit4, Depth::Four),
        (KeyCode::Digit5, Depth::Five),
        (KeyCode::Digit6, Depth::Six),
        (KeyCode::Digit7, Depth::Seven),
        (KeyCode::Digit8, Depth::Eight),
        (KeyCode::Digit9, Depth::Nine),
    ];
    for (key, depth) in pressed {
        if keys.just_pressed(key) {
            state.depth = depth;
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::PlacementMode;
    use bevy::asset::RenderAssetUsages;
    use bevy::camera::RenderTargetInfo;
    use bevy::ecs::message::Messages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    use bevy::window::WindowResolution;
    use carcinisation::stage::components::placement::Depth;
    use carcinisation::stage::data::{ObjectSpawn, ObjectType, SkyboxData, StageData, StageSpawn};

    #[derive(Bundle)]
    struct TestCameraBundle {
        camera: Camera,
        camera_2d: Camera2d,
        transform: Transform,
        global_transform: GlobalTransform,
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn drag_updates_stage_spawn_coordinates() {
        let mut app = App::new();
        app.add_message::<CursorMoved>();
        app.init_resource::<DragState>();
        app.init_resource::<GestureState>();
        app.init_resource::<PlacementMode>();
        app.init_resource::<crate::resources::PendingSceneRebuild>();
        app.insert_resource(Assets::<Image>::default());
        app.insert_resource(Assets::<TextureAtlasLayout>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());
        app.insert_resource(ButtonInput::<KeyCode>::default());

        let stage_data = StageData {
            name: "Test".to_string(),
            background_path: String::new(),
            music_path: String::new(),
            skybox: SkyboxData {
                path: String::new(),
                frames: 1,
            },
            start_coordinates: Vec2::ZERO,
            spawns: vec![StageSpawn::Object(ObjectSpawn {
                object_type: ObjectType::BenchBig,
                coordinates: Vec2::ZERO,
                depth: Depth::Zero,
                authored_depths: None,
            })],
            steps: Vec::new(),
            on_start_transition_o: None,
            on_end_transition_o: None,
            gravity: None,
            projection: None,
        };
        app.insert_resource(SceneData::Stage(stage_data));

        app.add_systems(Update, (on_mouse_press, on_mouse_drag, on_mouse_release));

        let window_entity = app
            .world_mut()
            .spawn((
                Window {
                    resolution: WindowResolution::new(200, 200),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();

        {
            let mut window_entity_mut = app.world_mut().entity_mut(window_entity);
            let mut window = window_entity_mut.get_mut::<Window>().unwrap();
            window.set_cursor_position(Some(Vec2::new(100.0, 100.0)));
        }

        let mut camera = Camera::default();
        let half = Vec2::new(100.0, 100.0);
        camera.computed.clip_from_view =
            Mat4::from_scale(Vec3::new(1.0 / half.x, 1.0 / half.y, 1.0));
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: UVec2::new(200, 200),
            scale_factor: 1.0,
        });

        app.world_mut().spawn((
            TestCameraBundle {
                camera,
                camera_2d: Camera2d,
                transform: Transform::default(),
                global_transform: GlobalTransform::default(),
            },
            EditorCamera,
        ));

        let image = Image::new_fill(
            Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255, 255, 255, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::all(),
        );
        let image_handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);

        let transform = Transform::from_xyz(0.0, 0.0, 0.0);
        let entity = app
            .world_mut()
            .spawn((
                Sprite {
                    image: image_handle,
                    ..default()
                },
                Anchor::CENTER,
                Draggable,
                StageSpawnRef::Static { index: 0 },
                transform,
                GlobalTransform::from(transform),
            ))
            .id();

        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();

        app.world_mut()
            .resource_mut::<Messages<CursorMoved>>()
            .write(CursorMoved {
                window: window_entity,
                position: Vec2::new(120.0, 100.0),
                delta: Some(Vec2::new(20.0, 0.0)),
            });
        app.update();

        let transform = app.world().entity(entity).get::<Transform>().unwrap();
        assert!((transform.translation.x - 20.0).abs() < 0.01);
        assert!((transform.translation.y - 0.0).abs() < 0.01);

        let scene_data = app.world().resource::<SceneData>();
        let SceneData::Stage(stage_data) = scene_data else {
            panic!("Expected stage data");
        };
        let StageSpawn::Object(spawn) = &stage_data.spawns[0] else {
            panic!("Expected object spawn");
        };
        assert!((spawn.coordinates.x - 20.0).abs() < 0.01);
        assert!((spawn.coordinates.y - 0.0).abs() < 0.01);
    }

    /// Helper: build a minimal test App with the systems needed for path node testing.
    fn make_path_node_test_app() -> App {
        use carcinisation::stage::components::TweenStageStep;
        use carcinisation::stage::data::StageStep;

        let mut app = App::new();
        app.add_message::<CursorMoved>();
        app.init_resource::<DragState>();
        app.init_resource::<GestureState>();
        app.init_resource::<PlacementMode>();
        app.init_resource::<crate::resources::PendingSceneRebuild>();
        app.insert_resource(Assets::<Image>::default());
        app.insert_resource(Assets::<TextureAtlasLayout>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());
        app.insert_resource(ButtonInput::<KeyCode>::default());

        let stage_data = StageData {
            name: "Test".to_string(),
            background_path: String::new(),
            music_path: String::new(),
            skybox: SkyboxData {
                path: String::new(),
                frames: 1,
            },
            start_coordinates: Vec2::ZERO,
            spawns: Vec::new(),
            steps: vec![StageStep::Tween(TweenStageStep {
                coordinates: Vec2::new(100.0, 50.0),
                base_speed: 1.0,
                spawns: Vec::new(),
                floor_depths: None,
                projection: None,
            })],
            on_start_transition_o: None,
            on_end_transition_o: None,
            gravity: None,
            projection: None,
        };
        app.insert_resource(SceneData::Stage(stage_data));

        app.add_systems(Update, (on_mouse_press, on_mouse_drag, on_mouse_release));

        app
    }

    /// Spawns the test window, camera, and a TweenPathNode handle. Returns (window_entity, node_entity).
    fn spawn_path_node_test_entities(app: &mut App) -> (Entity, Entity) {
        let window_entity = app
            .world_mut()
            .spawn((
                Window {
                    resolution: WindowResolution::new(200, 200),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();

        {
            let mut window_entity_mut = app.world_mut().entity_mut(window_entity);
            let mut window = window_entity_mut.get_mut::<Window>().unwrap();
            // Cursor at center → maps to world (0, 0) with default camera.
            window.set_cursor_position(Some(Vec2::new(100.0, 100.0)));
        }

        let mut camera = Camera::default();
        let half = Vec2::new(100.0, 100.0);
        camera.computed.clip_from_view =
            Mat4::from_scale(Vec3::new(1.0 / half.x, 1.0 / half.y, 1.0));
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: UVec2::new(200, 200),
            scale_factor: 1.0,
        });

        app.world_mut().spawn((
            TestCameraBundle {
                camera,
                camera_2d: Camera2d,
                transform: Transform::default(),
                global_transform: GlobalTransform::default(),
            },
            EditorCamera,
        ));

        // Node at world (0, 0) — within hit radius of cursor at world (0, 0).
        let transform = Transform::from_xyz(0.0, 0.0, 10.1);
        let node_entity = app
            .world_mut()
            .spawn((
                TweenPathNode { step_index: 0 },
                Draggable,
                crate::components::SceneItem,
                transform,
                GlobalTransform::from(transform),
            ))
            .id();

        (window_entity, node_entity)
    }

    #[test]
    fn drag_tween_path_node_updates_coordinates() {
        use carcinisation::stage::data::StageStep;

        let mut app = make_path_node_test_app();
        let (window_entity, node_entity) = spawn_path_node_test_entities(&mut app);

        // Press left mouse to pick the path node.
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();

        // Verify drag was started on the path node.
        let drag = app.world().resource::<DragState>();
        assert!(drag.active.is_some());
        assert_eq!(drag.active.unwrap().entity, node_entity);
        assert_eq!(drag.active.unwrap().kind, DragKind::PathNode);

        // Drag to (120, 100) → world offset of (+20, 0).
        app.world_mut()
            .resource_mut::<Messages<CursorMoved>>()
            .write(CursorMoved {
                window: window_entity,
                position: Vec2::new(120.0, 100.0),
                delta: Some(Vec2::new(20.0, 0.0)),
            });
        app.update();

        // Entity should still be alive.
        assert!(app.world().get_entity(node_entity).is_ok());

        // The tween coordinates should have been updated.
        let scene_data = app.world().resource::<SceneData>();
        let SceneData::Stage(stage_data) = scene_data else {
            panic!("Expected stage data");
        };
        let StageStep::Tween(tween) = &stage_data.steps[0] else {
            panic!("Expected tween step");
        };
        // Node was at world (0,0), dragged +20 on x. Data coords = world - h_screen.
        // With h_screen = SCREEN_RESOLUTION/2 = (80, 72), data_coords.x = 20.0 - 80.0 = -60.0
        let h_screen = carcinisation::globals::SCREEN_RESOLUTION.as_vec2() / 2.0;
        let expected_x = 20.0 - h_screen.x;
        assert!(
            (tween.coordinates.x - expected_x).abs() < 0.01,
            "expected x ~{}, got {}",
            expected_x,
            tween.coordinates.x,
        );
    }

    #[test]
    fn path_node_drag_survives_multiple_frames() {
        let mut app = make_path_node_test_app();
        let (window_entity, node_entity) = spawn_path_node_test_entities(&mut app);

        // Pick the node.
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();

        // Drag frame 1.
        app.world_mut()
            .resource_mut::<Messages<CursorMoved>>()
            .write(CursorMoved {
                window: window_entity,
                position: Vec2::new(110.0, 100.0),
                delta: Some(Vec2::new(10.0, 0.0)),
            });
        app.update();
        assert!(app.world().resource::<DragState>().active.is_some());

        // Drag frame 2 — entity must still be alive and drag must continue.
        app.world_mut()
            .resource_mut::<Messages<CursorMoved>>()
            .write(CursorMoved {
                window: window_entity,
                position: Vec2::new(130.0, 100.0),
                delta: Some(Vec2::new(20.0, 0.0)),
            });
        app.update();
        assert!(app.world().resource::<DragState>().active.is_some());
        assert!(app.world().get_entity(node_entity).is_ok());
    }

    #[test]
    fn path_node_release_clears_drag_state() {
        let mut app = make_path_node_test_app();
        let (window_entity, _node_entity) = spawn_path_node_test_entities(&mut app);

        // Pick.
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();

        // Drag.
        app.world_mut()
            .resource_mut::<Messages<CursorMoved>>()
            .write(CursorMoved {
                window: window_entity,
                position: Vec2::new(120.0, 100.0),
                delta: Some(Vec2::new(20.0, 0.0)),
            });
        app.update();
        assert!(app.world().resource::<DragState>().active.is_some());

        // Release: clear press first, then release. Bevy's ButtonInput needs both.
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        // Need an extra update to flush the release through on_mouse_release.
        // But first, clear_just_pressed/released from previous frame.
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();

        // After release, DragState should be cleared.
        assert!(app.world().resource::<DragState>().active.is_none());
    }

    #[test]
    fn delete_selected_path_node_removes_step() {
        let mut app = make_path_node_test_app();
        let (_window_entity, node_entity) = spawn_path_node_test_entities(&mut app);

        // Add on_delete_selected to the system schedule.
        app.add_systems(Update, on_delete_selected);

        // Mark the node as selected (simulating a click pick).
        app.world_mut().entity_mut(node_entity).insert(SelectedItem);

        // Press Delete.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Delete);
        app.update();

        // The tween step should have been removed.
        let scene_data = app.world().resource::<SceneData>();
        let SceneData::Stage(stage_data) = scene_data else {
            panic!("Expected stage data");
        };
        assert!(
            stage_data.steps.is_empty(),
            "expected steps to be empty after deletion, got {}",
            stage_data.steps.len()
        );
    }

    #[test]
    fn alt_click_inserts_tween_at_correct_path_position() {
        use carcinisation::stage::components::TweenStageStep;
        use carcinisation::stage::data::StageStep;

        let mut app = App::new();
        app.add_message::<CursorMoved>();
        app.init_resource::<DragState>();
        app.init_resource::<GestureState>();
        app.init_resource::<PlacementMode>();
        app.insert_resource(Assets::<Image>::default());
        app.insert_resource(Assets::<TextureAtlasLayout>::default());
        app.insert_resource(ButtonInput::<MouseButton>::default());
        app.insert_resource(ButtonInput::<KeyCode>::default());

        // Path: start(0,0) → tween[0](100,0) → tween[1](200,0)
        let stage_data = StageData {
            name: "Test".to_string(),
            background_path: String::new(),
            music_path: String::new(),
            skybox: SkyboxData {
                path: String::new(),
                frames: 1,
            },
            start_coordinates: Vec2::ZERO,
            spawns: Vec::new(),
            steps: vec![
                StageStep::Tween(TweenStageStep {
                    coordinates: Vec2::new(100.0, 0.0),
                    base_speed: 1.0,
                    spawns: Vec::new(),
                    floor_depths: None,
                    projection: None,
                }),
                StageStep::Tween(TweenStageStep {
                    coordinates: Vec2::new(200.0, 0.0),
                    base_speed: 1.0,
                    spawns: Vec::new(),
                    floor_depths: None,
                    projection: None,
                }),
            ],
            on_start_transition_o: None,
            on_end_transition_o: None,
            gravity: None,
            projection: None,
        };
        app.insert_resource(SceneData::Stage(stage_data));

        app.add_systems(Update, on_mouse_press);

        let window_entity = app
            .world_mut()
            .spawn((
                Window {
                    resolution: WindowResolution::new(200, 200),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();

        {
            let mut we = app.world_mut().entity_mut(window_entity);
            let mut window = we.get_mut::<Window>().unwrap();
            // Click at screen center → world (0,0). With h_screen offset, data_coords will be
            // (-80, -72). That's closest to the first segment start(0,0)→tween(100,0).
            window.set_cursor_position(Some(Vec2::new(100.0, 100.0)));
        }

        let mut camera = Camera::default();
        let half = Vec2::new(100.0, 100.0);
        camera.computed.clip_from_view =
            Mat4::from_scale(Vec3::new(1.0 / half.x, 1.0 / half.y, 1.0));
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: UVec2::new(200, 200),
            scale_factor: 1.0,
        });

        app.world_mut().spawn((
            TestCameraBundle {
                camera,
                camera_2d: Camera2d,
                transform: Transform::default(),
                global_transform: GlobalTransform::default(),
            },
            EditorCamera,
        ));

        // Press Alt + Left click.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::AltLeft);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();

        let scene_data = app.world().resource::<SceneData>();
        let SceneData::Stage(stage_data) = scene_data else {
            panic!("Expected stage data");
        };

        // Should now have 3 steps (was 2).
        assert_eq!(stage_data.steps.len(), 3);

        // The new tween should have been inserted at index 0 (before the first tween),
        // since the click point (-80, -72) is closest to the start→first_tween segment.
        assert!(
            matches!(stage_data.steps[0], StageStep::Tween(_)),
            "expected inserted tween at index 0"
        );

        // Original tweens should now be at indices 1 and 2.
        if let StageStep::Tween(t) = &stage_data.steps[1] {
            assert!(
                (t.coordinates.x - 100.0).abs() < 0.01,
                "expected original tween at index 1, got x={}",
                t.coordinates.x
            );
        } else {
            panic!("expected tween at index 1");
        }
        if let StageStep::Tween(t) = &stage_data.steps[2] {
            assert!(
                (t.coordinates.x - 200.0).abs() < 0.01,
                "expected original tween at index 2, got x={}",
                t.coordinates.x
            );
        } else {
            panic!("expected tween at index 2");
        }
    }

    #[test]
    fn path_insert_index_finds_nearest_segment() {
        use carcinisation::stage::components::TweenStageStep;
        use carcinisation::stage::data::StageStep;

        let stage_data = StageData {
            name: "Test".to_string(),
            background_path: String::new(),
            music_path: String::new(),
            skybox: SkyboxData {
                path: String::new(),
                frames: 1,
            },
            start_coordinates: Vec2::ZERO,
            spawns: Vec::new(),
            steps: vec![
                StageStep::Tween(TweenStageStep {
                    coordinates: Vec2::new(100.0, 0.0),
                    base_speed: 1.0,
                    spawns: Vec::new(),
                    floor_depths: None,
                    projection: None,
                }),
                StageStep::Tween(TweenStageStep {
                    coordinates: Vec2::new(200.0, 0.0),
                    base_speed: 1.0,
                    spawns: Vec::new(),
                    floor_depths: None,
                    projection: None,
                }),
            ],
            on_start_transition_o: None,
            on_end_transition_o: None,
            gravity: None,
            projection: None,
        };

        // Point near the first segment (0,0)→(100,0): should insert at index 0.
        assert_eq!(path_insert_index(&stage_data, Vec2::new(50.0, 5.0)), 0);

        // Point near the second segment (100,0)→(200,0): should insert at index 1.
        assert_eq!(path_insert_index(&stage_data, Vec2::new(150.0, 5.0)), 1);

        // Point far beyond the last tween: should append.
        assert_eq!(
            path_insert_index(&stage_data, Vec2::new(500.0, 0.0)),
            stage_data.steps.len()
        );
    }

    #[test]
    fn depth_hotkey_sets_depth_in_placement_mode() {
        let mut app = App::new();
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(PlacementMode {
            active: Some(crate::placement::PlacementState {
                template: crate::placement::SpawnTemplate::Enemy(
                    carcinisation::stage::enemy::entity::EnemyType::Mosquiton,
                ),
                depth: Depth::Three,
                animation_tag: None,
            }),
        });
        app.add_systems(Update, placement_depth_hotkeys);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit7);
        app.update();

        let pm = app.world().resource::<PlacementMode>();
        assert_eq!(pm.active.as_ref().unwrap().depth, Depth::Seven);
    }

    #[test]
    fn depth_hotkey_noop_without_placement() {
        let mut app = App::new();
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(PlacementMode { active: None });
        app.add_systems(Update, placement_depth_hotkeys);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit5);
        app.update();

        let pm = app.world().resource::<PlacementMode>();
        assert!(pm.active.is_none());
    }
}
