use crate::pixel::{PxAssets, PxSpriteBundle};
use crate::{
    components::DespawnMark,
    game::score::components::Score,
    layer::Layer,
    plugins::movement::linear::components::{
        LinearTargetReached, MovementChildAcceleratedBundle, TargetingPositionX, TargetingPositionY,
    },
    stage::{
        components::interactive::{Dead, Health},
        pickup::components::{
            HealthRecovery, PickupFeedback, PICKUP_FEEDBACK_INITIAL_SPEED_Y, PICKUP_FEEDBACK_TIME,
        },
        player::components::{Player, PLAYER_MAX_HEALTH},
        resources::StageTimeDomain,
    },
    systems::camera::CameraPos,
};
use assert_assets_path::assert_assets_path;
use bevy::{ecs::hierarchy::ChildOf, prelude::*};
use seldom_pixel::prelude::{PxAnchor, PxCanvas, PxSprite, PxSubPosition};

/// Marker component for pickup feedback movement children.
#[derive(Component, Clone, Debug)]
pub struct PickupFeedbackMovement;

#[derive(Bundle)]
struct PickupFeedbackMovementXBundle {
    movement_child: MovementChildAcceleratedBundle<StageTimeDomain, TargetingPositionX>,
    pickup_feedback_movement: PickupFeedbackMovement,
    name: Name,
}

impl PickupFeedbackMovementXBundle {
    fn new(parent: Entity, current: f32, target: f32, speed: f32) -> Self {
        Self {
            movement_child:
                MovementChildAcceleratedBundle::<StageTimeDomain, TargetingPositionX>::new(
                    parent, current, target, speed, 0.0,
                ),
            pickup_feedback_movement: PickupFeedbackMovement,
            name: Name::new("Pickup Feedback Movement X"),
        }
    }
}

#[derive(Bundle)]
struct PickupFeedbackMovementYBundle {
    movement_child: MovementChildAcceleratedBundle<StageTimeDomain, TargetingPositionY>,
    pickup_feedback_movement: PickupFeedbackMovement,
    name: Name,
}

impl PickupFeedbackMovementYBundle {
    fn new(parent: Entity, current: f32, target: f32, speed: f32, acceleration: f32) -> Self {
        Self {
            movement_child:
                MovementChildAcceleratedBundle::<StageTimeDomain, TargetingPositionY>::new(
                    parent,
                    current,
                    target,
                    speed,
                    acceleration,
                ),
            pickup_feedback_movement: PickupFeedbackMovement,
            name: Name::new("Pickup Feedback Movement Y"),
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
    targeting_position_x: TargetingPositionX,
    targeting_position_y: TargetingPositionY,
    default: PickupFeedbackDefaultBundle,
}

pub fn pickup_health(
    mut commands: Commands,
    mut score: ResMut<Score>,
    query: Query<(Entity, &HealthRecovery, &PxSubPosition), Added<Dead>>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    mut player_query: Query<&mut Health, With<Player>>,
    assets_sprite: PxAssets<PxSprite>,
) {
    let camera_pos = camera_query.single().unwrap();
    if let Ok(mut health) = player_query.single_mut() {
        for (entity, recovery, position) in query.iter() {
            commands.entity(entity).insert(DespawnMark);

            health.0 = health.0.saturating_add(recovery.0).min(PLAYER_MAX_HEALTH);
            score.add(recovery.score_deduction());

            let current = position.0 - camera_pos.0;
            let sprite = assets_sprite.load(assert_assets_path!(
                "sprites/pickups/health_4.px_sprite.png"
            ));

            let t = PICKUP_FEEDBACK_TIME;
            let target = Vec2::new(12., 8.);
            let d = target - current;

            let speed_x = d.x / t;
            let speed_y = PICKUP_FEEDBACK_INITIAL_SPEED_Y;
            let adjusted_d_y = d.y - speed_y * t;
            let acceleration_y = 2. * adjusted_d_y / (t * t);

            let feedback_entity = commands
                .spawn(PickupFeedbackBundle {
                    position: current.into(),
                    sprite: PxSpriteBundle::<Layer> {
                        sprite: sprite.into(),
                        // TODO the position should be stuck to the floor beneath the dropper
                        anchor: PxAnchor::Center,
                        canvas: PxCanvas::Camera,
                        layer: Layer::Pickups,
                        ..default()
                    },
                    targeting_position_x: current.x.into(),
                    targeting_position_y: current.y.into(),
                    default: default(),
                })
                .id();

            // Spawn movement children for X (constant speed) and Y (accelerated)
            commands.spawn(PickupFeedbackMovementXBundle::new(
                feedback_entity,
                current.x,
                target.x,
                speed_x,
            ));

            commands.spawn(PickupFeedbackMovementYBundle::new(
                feedback_entity,
                current.y,
                target.y,
                speed_y,
                acceleration_y,
            ));
        }
    }
}

/// @system Marks pickup feedback for despawn when its Y-axis movement child reaches the target.
pub fn mark_pickup_feedback_for_despawn(
    mut commands: Commands,
    mut parent_query: Query<Entity, With<PickupFeedback>>,
    child_query: Query<
        &ChildOf,
        (
            With<PickupFeedbackMovement>,
            Added<LinearTargetReached<StageTimeDomain, TargetingPositionY>>,
        ),
    >,
) {
    for child_of in child_query.iter() {
        if let Ok(parent_entity) = parent_query.get_mut(child_of.0) {
            commands.entity(parent_entity).insert(DespawnMark);
        }
    }
}
