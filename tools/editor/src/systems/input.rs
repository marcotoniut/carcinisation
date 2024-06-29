use crate::components::{Draggable, SelectedItem};
use crate::constants::{
    CAMERA_MOVE_BOUNDARY, CAMERA_MOVE_SENSITIVITY, CAMERA_ZOOM_MAX, CAMERA_ZOOM_MIN,
    CAMERA_ZOOM_SPEED,
};
use bevy::input::mouse::MouseButton;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

pub fn on_mouse_motion(
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard_buttons: Res<ButtonInput<KeyCode>>,
    mut selected_query: Query<
        &mut Transform,
        (With<SelectedItem>, With<Draggable>, Without<Camera>),
    >,
    mut camera_query: Query<&mut Transform, (With<Camera>, Without<SelectedItem>)>,
    mut window_query: Query<&Window>,
) {
    // Check if the Ctrl key and left mouse button are both pressed
    let ctrl_pressed = keyboard_buttons.pressed(KeyCode::ControlLeft)
        || keyboard_buttons.pressed(KeyCode::ControlRight);

    if selected_query.is_empty() && ctrl_pressed && mouse_buttons.pressed(MouseButton::Left) {
        for event in mouse_motion_events.read() {
            let delta = event.delta;

            let mut camera_transform = camera_query.single_mut();

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
        let window = window_query.single_mut();
        let window_size: Vec2 = Vec2::new(window.width(), window.height());

        if let Ok(camera_transform) = camera_query.get_single() {
            for event in cursor_moved_events.read() {
                let cursor_position = event.position;

                let ndc = (cursor_position / window_size) * 2.0 - Vec2::ONE;
                let world_position =
                    camera_transform.compute_matrix() * ndc.extend(-1.0).extend(1.0);
                let world_position = Vec3::new(world_position.x, world_position.y, 0.0);

                for mut transform in selected_query.iter_mut() {
                    transform.translation = world_position;
                }
            }
        }
    }
}

pub fn on_mouse_press(
    buttons: Res<ButtonInput<MouseButton>>,
    mut window_query: Query<&Window>,
    mut selected_query: Query<Entity, With<SelectedItem>>,
    mut commands: Commands,
    draggable_query: Query<
        (Entity, &Transform, &GlobalTransform, &Sprite),
        (With<Draggable>, Without<SelectedItem>),
    >,
    camera_query: Query<&Transform, With<Camera>>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        for entity in selected_query.iter_mut() {
            commands.entity(entity).remove::<SelectedItem>();
        }

        let window = window_query.single_mut();
        if let Some(cursor_position) = window.cursor_position() {
            let window_size = Vec2::new(window.width() as f32, window.height() as f32);
            let ndc = (cursor_position / window_size) * 2.0 - Vec2::ONE;

            if let Ok(camera_transform) = camera_query.get_single() {
                let camera_matrix = camera_transform.compute_matrix();
                let world_position = camera_matrix * ndc.extend(-1.0).extend(1.0);
                let world_position = Vec3::new(world_position.x, world_position.y, 0.0);

                // Sort draggable entities by their z index
                let mut sorted_entities: Vec<_> = draggable_query.iter().collect();
                sorted_entities
                    .sort_by(|a, b| b.1.translation.z.partial_cmp(&a.1.translation.z).unwrap());

                for (entity, transform, global_transform, sprite) in sorted_entities {
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

pub fn on_mouse_release(mut commands: Commands, selected_query: Query<Entity, With<SelectedItem>>) {
    for entity in selected_query.iter() {
        commands.entity(entity).remove::<SelectedItem>();
    }
}

pub fn on_mouse_wheel(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    let mut camera_transform = query.single_mut();

    if buttons.pressed(MouseButton::Left) {
        for event in mouse_wheel_events.read() {
            camera_transform.scale += Vec3::splat(event.y * CAMERA_ZOOM_SPEED);
            camera_transform.scale = camera_transform
                .scale
                .clamp(Vec3::splat(CAMERA_ZOOM_MIN), Vec3::splat(CAMERA_ZOOM_MAX));
        }
    }
}
