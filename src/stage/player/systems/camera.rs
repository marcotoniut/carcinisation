use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

use crate::{
    stage::{
        components::DepthReached, enemy::components::EnemyAttack, player::components::CameraShake,
    },
    systems::camera::CameraPos,
};

pub fn camera_shake(time: Res<Time>, mut query: Query<(&mut CameraShake, &mut PxSubPosition)>) {
    for (mut shake, mut pos) in query.iter_mut() {
        if shake.shaking {
            if shake.timer.tick(time.delta()).just_finished() {
                let random_x = (rand::random::<f32>() - 0.5) * 2.0 * shake.intensity;
                let random_y = (rand::random::<f32>() - 0.5) * 2.0 * shake.intensity;
                pos.0 = shake.original_pos + Vec2::new(random_x, random_y);
                shake.shaking = false;
            }
        }
        // TODO This breaks the rail system
        // } else {
        //     pos.0 = shake.original_pos;
        // }
    }
}

pub fn trigger_shake(
    mut commands: Commands,
    trigger_query: Query<Entity, (With<EnemyAttack>, With<DepthReached>)>,
    camera_query: Query<(Entity, &PxSubPosition), With<CameraPos>>,
) {
    if trigger_query.iter().next().is_some() {
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
