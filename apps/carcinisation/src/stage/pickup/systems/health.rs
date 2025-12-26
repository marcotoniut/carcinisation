use crate::pixel::{PxAssets, PxSpriteBundle};
use crate::{
    components::DespawnMark,
    game::score::components::Score,
    layer::Layer,
    stage::{
        components::interactive::{Dead, Health},
        pickup::components::{
            HealthRecovery, PickupFeedback, PickupFeedbackGlitter, PICKUP_FEEDBACK_GLITTER_TIME,
            PICKUP_FEEDBACK_GLITTER_TOGGLE_SECS, PICKUP_FEEDBACK_INITIAL_SPEED_Y,
            PICKUP_FEEDBACK_TIME, PICKUP_HUD_GLITTER_TIME,
        },
        player::components::{Player, PLAYER_MAX_HEALTH},
        resources::StageTimeDomain,
        ui::hud::components::{HealthIcon, HealthText},
    },
    systems::camera::CameraPos,
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use cween::linear::components::{
    LinearValueReached, TargetingValueX, TargetingValueY, TweenChildAcceleratedBundle,
};
use seldom_pixel::prelude::{PxAnchor, PxCanvas, PxFilter, PxSprite, PxSubPosition};
use std::time::Duration;

/// Marker component for pickup feedback tween children.
#[derive(Component, Clone, Debug)]
pub struct PickupFeedbackTween;

#[derive(Bundle)]
struct PickupFeedbackTweenXBundle {
    tween_child: TweenChildAcceleratedBundle<StageTimeDomain, TargetingValueX>,
    pickup_feedback_tween: PickupFeedbackTween,
    name: Name,
}

impl PickupFeedbackTweenXBundle {
    fn new(parent: Entity, current: f32, target: f32, speed: f32) -> Self {
        Self {
            tween_child: TweenChildAcceleratedBundle::<StageTimeDomain, TargetingValueX>::new(
                parent, current, target, speed, 0.0,
            ),
            pickup_feedback_tween: PickupFeedbackTween,
            name: Name::new("Pickup Feedback Tween X"),
        }
    }
}

#[derive(Bundle)]
struct PickupFeedbackTweenYBundle {
    tween_child: TweenChildAcceleratedBundle<StageTimeDomain, TargetingValueY>,
    pickup_feedback_tween: PickupFeedbackTween,
    name: Name,
}

impl PickupFeedbackTweenYBundle {
    fn new(parent: Entity, current: f32, target: f32, speed: f32, acceleration: f32) -> Self {
        Self {
            tween_child: TweenChildAcceleratedBundle::<StageTimeDomain, TargetingValueY>::new(
                parent,
                current,
                target,
                speed,
                acceleration,
            ),
            pickup_feedback_tween: PickupFeedbackTween,
            name: Name::new("Pickup Feedback Tween Y"),
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
    targeting_value_x: TargetingValueX,
    targeting_value_y: TargetingValueY,
    default: PickupFeedbackDefaultBundle,
}

pub fn pickup_health(
    mut commands: Commands,
    mut score: ResMut<Score>,
    query: Query<(Entity, &HealthRecovery, &PxSubPosition), Added<Dead>>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    mut player_query: Query<&mut Health, With<Player>>,
    hud_icon_query: Query<(Entity, Option<&PxFilter>), With<HealthIcon>>,
    hud_text_query: Query<(Entity, Option<&PxFilter>), With<HealthText>>,
    stage_time: Res<Time<StageTimeDomain>>,
    assets_sprite: PxAssets<PxSprite>,
    filters: PxAssets<PxFilter>,
) {
    let camera_pos = camera_query.single().unwrap();
    let glitter_filter = PxFilter(filters.load(assert_assets_path!("filter/color3.px_filter.png")));
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

            let now = stage_time.elapsed();
            let glitter_time = (PICKUP_FEEDBACK_TIME - PICKUP_FEEDBACK_GLITTER_TIME).max(0.0);
            let glitter_start = now + Duration::from_secs_f32(glitter_time);
            let glitter_end = now + Duration::from_secs_f32(PICKUP_FEEDBACK_TIME);
            let glitter = PickupFeedbackGlitter::new(
                glitter_start,
                glitter_end,
                Duration::from_secs_f32(PICKUP_FEEDBACK_GLITTER_TOGGLE_SECS),
                glitter_filter.clone(),
                None,
            );

            let feedback_entity = commands
                .spawn(PickupFeedbackBundle {
                    position: current.into(),
                    sprite: PxSpriteBundle::<Layer> {
                        sprite: sprite.into(),
                        // TODO the position should be stuck to the floor beneath the dropper
                        anchor: PxAnchor::Center,
                        canvas: PxCanvas::Camera,
                        layer: Layer::HudUnderlay,
                        ..default()
                    },
                    targeting_value_x: current.x.into(),
                    targeting_value_y: current.y.into(),
                    default: default(),
                })
                .insert(glitter)
                .id();

            let hud_glitter_start = now;
            let hud_glitter_end = now + Duration::from_secs_f32(PICKUP_HUD_GLITTER_TIME);
            for (entity, current_filter) in hud_icon_query.iter().chain(hud_text_query.iter()) {
                commands.entity(entity).insert(PickupFeedbackGlitter::new(
                    hud_glitter_start,
                    hud_glitter_end,
                    Duration::from_secs_f32(PICKUP_FEEDBACK_GLITTER_TOGGLE_SECS),
                    glitter_filter.clone(),
                    current_filter.cloned(),
                ));
            }

            // Spawn tween children for X (constant speed) and Y (accelerated)
            commands.spawn(PickupFeedbackTweenXBundle::new(
                feedback_entity,
                current.x,
                target.x,
                speed_x,
            ));

            commands.spawn(PickupFeedbackTweenYBundle::new(
                feedback_entity,
                current.y,
                target.y,
                speed_y,
                acceleration_y,
            ));
        }
    }
}

/// @system Marks pickup feedback for despawn when its Y-axis tween child reaches the target.
pub fn mark_pickup_feedback_for_despawn(
    mut commands: Commands,
    query: Query<
        Entity,
        (
            With<PickupFeedback>,
            Added<LinearValueReached<StageTimeDomain, TargetingValueY>>,
        ),
    >,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(DespawnMark);
    }
}

/// @system Applies a glitter filter as pickup feedback reaches its destination.
pub fn update_pickup_feedback_glitter(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<(Entity, &mut PickupFeedbackGlitter)>,
) {
    let now = stage_time.elapsed();
    for (entity, mut glitter) in query.iter_mut() {
        if now < glitter.start_at {
            continue;
        }
        if now >= glitter.end_at {
            let mut entity_commands = commands.entity(entity);
            if glitter.filter_on {
                if let Some(original_filter) = glitter.original_filter.clone() {
                    entity_commands.insert(original_filter);
                } else {
                    entity_commands.remove::<PxFilter>();
                }
            }
            entity_commands.remove::<PickupFeedbackGlitter>();
            continue;
        }
        if now >= glitter.next_toggle_at {
            glitter.next_toggle_at = now + glitter.toggle_interval;
            let mut entity_commands = commands.entity(entity);
            if glitter.filter_on {
                if let Some(original_filter) = glitter.original_filter.clone() {
                    entity_commands.insert(original_filter);
                } else {
                    entity_commands.remove::<PxFilter>();
                }
            } else {
                entity_commands.insert(glitter.glitter_filter.clone());
            }
            glitter.filter_on = !glitter.filter_on;
        }
    }
}
