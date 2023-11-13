use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxCanvas, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    components::DespawnMark,
    game::score::components::Score,
    plugins::movement::linear::components::{
        LinearAcceleration, LinearDirection, LinearSpeed, LinearTargetPosition,
        LinearTargetReached, TargetingPositionX, TargetingPositionY,
    },
    stage::{
        components::interactive::{Dead, Health},
        pickup::components::{
            HealthRecovery, PickupFeedback, PICKUP_FEEDBACK_INITIAL_SPEED_Y, PICKUP_FEEDBACK_TIME,
        },
        player::components::{Player, PLAYER_MAX_HEALTH},
        resources::StageTime,
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
            health.0 = health.0.saturating_add(recovery.0).min(PLAYER_MAX_HEALTH);
            score.add(recovery.score_deduction());

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
                TargetingPositionX(current.x),
                TargetingPositionY(current.y),
                LinearTargetPosition::<StageTime, TargetingPositionX>::new(target.x),
                LinearTargetPosition::<StageTime, TargetingPositionY>::new(target.y),
                LinearDirection::<StageTime, TargetingPositionX>::from_delta(direction_delta.x),
                LinearDirection::<StageTime, TargetingPositionY>::from_delta(direction_delta.y),
                LinearSpeed::<StageTime, TargetingPositionX>::new(speed_x),
                LinearSpeed::<StageTime, TargetingPositionY>::new(speed_y),
                LinearAcceleration::<StageTime, TargetingPositionY>::new(acceleration_y),
            );

            let sprite = assets_sprite.load(assert_assets_path!("sprites/pickups/health_4.png"));

            commands
                .spawn((
                    Name::new("Pickup Healthpack Feedback"),
                    PickupFeedback,
                    PxSubPosition::from(current),
                    PxSpriteBundle::<Layer> {
                        sprite,
                        // TODO the position should be stuck to the floor beneah the dropper
                        anchor: PxAnchor::Center,
                        canvas: PxCanvas::Camera,
                        layer: Layer::Pickups,
                        ..Default::default()
                    },
                ))
                .insert(movement_bundle);
        }
    }
}

pub fn mark_despawn_pickup_feedback(
    mut commands: Commands,
    query: Query<
        (Entity, &PickupFeedback),
        Added<LinearTargetReached<StageTime, TargetingPositionY>>,
    >,
) {
    for (entity, _) in query.iter() {
        commands.entity(entity).insert(DespawnMark);
    }
}
