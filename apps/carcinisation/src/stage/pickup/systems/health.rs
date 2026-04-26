use crate::pixel::CxAssets;
use crate::{
    components::DespawnMark,
    game::score::components::Score,
    layer::Layer,
    stage::{
        components::{
            interactive::{Dead, Health},
            placement::Depth,
        },
        depth_scale::DepthScaleConfig,
        pickup::components::{
            HealthRecovery, PickupDropPhysics, PickupFeedbackScale,
            PICKUP_FEEDBACK_GLITTER_TIME, PICKUP_FEEDBACK_GLITTER_TOGGLE_SECS,
            PICKUP_FEEDBACK_INITIAL_SPEED_Y, PICKUP_FEEDBACK_TIME, PICKUP_HUD_GLITTER_TIME,
            PickupFeedback, PickupFeedbackGlitter,
        },
        player::components::{PLAYER_MAX_HEALTH, Player},
        resources::StageTimeDomain,
        ui::hud::components::{HealthIcon, HealthText},
    },
    systems::camera::CameraPos,
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxCompositeSprite, CxFilter, CxPosition, CxPresentationTransform, CxRenderSpace,
    WorldPos,
};
use cween::linear::components::{
    LinearValueReached, TargetingValueX, TargetingValueY, TweenChildAcceleratedBundle,
};
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

/// @system Heals the player, despawns the pickup, and spawns the feedback animation.
///
/// # Panics
///
/// Panics if the camera entity is missing from the world.
#[allow(clippy::too_many_arguments)]
pub fn pickup_health(
    mut commands: Commands,
    mut score: ResMut<Score>,
    query: Query<
        (
            Entity,
            &HealthRecovery,
            &WorldPos,
            &Depth,
            Option<&CxCompositeSprite>,
        ),
        Added<Dead>,
    >,
    camera_query: Query<&WorldPos, With<CameraPos>>,
    mut player_query: Query<&mut Health, With<Player>>,
    hud_icon_query: Query<(Entity, Option<&CxFilter>), With<HealthIcon>>,
    hud_text_query: Query<(Entity, Option<&CxFilter>), With<HealthText>>,
    stage_time: Res<Time<StageTimeDomain>>,
    depth_scale_config: Res<DepthScaleConfig>,
    filters: CxAssets<CxFilter>,
) {
    let camera_pos = camera_query.single().unwrap();
    let glitter_filter = CxFilter(filters.load(assert_assets_path!("filter/color3.px_filter.png")));
    if let Ok(mut health) = player_query.single_mut() {
        for (entity, recovery, position, depth, composite_sprite_o) in query.iter() {
            commands.entity(entity).insert(DespawnMark);

            health.0 = health.0.saturating_add(recovery.0).min(PLAYER_MAX_HEALTH);
            score.add(recovery.score_deduction());

            let current = position.0 - camera_pos.0;
            let snapped = CxPosition::from(IVec2::new(
                current.x.round() as i32,
                current.y.round() as i32,
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

            // Compute depth-to-depth3 scale for the feedback visual, clamped to <= 1.0.
            let depth3_ref = crate::stage::components::placement::Depth::Three;
            let start_scale = depth_scale_config
                .fallback_scale(*depth, depth3_ref)
                .unwrap_or(1.0)
                .min(1.0);

            let mut feedback_entity_commands = commands.spawn((
                WorldPos::from(current),
                snapped,
                CxAnchor::Center,
                CxRenderSpace::Camera,
                Layer::HudUnderlay,
                TargetingValueX::from(current.x),
                TargetingValueY::from(current.y),
                PickupFeedbackDefaultBundle::default(),
                glitter,
                CxPresentationTransform::scaled(start_scale),
                PickupFeedbackScale {
                    start_scale,
                    end_scale: 1.0,
                    start_at: now,
                    end_at: glitter_end,
                },
            ));

            if let Some(composite) = composite_sprite_o {
                feedback_entity_commands.insert(composite.clone());
            }

            let feedback_entity = feedback_entity_commands.id();

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
    for (entity, mut glitter) in &mut query {
        if now < glitter.start_at {
            continue;
        }
        if now >= glitter.end_at {
            let mut entity_commands = commands.entity(entity);
            if glitter.filter_on {
                if let Some(original_filter) = glitter.original_filter.clone() {
                    entity_commands.insert(original_filter);
                } else {
                    entity_commands.remove::<CxFilter>();
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
                    entity_commands.remove::<CxFilter>();
                }
            } else {
                entity_commands.insert(glitter.glitter_filter.clone());
            }
            glitter.filter_on = !glitter.filter_on;
        }
    }
}

/// @system Interpolates `PickupFeedbackScale` on feedback entities over time.
pub fn update_pickup_feedback_scale(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<(Entity, &PickupFeedbackScale, &mut CxPresentationTransform)>,
) {
    let now = stage_time.elapsed();
    for (entity, scale_comp, mut presentation) in &mut query {
        if now >= scale_comp.end_at {
            presentation.scale = Vec2::splat(scale_comp.end_scale);
            commands.entity(entity).remove::<PickupFeedbackScale>();
            continue;
        }
        let elapsed = now.saturating_sub(scale_comp.start_at);
        let total = scale_comp.end_at.saturating_sub(scale_comp.start_at);
        let t = if total.is_zero() {
            1.0
        } else {
            elapsed.as_secs_f32() / total.as_secs_f32()
        };
        let scale = scale_comp.start_scale + (scale_comp.end_scale - scale_comp.start_scale) * t;
        presentation.scale = Vec2::splat(scale);
    }
}

/// @system Applies velocity and gravity to pickup drop arcs, clamping to floor.
pub fn tick_pickup_drop_physics(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<(Entity, &mut WorldPos, &mut PickupDropPhysics)>,
) {
    let dt = stage_time.delta_secs();
    for (entity, mut pos, mut physics) in &mut query {
        physics.velocity_y -= physics.gravity * dt;
        pos.0.y += physics.velocity_y * dt;

        if pos.0.y <= physics.floor_y {
            pos.0.y = physics.floor_y;
            commands.entity(entity).remove::<PickupDropPhysics>();
        }
    }
}
