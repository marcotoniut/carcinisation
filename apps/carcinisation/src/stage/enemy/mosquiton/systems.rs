use super::entity::{
    BrokenParts, Dying, EnemyMosquiton, EnemyMosquitonAnimation, FallingState, WingsBroken,
};
use crate::stage::enemy::composed::ComposedAnimationOverride;
use crate::{
    components::DespawnMark,
    game::score::components::Score,
    globals::SCREEN_RESOLUTION_F32_H,
    pixel::PxAssets,
    stage::{
        attack::spawns::blood_shot::spawn_blood_shot_attack,
        components::{
            interactive::Dead,
            placement::{Depth, Floor},
        },
        enemy::{
            components::behavior::EnemyCurrentBehavior,
            composed::{ComposedAnimationState, ComposedPartStates, ComposedResolvedParts},
            data::{
                mosquiton::{
                    MOSQUITON_WING_PART_TAGS, TAG_DEATH_FLY, TAG_FALLING, TAG_IDLE_FLY,
                    TAG_MELEE_FLY, TAG_SHOOT_FLY, apply_mosquiton_animation_state,
                },
                steps::{EnemyStep, JumpEnemyStep},
            },
            mosquito::entity::{EnemyMosquitoAttack, EnemyMosquitoAttacking},
        },
        messages::ComposedAnimationCueMessage,
        resources::StageGravity,
        resources::StageTimeDomain,
    },
    systems::camera::CameraPos,
};
use bevy::prelude::*;
use carapace::prelude::{PxSprite, PxSubPosition};

const MOSQUITON_BLOOD_SHOT_EVENT_ID: &str = "blood_shot";

/// Mosquiton keeps its wing flap loop sourced from `idle_fly` while the body
/// track switches between airborne action tags.
///
/// The composed renderer resolves that request generically via part-tag
/// overrides; this system only selects semantic animation sources.
///
/// If the mosquiton's wings are broken, it will use the falling animation
/// instead of any flying animations.
///
/// Dead mosquitons are excluded to prevent overriding death animations.
pub fn assign_mosquiton_animation(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &EnemyCurrentBehavior,
            &EnemyMosquitoAttacking,
            Option<&EnemyMosquitonAnimation>,
            Option<&WingsBroken>,
            &mut ComposedAnimationState,
            &Depth,
        ),
        (With<EnemyMosquiton>, Without<Dead>),
    >,
) {
    for (
        entity,
        behavior,
        attacking,
        current_animation,
        wings_broken,
        mut animation_state,
        _depth,
    ) in &mut query
    {
        let (next_animation, next_tag) = if wings_broken.is_some() {
            // Wings are broken - always use falling animation
            (EnemyMosquitonAnimation::Falling, TAG_FALLING)
        } else {
            // Normal flight behavior
            match attacking.attack {
                Some(EnemyMosquitoAttack::Melee | EnemyMosquitoAttack::Ranged) => {
                    let animation = match attacking.attack {
                        Some(EnemyMosquitoAttack::Melee) => EnemyMosquitonAnimation::MeleeFly,
                        Some(EnemyMosquitoAttack::Ranged) => EnemyMosquitonAnimation::ShootFly,
                        None => unreachable!("attack arm already matched on Some"),
                    };
                    let tag = match attacking.attack {
                        Some(EnemyMosquitoAttack::Melee) => TAG_MELEE_FLY,
                        Some(EnemyMosquitoAttack::Ranged) => TAG_SHOOT_FLY,
                        None => unreachable!("attack arm already matched on Some"),
                    };
                    (animation, tag)
                }
                None => match behavior.behavior {
                    EnemyStep::Attack { .. }
                    | EnemyStep::Circle { .. }
                    | EnemyStep::Idle { .. }
                    | EnemyStep::LinearTween { .. }
                    | EnemyStep::Jump(JumpEnemyStep { .. }) => {
                        (EnemyMosquitonAnimation::IdleFly, TAG_IDLE_FLY)
                    }
                },
            }
        };

        if current_animation != Some(&next_animation) {
            commands.entity(entity).insert(next_animation);
        }
        apply_mosquiton_animation_state(&mut animation_state, next_tag);
    }
}

pub fn despawn_dead_mosquitons(
    mut commands: Commands,
    mut score: ResMut<Score>,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<
        (
            Entity,
            &EnemyMosquiton,
            &mut ComposedAnimationState,
            &mut ComposedPartStates,
        ),
        Added<Dead>,
    >,
) {
    for (entity, mosquiton, mut animation_state, mut part_states) in &mut query {
        info!(
            "Mosquiton {:?} died - applying death face in current pose",
            entity
        );

        // Clear all active hit blinks to prevent flickering during death
        for (_part_id, part_state) in part_states.iter_mut() {
            part_state.hit_blink = None;
        }

        // Keep the current animation pose (falling, shooting, idle, etc.)
        // but override just the head sprite to show death eyes from death_fly animation.
        // Use sprite_only to preserve the base animation's head position and avoid misalignment.
        let head_override =
            ComposedAnimationOverride::for_part_tags_sprite_only(TAG_DEATH_FLY, ["head"]);

        // Preserve existing wing overrides if present, add head override
        let mut overrides = animation_state.part_overrides.clone();
        overrides.push(head_override);
        animation_state.set_part_overrides(overrides);

        commands.entity(entity).insert(Dying {
            started: stage_time.elapsed(),
        });

        score.add_u(mosquiton.kill_score());
    }
}

/// Consumes generic composed-animation cues and applies Mosquiton-specific gameplay.
///
/// The composed runtime owns authored timing and dispatch. Species systems own
/// the gameplay reaction so cue kinds stay generic rather than hardcoded into
/// the renderer.
#[allow(clippy::missing_panics_doc)]
pub fn trigger_mosquiton_authored_attack_cues(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    stage_time: Res<Time<StageTimeDomain>>,
    mut cue_reader: MessageReader<ComposedAnimationCueMessage>,
    query: Query<
        (
            &EnemyMosquiton,
            &PxSubPosition,
            &Depth,
            Option<&ComposedResolvedParts>,
        ),
        Without<Dead>,
    >,
) {
    let camera_pos = camera_query.single().unwrap();

    for cue in cue_reader.read() {
        if cue.id != MOSQUITON_BLOOD_SHOT_EVENT_ID
            || cue.kind != asset_pipeline::aseprite::AnimationEventKind::ProjectileSpawn
        {
            continue;
        }

        let Ok((_mosquiton, position, depth, resolved_parts)) = query.get(cue.entity) else {
            continue;
        };

        let current_pos = cue
            .part_id
            .as_deref()
            .and_then(|part_id| {
                resolved_parts
                    .and_then(|parts| parts.parts().iter().find(|part| part.part_id == part_id))
            })
            .map_or(position.0, |part| {
                part.world_point_from_local_offset(IVec2::new(
                    cue.local_offset.x,
                    cue.local_offset.y,
                ))
            });

        spawn_blood_shot_attack(
            &mut commands,
            &mut assets_sprite,
            &stage_time,
            *SCREEN_RESOLUTION_F32_H + camera_pos.0,
            current_pos,
            depth,
        );
    }
}

/// Detects when any part of the mosquiton is destroyed and tracks it.
///
/// This generic system:
/// - Tracks all broken parts in the `BrokenParts` component
/// - Adds specific behavioral markers (like `WingsBroken`) when certain parts break
/// - Initiates appropriate state changes (like falling when wings break)
///
/// Dead mosquitons are excluded since part breakage is irrelevant once dead.
pub fn detect_part_breakage(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &ComposedPartStates,
            &PxSubPosition,
            Option<&mut BrokenParts>,
        ),
        (With<EnemyMosquiton>, Without<Dead>),
    >,
) {
    for (entity, part_states, position, broken_parts) in &mut query {
        // Get or create BrokenParts component
        let mut newly_broken_parts = Vec::new();

        // Check all parts for breakage
        for (part_id, part_state) in part_states.iter() {
            if part_state.broken {
                // Check if this is a newly broken part
                let is_newly_broken = broken_parts
                    .as_ref()
                    .is_none_or(|bp| !bp.is_broken(part_id));

                if is_newly_broken {
                    newly_broken_parts.push(part_id.clone());
                }
            }
        }

        // Process newly broken parts
        if !newly_broken_parts.is_empty() {
            // Update or insert BrokenParts component
            if let Some(mut broken_parts) = broken_parts {
                for part_id in &newly_broken_parts {
                    broken_parts.mark_broken(part_id.clone());
                    info!("Mosquiton {:?} part '{}' destroyed", entity, part_id);
                }
            } else {
                let mut broken_parts = BrokenParts::default();
                for part_id in &newly_broken_parts {
                    broken_parts.mark_broken(part_id.clone());
                    info!("Mosquiton {:?} part '{}' destroyed", entity, part_id);
                }
                commands.entity(entity).insert(broken_parts);
            }

            // Wings are a single gameplay part (wings_visual). When the wing
            // part breaks, the mosquiton starts falling.
            let all_wings_broken = part_states
                .iter()
                .all(|(_, state)| !state.has_any_tag(MOSQUITON_WING_PART_TAGS) || state.broken);

            if all_wings_broken
                && newly_broken_parts.iter().any(|part_id| {
                    part_states
                        .part(part_id)
                        .is_some_and(|state| state.has_any_tag(MOSQUITON_WING_PART_TAGS))
                })
            {
                info!("Mosquiton {:?} wings destroyed - initiating fall", entity);
                commands.entity(entity).insert((
                    WingsBroken,
                    FallingState {
                        fall_start_y: position.0.y,
                        vertical_velocity: 0.0,
                        grounded: false,
                    },
                ));
            }
        }
    }
}

/// Applies gravity and falling physics to mosquitons with broken wings.
///
/// The mosquiton will fall until the body part reaches the floor for its depth.
/// Upon landing, it takes fall damage based on the drop height and transitions to grounded movement.
///
/// Dead mosquitons are excluded to prevent physics simulation during death animations.
pub fn apply_mosquiton_falling_physics(
    mut messages: MessageWriter<crate::stage::messages::DamageMessage>,
    stage_time: Res<Time<StageTimeDomain>>,
    stage_gravity: Res<StageGravity>,
    floors: Query<(&Floor, &Depth)>,
    mut query: Query<
        (
            Entity,
            &mut PxSubPosition,
            &mut FallingState,
            &Depth,
            &ComposedResolvedParts,
        ),
        (With<EnemyMosquiton>, With<WingsBroken>, Without<Dead>),
    >,
) {
    const TERMINAL_VELOCITY: f32 = 600.0; // max fall speed pixels per second

    let gravity = stage_gravity.acceleration;
    let delta = stage_time.delta_secs();

    for (entity, mut position, mut falling_state, depth, resolved_parts) in &mut query {
        if falling_state.grounded {
            continue;
        }

        // Find the floor height for this depth
        let floor_y = floors
            .iter()
            .find(|(_, floor_depth)| *floor_depth == depth)
            .map(|(floor, _)| floor.0);

        let Some(floor_y) = floor_y else {
            warn!(
                "Mosquiton {:?} at depth {:?} has no floor - cannot apply falling physics",
                entity, depth
            );
            continue;
        };

        // Find the body part and calculate relative offset
        let body_part = resolved_parts.parts().iter().find(|p| p.part_id == "body");

        let (body_relative_offset, body_half_height) = if let Some(body) = body_part {
            // Calculate offset of body pivot from entity pivot
            let offset_y = body.world_pivot_position.y - position.0.y;
            let half_height = body.frame_size.y as f32 / 2.0;
            (offset_y, half_height)
        } else {
            warn!(
                "Mosquiton {:?} has no 'body' part - using entity position for floor collision",
                entity
            );
            (0.0, 0.0)
        };

        // Apply gravity (negative because Y increases upward in this coordinate system)
        falling_state.vertical_velocity -= gravity * delta;
        falling_state.vertical_velocity = falling_state.vertical_velocity.max(-TERMINAL_VELOCITY);

        // Apply velocity
        position.0.y += falling_state.vertical_velocity * delta;

        // Calculate current body bottom position after movement
        // (subtract half_height because Y increases upward)
        let body_bottom_y = position.0.y + body_relative_offset - body_half_height;

        // Log falling state periodically
        #[allow(clippy::float_cmp)]
        if (position.0.y / 10.0).floor() != (falling_state.fall_start_y / 10.0).floor() {
            info!(
                "Mosquiton {:?} falling: y={:.1}, body_bottom={:.1}, floor={:.1}, velocity={:.1}",
                entity, position.0.y, body_bottom_y, floor_y, falling_state.vertical_velocity
            );
        }

        // Check for ground collision using body bottom
        // (use <= because body bottom falling down reaches floor at lower Y value)
        if body_bottom_y <= floor_y {
            // Snap entity position so body bottom aligns with floor
            position.0.y = floor_y - body_relative_offset + body_half_height;
            falling_state.grounded = true;
            falling_state.vertical_velocity = 0.0;

            // Calculate fall damage based on drop height
            // (fall_start_y - floor_y because Y increases upward)
            let fall_distance = falling_state.fall_start_y - floor_y;
            let fall_damage = calculate_fall_damage(fall_distance);

            info!(
                "Mosquiton {:?} landed at floor {:.1} - fell {:.1}px, taking {} damage",
                entity, floor_y, fall_distance, fall_damage
            );

            if fall_damage > 0 {
                // Apply fall damage as entity-level damage (bypasses part durability).
                // Fall damage goes directly to the entity's health pool, not individual parts.
                use crate::stage::messages::DamageMessage;
                messages.write(DamageMessage {
                    entity,
                    value: fall_damage,
                });
            }
        }
    }
}

/// Calculates fall damage based on drop height.
///
/// - Falls under 50px: no damage
/// - Falls 50-150px: 1-5 damage (linear)
/// - Falls over 150px: 5+ damage (scaling)
fn calculate_fall_damage(fall_distance: f32) -> u32 {
    const MIN_DAMAGE_HEIGHT: f32 = 50.0;
    const MEDIUM_DAMAGE_HEIGHT: f32 = 150.0;

    if fall_distance < MIN_DAMAGE_HEIGHT {
        0
    } else if fall_distance < MEDIUM_DAMAGE_HEIGHT {
        // Linear scaling: 50px = 1 damage, 150px = 5 damage
        let ratio =
            (fall_distance - MIN_DAMAGE_HEIGHT) / (MEDIUM_DAMAGE_HEIGHT - MIN_DAMAGE_HEIGHT);
        (1.0 + ratio * 4.0).round() as u32
    } else {
        // Heavy falls: 5 damage + 1 per additional 30px
        5 + ((fall_distance - MEDIUM_DAMAGE_HEIGHT) / 30.0).floor() as u32
    }
}

/// Applies a progressive pixel-disappearing effect to dying mosquitons.
///
/// Broken parts disappear faster (0.5 seconds) than intact parts (1.0 second).
/// The effect works by progressively hiding parts based on elapsed time with an
/// accelerating flicker pattern that simulates pixels disappearing.
/// After the effect completes, the entity is marked for despawn.
///
/// # Panics
///
/// Panics if `stage_time.elapsed()` is less than `dying.started`, which should never
/// happen in normal operation as the death timestamp is set from the same clock.
pub fn update_mosquiton_death_effect(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<
        (
            Entity,
            &Dying,
            Option<&BrokenParts>,
            &mut ComposedPartStates,
        ),
        With<EnemyMosquiton>,
    >,
) {
    use std::time::Duration;

    const DEATH_DURATION: Duration = Duration::from_millis(750);

    for (entity, dying, broken_parts_opt, mut part_states) in &mut query {
        let elapsed = stage_time.elapsed().checked_sub(dying.started).unwrap();

        // Check if we should despawn (after all effects complete)
        if elapsed >= DEATH_DURATION {
            info!("Mosquiton {:?} death effect complete - despawning", entity);
            commands.entity(entity).insert(DespawnMark);
            continue;
        }

        // Hide parts progressively based on whether they're broken
        let broken_part_ids: std::collections::HashSet<String> = broken_parts_opt
            .map(|bp| bp.broken_parts().clone())
            .unwrap_or_default();

        for (part_id, part_state) in part_states.iter_mut() {
            let is_broken = broken_part_ids.contains(part_id);

            // Broken parts are already invisible (broken on impact), so skip them
            if is_broken {
                continue;
            }

            let part_progress = elapsed.as_secs_f32() / DEATH_DURATION.as_secs_f32();

            // Hide intact part based on disappearing pixels effect:
            // - 0-40%: Fully visible
            // - 40-70%: Slow flicker (visible most of the time)
            // - 70-90%: Medium flicker (visible half the time)
            // - 90-100%: Fast flicker (visible rarely)
            // - 100%+: Fully invisible
            if part_progress >= 1.0 {
                // Part has fully disappeared
                part_state.visible = false;
            } else if part_progress >= 0.9 {
                // Fast flicker - visible only 25% of the time (mostly disappeared)
                part_state.visible = (elapsed.as_millis() / 50).is_multiple_of(4);
            } else if part_progress >= 0.7 {
                // Medium flicker - visible 50% of the time
                part_state.visible = (elapsed.as_millis() / 75).is_multiple_of(2);
            } else if part_progress >= 0.4 {
                // Slow flicker - visible 75% of the time (just starting to fade)
                part_state.visible = (elapsed.as_millis() / 100) % 4 != 3;
            }
            // else: fully visible (part_progress < 0.4)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::{
        enemy::{data::steps::IdleEnemyStep, mosquito::systems::clear_finished_mosquito_attacks},
        resources::StageTimeDomain,
    };
    use std::time::Duration;

    #[test]
    fn idle_spawn_state_requests_idle_fly() {
        let mut app = App::new();
        app.add_systems(Update, assign_mosquiton_animation);

        let entity = app
            .world_mut()
            .spawn((
                EnemyMosquiton,
                EnemyCurrentBehavior {
                    started: Duration::ZERO,
                    behavior: EnemyStep::Idle(crate::stage::enemy::data::steps::IdleEnemyStep {
                        duration: 99999.0,
                    }),
                },
                EnemyMosquitoAttacking::default(),
                ComposedAnimationState::new(TAG_SHOOT_FLY),
                Depth::Three,
            ))
            .id();

        app.update();

        let animation = app
            .world()
            .entity(entity)
            .get::<EnemyMosquitonAnimation>()
            .expect("animation should be assigned");
        let state = app
            .world()
            .entity(entity)
            .get::<ComposedAnimationState>()
            .expect("composed animation state should exist");

        assert_eq!(*animation, EnemyMosquitonAnimation::IdleFly);
        assert_eq!(state.requested_tag, TAG_IDLE_FLY);
    }

    #[test]
    fn ranged_attack_stays_visible_until_presentation_window_finishes() {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.add_systems(
            Update,
            (clear_finished_mosquito_attacks, assign_mosquiton_animation).chain(),
        );

        let entity = app
            .world_mut()
            .spawn((
                EnemyMosquiton,
                crate::stage::enemy::mosquito::entity::EnemyMosquito,
                EnemyCurrentBehavior {
                    started: Duration::ZERO,
                    behavior: EnemyStep::Idle(IdleEnemyStep { duration: 99999.0 }),
                },
                EnemyMosquitoAttacking {
                    attack: Some(EnemyMosquitoAttack::Ranged),
                    last_attack_started: Duration::ZERO,
                },
                ComposedAnimationState::new(TAG_IDLE_FLY),
                Depth::Three,
            ))
            .id();

        app.update();
        {
            let world = app.world();
            let animation = world
                .entity(entity)
                .get::<EnemyMosquitonAnimation>()
                .expect("animation should be assigned at attack start");
            let state = world
                .entity(entity)
                .get::<ComposedAnimationState>()
                .expect("composed animation state should exist");
            assert_eq!(*animation, EnemyMosquitonAnimation::ShootFly);
            assert_eq!(state.requested_tag, TAG_SHOOT_FLY);
        }

        app.world_mut()
            .resource_mut::<Time<StageTimeDomain>>()
            .advance_by(Duration::from_secs(1));
        app.update();
        {
            let world = app.world();
            let state = world
                .entity(entity)
                .get::<ComposedAnimationState>()
                .expect("composed animation state should exist");
            let attacking = world
                .entity(entity)
                .get::<EnemyMosquitoAttacking>()
                .expect("attack component should still exist");
            assert_eq!(state.requested_tag, TAG_SHOOT_FLY);
            assert!(matches!(
                attacking.attack,
                Some(EnemyMosquitoAttack::Ranged)
            ));
        }

        app.world_mut()
            .resource_mut::<Time<StageTimeDomain>>()
            .advance_by(Duration::from_millis(500));
        app.update();
        let world = app.world();
        let state = world
            .entity(entity)
            .get::<ComposedAnimationState>()
            .expect("composed animation state should exist");
        let attacking = world
            .entity(entity)
            .get::<EnemyMosquitoAttacking>()
            .expect("attack component should still exist");
        assert_eq!(state.requested_tag, TAG_IDLE_FLY);
        assert!(attacking.attack.is_none());
    }
}
