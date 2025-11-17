use crate::{
    globals::{is_inside_area, HUD_HEIGHT, SCREEN_RESOLUTION},
    stage::{components::placement::InView, player::components::CameraShake},
    systems::camera::CameraPos,
};
use bevy::prelude::*;
use cween::linear::components::{TargetingValueX, TargetingValueY};
use seldom_pixel::prelude::PxSubPosition;

const IN_VIEW_OFFSET: u32 = 5;
const IN_VIEW_OFFSET_BOTTOM: u32 = HUD_HEIGHT + IN_VIEW_OFFSET;

pub fn check_in_view(
    mut commands: Commands,
    mut query: Query<(Entity, &PxSubPosition), Without<InView>>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
) {
    if let Ok(camera_pos) = camera_query.single() {
        for (entity, position) in query.iter_mut() {
            if is_inside_area(
                position.0,
                camera_pos.0 + Vec2::new(IN_VIEW_OFFSET as f32, IN_VIEW_OFFSET_BOTTOM as f32),
                camera_pos.0
                    + Vec2::new(
                        SCREEN_RESOLUTION.x as f32 - IN_VIEW_OFFSET as f32,
                        SCREEN_RESOLUTION.y as f32 - IN_VIEW_OFFSET as f32,
                    ),
            ) {
                commands.entity(entity).insert(InView);
            }
        }
    }
}

pub fn check_outside_view(
    mut commands: Commands,
    mut query: Query<(Entity, &PxSubPosition), With<InView>>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
) {
    if let Ok(camera_pos) = camera_query.single() {
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

pub fn update_camera_pos_x(
    mut query: Query<
        (&TargetingValueX, &mut PxSubPosition),
        (With<CameraPos>, Without<CameraShake>),
    >,
) {
    if let Ok((pos, mut camera_pos)) = query.single_mut() {
        camera_pos.0.x = pos.0;
    }
}

pub fn update_camera_pos_y(
    mut query: Query<
        (&TargetingValueY, &mut PxSubPosition),
        (With<CameraPos>, Without<CameraShake>),
    >,
) {
    if let Ok((pos, mut camera_pos)) = query.single_mut() {
        camera_pos.0.y = pos.0;
    }
}
