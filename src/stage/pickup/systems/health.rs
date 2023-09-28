use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxCanvas, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    components::DespawnMark,
    plugins::movement::linear::components::{
        LinearAcceleration, LinearDirection, LinearSpeed, LinearTargetPosition,
        LinearTargetReached, XAxisPosition, YAxisPosition,
    },
    stage::{
        components::{Dead, Health},
        pickup::components::{
            HealthRecovery, PickupFeedback, PICKUP_FEEDBACK_INITIAL_SPEED_Y, PICKUP_FEEDBACK_TIME,
        },
        player::components::{Player, PLAYER_MAX_HEALTH},
        resources::StageTime,
        score::components::Score,
    },
    systems::camera::CameraPos,
    Layer,
};

pub fn pickup_health(
    mut commands: Commands,
    mut score: ResMut<Score>,
    query: Query<(Entity, &HealthRecovery, &PxSubPosition), Added<Dead>>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    mut player_query: Query<&mut Health, With<Player>>,
    mut assets_sprite: PxAssets<PxSprite>,
) {
    let camera_pos = camera_query.get_single().unwrap();
    if let Ok(mut health) = player_query.get_single_mut() {
        for (entity, recovery, position) in query.iter() {
            commands.entity(entity).insert(DespawnMark);

            health.0 += recovery.0;
            if health.0 > PLAYER_MAX_HEALTH {
                health.0 = PLAYER_MAX_HEALTH;
            }

            score.add(recovery.score_deduction());

            // let speed_x = PICKUP_FEEDBACK_TIME;
            // let speed_y = PICKUP_FEEDBACK_ACCELERATION_Y;

            let t = PICKUP_FEEDBACK_TIME;

            let target = Vec2::new(12., 8.);
            let current = position.0 - camera_pos.0;
            let d = target - current;

            let speed_x = d.x / t;
            let speed_y = PICKUP_FEEDBACK_INITIAL_SPEED_Y;
            let adjusted_d_y = d.y - speed_y * t;
            let acceleration_y = 2. * adjusted_d_y / (t * t);
            // let acceleration_y = 0.1;

            let direction_delta = target - current;

            let movement_bundle = (
                XAxisPosition(current.x),
                YAxisPosition(current.y),
                LinearTargetPosition::<StageTime, XAxisPosition>::new(target.x),
                LinearTargetPosition::<StageTime, YAxisPosition>::new(target.y),
                LinearDirection::<StageTime, XAxisPosition>::from_delta(direction_delta.x),
                LinearDirection::<StageTime, YAxisPosition>::from_delta(direction_delta.y),
                LinearSpeed::<StageTime, XAxisPosition>::new(speed_x),
                LinearSpeed::<StageTime, YAxisPosition>::new(speed_y),
                LinearAcceleration::<StageTime, YAxisPosition>::new(acceleration_y),
            );

            let sprite = assets_sprite.load("sprites/pickups/health_2.png");

            commands
                .spawn((
                    Name::new("Pickup Healthpack Feedback"),
                    PickupFeedback,
                    PxSubPosition::from(current),
                    PxSpriteBundle::<Layer> {
                        sprite,
                        anchor: PxAnchor::Center,
                        canvas: PxCanvas::Camera,
                        layer: Layer::Pickups,
                        ..default()
                    },
                ))
                .insert(movement_bundle);
        }
    }
}

pub fn mark_despawn_pickup_feedback(
    mut commands: Commands,
    query: Query<(Entity, &PickupFeedback), Added<LinearTargetReached<StageTime, YAxisPosition>>>,
    stage_time: Res<StageTime>,
) {
    for (entity, _) in query.iter() {
        commands.entity(entity).insert(DespawnMark);
    }
}
