use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

use crate::{
    core::time::DeltaTime,
    stage::player::{components::CameraShake, events::CameraShakeEvent},
    systems::camera::CameraPos,
};

pub fn camera_shake<T: DeltaTime + Resource>(
    mut commands: Commands,
    mut query: Query<(Entity, &mut CameraShake, &mut PxSubPosition)>,
    time: Res<T>,
) {
    for (entity, mut shake, mut pos) in query.iter_mut() {
        if shake.shaking {
            if shake.timer.tick(time.delta()).just_finished() {
                let random_x = (rand::random::<f32>() - 0.5) * 2.0 * shake.intensity;
                let random_y = (rand::random::<f32>() - 0.5) * 2.0 * shake.intensity;
                pos.0 = shake.original_pos + Vec2::new(random_x, random_y);
                shake.shaking = false;
            }
        } else {
            commands.entity(entity).remove::<CameraShake>();
            pos.0 = shake.original_pos;
        }
    }
}

pub fn trigger_shake(
    mut commands: Commands,
    mut event_reader: EventReader<CameraShakeEvent>,
    camera_query: Query<(Entity, &PxSubPosition), With<CameraPos>>,
) {
    for _ in event_reader.iter() {
        for (entity, pos) in camera_query.iter() {
            commands.entity(entity).insert(CameraShake {
                timer: Timer::from_seconds(0.05, TimerMode::Once),
                intensity: 3.0,
                original_pos: pos.0,
                shaking: true,
            });
        }
    }
}
