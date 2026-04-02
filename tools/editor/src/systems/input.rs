use crate::components::{
    Draggable, EditorCamera, SceneData, SelectedItem, SelectionOutline, StageSpawnRef,
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
        if let Some(last) = self.last_scroll_time {
            if last.elapsed() > SCROLL_GESTURE_TIMEOUT {
                self.last_scroll_time = None;
                if self.owner != GestureTarget::Tool {
                    self.owner = GestureTarget::None;
                }
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
    (With<SelectedItem>, With<Draggable>, Without<EditorCamera>),
>;
type SelectedDragQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, Option<&'static StageSpawnRef>),
    (With<Draggable>, Without<EditorCamera>),
>;

#[derive(Resource, Default)]
pub struct DragState {
    pub active: Option<DragInfo>,
}

#[derive(Copy, Clone, Debug)]
pub struct DragInfo {
    pub entity: Entity,
    pub offset: Vec2,
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
#[allow(clippy::needless_pass_by_value)]
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
            });
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

/// @system Clears drag state, gesture ownership, and coordinate overlay on mouse release.
#[allow(clippy::needless_pass_by_value)]
pub fn on_mouse_release(
    buttons: Res<ButtonInput<MouseButton>>,
    mut drag_state: ResMut<DragState>,
    mut gesture: ResMut<GestureState>,
    mut commands: Commands,
    overlay_query: Query<Entity, With<CoordinateOverlay>>,
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
        drag_state.active = None;
        for entity in overlay_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

/// @system Drag selected entities with the left mouse button, showing coordinate overlay.
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
pub fn on_mouse_drag(
    mut cursor_moved_events: MessageReader<CursorMoved>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut selected_query: SelectedDragQuery,
    mut scene_data: Option<ResMut<SceneData>>,
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

    for event in cursor_moved_events.read() {
        if event.window != window_entity {
            continue;
        }

        if let Some(world_position) = screen_to_world(camera, camera_transform, event.position) {
            let target_position = world_position - drag_info.offset;
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
                drag_state.active = None;
            }
        }
    }
}

/// @system Delete/Backspace removes the selected entity.
#[allow(clippy::needless_pass_by_value)]
pub fn on_delete_selected(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    selected_query: Query<(Entity, &StageSpawnRef), With<SelectedItem>>,
    outline_query: Query<Entity, With<SelectionOutline>>,
    mut scene_data: Option<ResMut<SceneData>>,
) {
    if !keyboard.just_pressed(KeyCode::Delete) && !keyboard.just_pressed(KeyCode::Backspace) {
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
    );

    let mut sprite = thumbnail.sprite;
    sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.5);

    commands.spawn((
        PlacementGhost,
        sprite,
        thumbnail.anchor,
        Transform::from_translation(world_position.extend(200.0)),
    ));
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
            })],
            steps: Vec::new(),
            on_start_transition_o: None,
            on_end_transition_o: None,
            gravity: None,
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
}
