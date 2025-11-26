use crate::{
    stage::{
        player::{components::CameraShake, messages::CameraShakeEvent},
        resources::StageTimeDomain,
    },
    systems::camera::CameraPos,
};
use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

pub fn camera_shake(
    mut commands: Commands,
    mut query: Query<(Entity, &mut CameraShake, &mut PxSubPosition)>,
    time: Res<Time<StageTimeDomain>>,
) {
    for (entity, mut shake, mut position) in query.iter_mut() {
        if shake.shaking {
            if shake.timer.tick(time.delta()).just_finished() {
                let random_x = (rand::random::<f32>() - 0.5) * 2.0 * shake.intensity;
                let random_y = (rand::random::<f32>() - 0.5) * 2.0 * shake.intensity;
                position.0 = shake.original_position + Vec2::new(random_x, random_y);
                shake.shaking = false;
            }
        } else {
            commands.entity(entity).remove::<CameraShake>();
            position.0 = shake.original_position;
        }
    }
}

pub fn on_camera_shake(
    _trigger: On<CameraShakeEvent>,
    mut commands: Commands,
    camera_query: Query<(Entity, &PxSubPosition), With<CameraPos>>,
) {
    if let Ok((entity, position)) = camera_query.single() {
        commands.entity(entity).insert(CameraShake {
            timer: Timer::from_seconds(0.05, TimerMode::Once),
            intensity: 3.0,
            original_position: position.0,
            shaking: true,
        });
    }
}
