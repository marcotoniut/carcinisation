use crate::{
    globals::{HUD_HEIGHT, SCREEN_RESOLUTION, is_inside_area},
    stage::{components::placement::InView, data::StageData, player::components::CameraShake},
    systems::camera::CameraPos,
};
use bevy::prelude::*;
use carapace::prelude::PxSubPosition;
use cween::linear::components::{TargetingValueX, TargetingValueY};

const IN_VIEW_OFFSET: u32 = 5;
const IN_VIEW_OFFSET_BOTTOM: u32 = HUD_HEIGHT + IN_VIEW_OFFSET;

/// @system Adds `InView` to entities that enter the visible screen area.
pub fn check_in_view(
    mut commands: Commands,
    query: Query<(Entity, &PxSubPosition), Without<InView>>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
) {
    if let Ok(camera_pos) = camera_query.single() {
        for (entity, position) in query {
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

/// @system Removes `InView` from entities that leave the visible screen area.
pub fn check_outside_view(
    mut commands: Commands,
    query: Query<(Entity, &PxSubPosition), With<InView>>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
) {
    if let Ok(camera_pos) = camera_query.single() {
        for (entity, position) in query {
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

/// @system Positions the camera at `StageData::start_coordinates` when the resource is first added.
///
/// Runs as a normal Update system so that the camera entity (spawned during Startup)
/// is guaranteed to exist. Uses change detection on StageData to fire only once.
pub fn initialise_camera_from_stage(
    stage_data: Res<StageData>,
    mut camera_query: Query<&mut PxSubPosition, With<CameraPos>>,
) {
    if stage_data.is_added()
        && let Ok(mut cam_pos) = camera_query.single_mut()
    {
        cam_pos.0 = stage_data.start_coordinates;
    }
}

/// @system Writes the tween X value into the camera sub-position.
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

/// @system Writes the tween Y value into the camera sub-position.
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
