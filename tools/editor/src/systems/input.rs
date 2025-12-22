use crate::components::{Draggable, EditorCamera, SelectedItem};
use crate::constants::{
    CAMERA_MOVE_BOUNDARY, CAMERA_MOVE_SENSITIVITY, CAMERA_ZOOM_MAX, CAMERA_ZOOM_MIN,
};
use bevy::input::mouse::MouseButton;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::time::{Duration, Instant};

const ZOOM_SENSITIVITY: f32 = 0.003;
const WHEEL_ZOOM_SENSITIVITY: f32 = 0.0015;
const WHEEL_ANCHOR_TIMEOUT: Duration = Duration::from_millis(200);

/// @system Alt + mouse motion zooms the camera around the cursor.
pub fn on_alt_mouse_motion(
    mut cursor_moved_events: MessageReader<CursorMoved>,
    keyboard_buttons: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<EditorCamera>>,
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

        if !alt_pressed {
            continue;
        }

        if let Some(delta) = delta {
            let zoom_delta = -delta.y * ZOOM_SENSITIVITY;
            apply_zoom(&mut transform, zoom_delta);
        }
    }
}

/// @system Ctrl-drag pans the camera when nothing is selected; right-click drags selected entities.
pub fn on_ctrl_mouse_motion(
    mut cursor_moved_events: MessageReader<CursorMoved>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard_buttons: Res<ButtonInput<KeyCode>>,
    mut selected_query: Query<
        &mut Transform,
        (With<SelectedItem>, With<Draggable>, Without<EditorCamera>),
    >,
    mut camera_query: Query<(&Camera, &mut Transform), (With<EditorCamera>, Without<SelectedItem>)>,
    window_query: Query<Entity, With<PrimaryWindow>>,
    mut last_cursor: Local<Option<Vec2>>,
) {
    let ctrl_pressed = keyboard_buttons.pressed(KeyCode::ControlLeft)
        || keyboard_buttons.pressed(KeyCode::ControlRight);

    let no_selection = selected_query.is_empty();
    let Ok((camera, mut camera_transform)) = camera_query.single_mut() else {
        return;
    };
    let Ok(window_entity) = window_query.single() else {
        return;
    };

    for event in cursor_moved_events.read() {
        if event.window != window_entity {
            continue;
        }

        let delta = event
            .delta
            .or_else(|| last_cursor.map(|last| event.position - last));
        *last_cursor = Some(event.position);

        if ctrl_pressed && no_selection {
            if let Some(delta) = delta {
                camera_transform.translation.x -= delta.x * CAMERA_MOVE_SENSITIVITY;
                camera_transform.translation.y += delta.y * CAMERA_MOVE_SENSITIVITY;

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

        if mouse_buttons.pressed(MouseButton::Right) {
            let cursor_position = event.position;

            if let Some(world_position) =
                screen_to_world(camera, &camera_transform, cursor_position)
            {
                let world_position = world_position.extend(0.0);
                for mut transform in selected_query.iter_mut() {
                    transform.translation = world_position;
                }
            }
        }
    }
}

/// @system Left click selects the top-most draggable entity under the cursor.
pub fn on_mouse_press(
    buttons: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut selected_query: Query<Entity, With<SelectedItem>>,
    mut commands: Commands,
    draggable_query: Query<
        (Entity, &Transform, &GlobalTransform, &Sprite),
        (With<Draggable>, Without<SelectedItem>),
    >,
    camera_query: Query<(&Camera, &Transform), With<EditorCamera>>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        for entity in selected_query.iter_mut() {
            commands.entity(entity).remove::<SelectedItem>();
        }

        let Ok(window) = window_query.single() else {
            return;
        };
        if let Some(cursor_position) = window.cursor_position() {
            if let Ok((camera, camera_transform)) = camera_query.single() {
                if let Some(world_position) =
                    screen_to_world(camera, camera_transform, cursor_position)
                {
                    let world_position = world_position.extend(0.0);

                    // Sort draggable entities by their z index
                    let mut sorted_entities: Vec<_> = draggable_query.iter().collect();
                    sorted_entities
                        .sort_by(|a, b| b.1.translation.z.partial_cmp(&a.1.translation.z).unwrap());

                    for (entity, _transform, global_transform, sprite) in sorted_entities {
                        let position = global_transform.translation();
                        let size = sprite.custom_size.unwrap_or(Vec2::new(100.0, 100.0));

                        if world_position.x > position.x - size.x / 2.0
                            && world_position.x < position.x + size.x / 2.0
                            && world_position.y > position.y - size.y / 2.0
                            && world_position.y < position.y + size.y / 2.0
                        {
                            commands.entity(entity).insert(SelectedItem);
                            println!("POSITION: {} {}", position.to_string(), size.to_string());
                            break;
                        }
                    }
                }
            }
        }
    }
}

/// @system Clears selection on mouse release.
pub fn on_mouse_release(mut commands: Commands, selected_query: Query<Entity, With<SelectedItem>>) {
    for entity in selected_query.iter() {
        commands.entity(entity).remove::<SelectedItem>();
    }
}

/// @system Mouse wheel zooms around a stable cursor anchor for short bursts.
pub fn on_mouse_wheel(
    mut mouse_wheel_events: MessageReader<MouseWheel>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut query: Query<(&Camera, &mut Transform), With<EditorCamera>>,
    mut wheel_anchor: Local<Option<Vec2>>,
    mut wheel_anchor_timestamp: Local<Option<Instant>>,
) {
    let Ok((camera, mut camera_transform)) = query.single_mut() else {
        return;
    };

    let Ok(window) = window_query.single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    let mut wheel_events = mouse_wheel_events.read();
    let Some(first_event) = wheel_events.next() else {
        return;
    };

    let now = Instant::now();
    let reset_anchor = wheel_anchor_timestamp
        .map(|ts| now.duration_since(ts) > WHEEL_ANCHOR_TIMEOUT)
        .unwrap_or(true);
    if reset_anchor {
        *wheel_anchor = Some(cursor_position);
    }
    let anchor = *wheel_anchor.get_or_insert(cursor_position);
    wheel_anchor_timestamp.replace(now);

    let zoom_delta = first_event.y * WHEEL_ZOOM_SENSITIVITY;
    apply_zoom_at_cursor(camera, &mut camera_transform, anchor, zoom_delta);
    for event in wheel_events {
        let zoom_delta = event.y * WHEEL_ZOOM_SENSITIVITY;
        apply_zoom_at_cursor(camera, &mut camera_transform, anchor, zoom_delta);
    }
}

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
