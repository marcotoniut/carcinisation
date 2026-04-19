use crate::{
    globals::{HUD_HEIGHT, SCREEN_RESOLUTION, is_inside_area},
    stage::{
        components::placement::InView, data::StageData, player::components::CameraShake,
        resources::StageTimeDomain, systems::CameraStepTween,
    },
    systems::camera::CameraPos,
};
use bevy::prelude::*;
use carapace::prelude::PxSubPosition;
use cween::linear::components::{TargetingValueX, TargetingValueY, extra::LinearTween2DReachCheck};

const IN_VIEW_OFFSET: u32 = 5;

/// Strips all stage-scoped state from the camera entity and despawns its
/// tween children.  Shared by `on_death` and `handle_stage_restart` to
/// prevent stale tween velocities from accumulating across restarts.
pub fn cleanup_camera_stage_state(
    commands: &mut Commands,
    camera_query: &mut Query<(Entity, Option<&CameraShake>, &mut PxSubPosition), With<CameraPos>>,
    tween_query: &Query<Entity, With<CameraStepTween>>,
) {
    if let Ok((cam, shake_o, mut pos)) = camera_query.single_mut() {
        if let Some(shake) = shake_o {
            pos.0 -= shake.current_offset;
        }
        commands
            .entity(cam)
            .remove::<CameraShake>()
            .remove::<TargetingValueX>()
            .remove::<TargetingValueY>()
            .remove::<LinearTween2DReachCheck<StageTimeDomain, TargetingValueX, TargetingValueY>>();
    }
    for entity in tween_query.iter() {
        commands.entity(entity).try_despawn();
    }
}
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

/// @system Positions the camera at `StageData::start_coordinates` whenever
/// the resource is (re-)inserted.
///
/// Uses `is_changed()` rather than `is_added()` because checkpoint restart
/// replaces the existing `StageData` resource via `insert_resource`, which
/// triggers `Changed` but not `Added`.
pub fn initialise_camera_from_stage(
    stage_data: Option<Res<StageData>>,
    mut camera_query: Query<&mut PxSubPosition, With<CameraPos>>,
) {
    let Some(stage_data) = stage_data else {
        return;
    };
    if stage_data.is_changed()
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
