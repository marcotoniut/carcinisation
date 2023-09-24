use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

use crate::{
    globals::{is_inside_area, SCREEN_RESOLUTION},
    stage::components::InView,
    systems::camera::CameraPos,
};

pub fn check_in_view(
    mut commands: Commands,
    mut query: Query<(Entity, &PxSubPosition), Without<InView>>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
) {
    if let Ok(camera_pos) = camera_query.get_single() {
        for (entity, position) in query.iter_mut() {
            if is_inside_area(
                position.0,
                camera_pos.0,
                camera_pos.0 + Vec2::new(SCREEN_RESOLUTION.x as f32, SCREEN_RESOLUTION.y as f32),
            ) {
                commands.entity(entity).insert(InView {});
            }
        }
    }
}

pub fn check_outside_view(
    mut commands: Commands,
    mut query: Query<(Entity, &PxSubPosition), With<InView>>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
) {
    if let Ok(camera_pos) = camera_query.get_single() {
        for (entity, position) in query.iter_mut() {
            if !is_inside_area(
                position.0,
                camera_pos.0,
                camera_pos.0 + Vec2::new(SCREEN_RESOLUTION.x as f32, SCREEN_RESOLUTION.y as f32),
            ) {
                commands.entity(entity).remove::<InView>();
            }
        }
    }
}
