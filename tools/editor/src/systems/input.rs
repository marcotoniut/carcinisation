// systems.rs

use crate::constants::{
    CAMERA_MOVE_BOUNDARY, CAMERA_MOVE_SENSITIVITY, CAMERA_ZOOM_MAX, CAMERA_ZOOM_MIN,
    CAMERA_ZOOM_SPEED,
};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

pub fn on_mouse_motion(
    mut mouse_motion_events: EventReader<MouseMotion>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    let mut camera_transform = query.single_mut();

    if buttons.pressed(MouseButton::Left) {
        for event in mouse_motion_events.read() {
            let delta = event.delta;
            camera_transform.translation.x -= delta.x * CAMERA_MOVE_SENSITIVITY;
            camera_transform.translation.y += delta.y * CAMERA_MOVE_SENSITIVITY;

            // Constrain camera movement within boundaries
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
