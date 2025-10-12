use crate::pixel::{PxAssets, PxSpriteBundle};
use crate::{
    components::DespawnMark,
    game::score::components::Score,
    layer::Layer,
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
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use seldom_pixel::prelude::{PxAnchor, PxCanvas, PxSprite, PxSubPosition};

// TODO could be generalised
#[derive(Bundle)]
pub struct PickupFeedbackMovementBundle {
    pub targeting_position_x: TargetingPositionX,
    pub targeting_position_y: TargetingPositionY,
    pub linear_speed_x: LinearSpeed<StageTime, TargetingPositionX>,
    pub linear_speed_y: LinearSpeed<StageTime, TargetingPositionY>,
    pub linear_acceleration_y: LinearAcceleration<StageTime, TargetingPositionY>,
    pub linear_direction_x: LinearDirection<StageTime, TargetingPositionX>,
    pub linear_direction_y: LinearDirection<StageTime, TargetingPositionY>,
    pub linear_target_position_x: LinearTargetPosition<StageTime, TargetingPositionX>,
    pub linear_target_position_y: LinearTargetPosition<StageTime, TargetingPositionY>,
}

impl PickupFeedbackMovementBundle {
    pub fn new(current: Vec2) -> Self {
        let t = PICKUP_FEEDBACK_TIME;

        let target = Vec2::new(12., 8.);
        let d = target - current;

        let speed_x = d.x / t;
        let speed_y = PICKUP_FEEDBACK_INITIAL_SPEED_Y;
        let adjusted_d_y = d.y - speed_y * t;
        let acceleration_y = 2. * adjusted_d_y / (t * t);
        // let acceleration_y = 0.1;

        let direction_delta = target - current;

        Self {
            targeting_position_x: current.x.into(),
            targeting_position_y: current.y.into(),
            linear_speed_x: LinearSpeed::<StageTime, TargetingPositionX>::new(speed_x),
            linear_speed_y: LinearSpeed::<StageTime, TargetingPositionY>::new(speed_y),
            linear_acceleration_y: LinearAcceleration::<StageTime, TargetingPositionY>::new(
                acceleration_y,
            ),
            linear_direction_x: LinearDirection::<StageTime, TargetingPositionX>::from_delta(
                direction_delta.x,
            ),
            linear_direction_y: LinearDirection::<StageTime, TargetingPositionY>::from_delta(
                direction_delta.y,
            ),
            linear_target_position_x: LinearTargetPosition::<StageTime, TargetingPositionX>::new(
                target.x,
            ),
            linear_target_position_y: LinearTargetPosition::<StageTime, TargetingPositionY>::new(
                target.y,
            ),
        }
    }
}

#[derive(Bundle)]
pub struct PickupFeedbackDefaultBundle {
    name: Name,
    pickup_feedback: PickupFeedback,
}

impl Default for PickupFeedbackDefaultBundle {
    fn default() -> Self {
        Self {
            name: Name::new("Pickup Healthpack Feedback"),
            pickup_feedback: PickupFeedback,
        }
    }
}

#[derive(Bundle)]
pub struct PickupFeedbackBundle {
    position: PxSubPosition,
    sprite: PxSpriteBundle<Layer>,
    movement: PickupFeedbackMovementBundle,
    default: PickupFeedbackDefaultBundle,
}

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

            let current = position.0 - camera_pos.0;
            let sprite = assets_sprite.load(assert_assets_path!(
                "sprites/pickups/health_4.px_sprite.png"
            ));

            commands.spawn(PickupFeedbackBundle {
                position: current.into(),
                sprite: PxSpriteBundle::<Layer> {
                    sprite: sprite.into(),
                    // TODO the position should be stuck to the floor beneah the dropper
                    anchor: PxAnchor::Center,
                    canvas: PxCanvas::Camera,
                    layer: Layer::Pickups,
                    ..default()
                },
                movement: PickupFeedbackMovementBundle::new(current),
                default: default(),
            });
        }
    }
}

pub type PickupDespawnFilter = (
    With<PickupFeedback>,
    Added<LinearTargetReached<StageTime, TargetingPositionY>>,
);
