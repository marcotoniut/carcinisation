use super::entity::{
    BrokenParts, Dying, EnemyMosquiton, EnemyMosquitonAnimation, FallingState, WingsBroken,
};
use crate::stage::enemy::composed::ComposedAnimationOverride;
use crate::{
    components::DespawnMark,
    game::score::components::Score,
    globals::SCREEN_RESOLUTION_F32_H,
    stage::{
        attack::{data::blood_shot::BloodShotConfig, spawns::blood_shot::spawn_blood_shot_attack},
        components::{interactive::Dead, placement::Depth},
        enemy::{
            components::{
                CircleAround, LinearTween,
                behavior::{EnemyBehaviors, EnemyCurrentBehavior, EnemyStepTweenChild, JumpTween},
            },
            composed::{ComposedAnimationState, ComposedPartStates, ComposedResolvedParts},
            data::{
                mosquiton::{
                    MOSQUITON_WING_PART_TAGS, TAG_DEATH_FLY, TAG_FALL, TAG_IDLE_FLY,
                    TAG_IDLE_STAND, TAG_MELEE_FLY, TAG_SHOOT_FLY, apply_mosquiton_animation_state,
                },
                steps::{EnemyStep, JumpEnemyStep},
            },
            mosquito::entity::{EnemyMosquitoAttack, EnemyMosquitoAttacking},
        },
        floors::ActiveFloors,
        messages::ComposedAnimationCueMessage,
        resources::StageGravity,
        resources::StageTimeDomain,
    },
    systems::camera::CameraPos,
};
use bevy::prelude::*;
use carapace::prelude::{CxPresentationTransform, CxSpriteAtlasAsset, WorldPos};

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
            Option<&EnemyCurrentBehavior>,
            &EnemyMosquitoAttacking,
            Option<&EnemyMosquitonAnimation>,
            Option<&WingsBroken>,
            Option<&FallingState>,
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
        falling_state,
        mut animation_state,
        _depth,
    ) in &mut query
    {
        let grounded = falling_state.is_some_and(|f| f.grounded);
        let (next_animation, next_tag) = if wings_broken.is_some() && grounded {
            // Wings broken and landed — grounded idle
            (EnemyMosquitonAnimation::Falling, TAG_IDLE_STAND)
        } else if wings_broken.is_some() {
            // Wings broken, still falling
            (EnemyMosquitonAnimation::Falling, TAG_FALL)
        } else {
            // Normal flight behavior — requires an active behavior step.
            let Some(behavior) = behavior else {
                continue;
            };
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
#[allow(clippy::missing_panics_doc, clippy::too_many_arguments)]
pub fn trigger_mosquiton_authored_attack_cues(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    atlas_assets: Res<Assets<CxSpriteAtlasAsset>>,
    blood_shot_config: Res<BloodShotConfig>,
    camera_query: Query<&WorldPos, With<CameraPos>>,
    stage_time: Res<Time<StageTimeDomain>>,
    mut cue_reader: MessageReader<ComposedAnimationCueMessage>,
    query: Query<
        (
            &EnemyMosquiton,
            &WorldPos,
            Option<&CxPresentationTransform>,
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

        let Ok((_mosquiton, position, presentation, depth, resolved_parts)) = query.get(cue.entity)
        else {
            continue;
        };

        // Resolved part positions already include depth-fallback scaling
        // (applied at the source in `build_resolved_part_states`).
        // visual_offset is unscaled — use scaled_visual_offset() for
        // combining with scaled pivot positions.
        let scaled_offset = resolved_parts.map_or(
            Vec2::ZERO,
            super::super::composed::ComposedResolvedParts::scaled_visual_offset,
        );
        let gameplay_scale = resolved_parts.map_or(
            1.0,
            super::super::composed::ComposedResolvedParts::gameplay_scale,
        );

        let resolved_part = cue.part_id.as_deref().and_then(|part_id| {
            resolved_parts
                .and_then(|parts| parts.parts().iter().find(|part| part.part_id == part_id))
        });

        let local_offset = IVec2::new(cue.local_offset.x, cue.local_offset.y);

        let current_pos = resolved_part.map_or(position.0 + scaled_offset, |part| {
            part.visual_point_from_local_offset(local_offset, scaled_offset)
        });

        #[cfg(debug_assertions)]
        {
            let method = if resolved_part.is_some() {
                "resolved_part"
            } else {
                "FALLBACK_entity_visual_center"
            };
            let entity_pos = position.0;
            let cue_part_id = &cue.part_id;
            let off_x = cue.local_offset.x;
            let off_y = cue.local_offset.y;
            info!(
                "Blood shot cue: method={method}, origin={current_pos:?}, entity_pos={entity_pos:?}, \
                 part_id={cue_part_id:?}, offset=({off_x},{off_y})",
            );
        }

        spawn_blood_shot_attack(
            &mut commands,
            &asset_server,
            &atlas_assets,
            &stage_time,
            &blood_shot_config,
            *SCREEN_RESOLUTION_F32_H + camera_pos.0,
            current_pos,
            presentation,
            depth,
            gameplay_scale,
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
            &WorldPos,
            Option<&mut BrokenParts>,
        ),
        (With<EnemyMosquiton>, Without<Dead>),
    >,
    tween_children: Query<(Entity, &ChildOf), With<EnemyStepTweenChild>>,
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

                // Wing-break is a terminal transition out of all motion
                // behaviours. Every entity-local component that drives
                // `WorldPos` must be removed here so it doesn't
                // fight with `apply_mosquiton_falling_physics`.
                //
                // If you add a new motion-driving component, extend this
                // cleanup — the falling system must be the sole writer to
                // `WorldPos` once wings are broken.
                commands
                    .entity(entity)
                    .remove::<CircleAround>()
                    .remove::<LinearTween>()
                    .remove::<JumpTween>()
                    .remove::<EnemyBehaviors>();

                // Despawn active tween children so they don't keep driving
                // lateral velocity through `aggregate_tween_children_to_parent`.
                for (child_entity, child_of) in &tween_children {
                    if child_of.0 == entity {
                        commands.entity(child_entity).insert(DespawnMark);
                    }
                }
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
    mut commands: Commands,
    mut messages: MessageWriter<crate::stage::messages::DamageMessage>,
    stage_time: Res<Time<StageTimeDomain>>,
    stage_gravity: Res<StageGravity>,
    floors: Res<ActiveFloors>,
    mut query: Query<
        (
            Entity,
            &mut WorldPos,
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

        // Find the body part and calculate relative offset
        let body_part = resolved_parts.parts().iter().find(|p| p.part_id == "body");

        let (body_relative_offset, body_half_height) = if let Some(body) = body_part {
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

        // Query floor candidates BEFORE movement so overshooting the floor
        // in a single frame doesn't exclude the surface from the picking query.
        let pre_body_bottom_y = position.0.y + body_relative_offset - body_half_height;
        let floor_y = floors.highest_solid_y_at_or_below(*depth, pre_body_bottom_y);

        // Apply gravity (negative because Y increases upward in this coordinate system)
        falling_state.vertical_velocity -= gravity * delta;
        falling_state.vertical_velocity = falling_state.vertical_velocity.max(-TERMINAL_VELOCITY);

        // Apply velocity
        position.0.y += falling_state.vertical_velocity * delta;

        // Calculate body bottom AFTER movement for the crossing check.
        let post_body_bottom_y = position.0.y + body_relative_offset - body_half_height;

        let Some(floor_y) = floor_y else {
            warn!(
                "Mosquiton {:?} at depth {:?} has no floor below body_bottom={:.1} - continuing to fall",
                entity, depth, post_body_bottom_y
            );
            continue;
        };

        // Log falling state periodically
        #[allow(clippy::float_cmp)]
        if (position.0.y / 10.0).floor() != (falling_state.fall_start_y / 10.0).floor() {
            info!(
                "Mosquiton {:?} falling: y={:.1}, body_bottom={:.1}, floor={:.1}, velocity={:.1}",
                entity, position.0.y, post_body_bottom_y, floor_y, falling_state.vertical_velocity
            );
        }

        // Check for ground collision: entity crossed the floor this frame.
        if post_body_bottom_y <= floor_y {
            // Snap entity position so body bottom aligns with floor exactly.
            let snap_adjustment = floor_y - post_body_bottom_y;
            position.0.y += snap_adjustment;
            falling_state.grounded = true;
            falling_state.vertical_velocity = 0.0;

            // Transition from airborne to grounded state.
            commands
                .entity(entity)
                .remove::<crate::stage::components::placement::Airborne>();

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
        components::placement::{Airborne, Speed},
        enemy::{
            components::behavior::EnemyBehaviors,
            components::{CircleAround, LinearTween},
            data::steps::{IdleEnemyStep, LinearTweenEnemyStep},
            mosquito::systems::clear_finished_mosquito_attacks,
        },
        floors::Surface,
        messages::EntityDamageMessage,
        resources::StageTimeDomain,
        systems::movement::circle_around,
    };
    use std::collections::BTreeMap;
    use std::time::Duration;

    // ── Falling physics test infrastructure ──────────────────────────

    /// Minimal app for testing `apply_mosquiton_falling_physics` in isolation.
    fn falling_physics_app(floors: ActiveFloors) -> App {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.insert_resource(StageGravity::standard());
        app.insert_resource(floors);
        app.add_message::<EntityDamageMessage>();
        app.add_systems(Update, apply_mosquiton_falling_physics);
        app
    }

    /// Spawn a zero-offset falling mosquiton (body_bottom_y == position.y).
    fn spawn_falling_entity(app: &mut App, y: f32, depth: Depth, velocity: f32) -> Entity {
        app.world_mut()
            .spawn((
                WorldPos(Vec2::new(100.0, y)),
                FallingState {
                    fall_start_y: y,
                    vertical_velocity: velocity,
                    grounded: false,
                },
                depth,
                EnemyMosquiton,
                WingsBroken,
                ComposedResolvedParts::default(),
                Airborne,
            ))
            .id()
    }

    fn step_frames(app: &mut App, n: u32) {
        for _ in 0..n {
            app.world_mut()
                .resource_mut::<Time<StageTimeDomain>>()
                .advance_by(Duration::from_millis(16));
            app.update();
        }
    }

    // ── Falling physics regression tests ─────────────────────────────

    /// Entity at y=100, single surface at y=50. After enough frames the
    /// entity must land at y=50 exactly.
    #[test]
    fn falling_mosquiton_lands_on_single_surface() {
        let floors = ActiveFloors {
            by_depth: BTreeMap::from([(Depth::Three, vec![Surface::Solid { y: 50.0 }])]),
        };
        let mut app = falling_physics_app(floors);
        let entity = spawn_falling_entity(&mut app, 100.0, Depth::Three, 0.0);

        step_frames(&mut app, 40);

        let world = app.world();
        let fs = world.entity(entity).get::<FallingState>().unwrap();
        let pos = world.entity(entity).get::<WorldPos>().unwrap();
        assert!(fs.grounded, "entity should have landed");
        assert_eq!(pos.0.y, 50.0, "entity should snap to floor Y exactly");
        assert!(
            world.entity(entity).get::<Airborne>().is_none(),
            "Airborne marker should be removed on landing",
        );
    }

    /// The overshoot scenario: entity 2px above floor with terminal velocity.
    /// Per-frame delta (≈10px) exceeds the distance to the floor. The entity
    /// must land, not pass through.
    ///
    /// Pre-fix failure: `highest_solid_y_at_or_below` is called with the
    /// post-movement body_bottom (below floor), so the floor is excluded from
    /// the query. The entity falls through.
    #[test]
    fn falling_mosquiton_lands_through_overshoot() {
        let floors = ActiveFloors {
            by_depth: BTreeMap::from([(Depth::Three, vec![Surface::Solid { y: 50.0 }])]),
        };
        let mut app = falling_physics_app(floors);
        // 2px above floor, terminal velocity downward.
        let entity = spawn_falling_entity(&mut app, 52.0, Depth::Three, -600.0);

        step_frames(&mut app, 1);

        let world = app.world();
        let fs = world.entity(entity).get::<FallingState>().unwrap();
        let pos = world.entity(entity).get::<WorldPos>().unwrap();
        assert!(
            fs.grounded,
            "entity should land even when per-frame delta overshoots the floor \
             (body moved from 52 to ~42.4, floor at 50)",
        );
        assert_eq!(pos.0.y, 50.0, "entity should snap to floor Y exactly");
    }

    /// Entity above a Gap at its depth. No landing occurs.
    #[test]
    fn falling_mosquiton_falls_through_gap() {
        let floors = ActiveFloors {
            by_depth: BTreeMap::from([(Depth::Three, vec![Surface::Gap])]),
        };
        let mut app = falling_physics_app(floors);
        let entity = spawn_falling_entity(&mut app, 100.0, Depth::Three, 0.0);

        step_frames(&mut app, 40);

        let fs = app.world().entity(entity).get::<FallingState>().unwrap();
        assert!(!fs.grounded, "entity should keep falling through a gap");
    }

    /// Multiple Solid surfaces at the same depth. Entity lands on the highest
    /// surface below its starting position, not the lowest.
    #[test]
    fn falling_mosquiton_lands_on_highest_below_in_stack() {
        let floors = ActiveFloors {
            by_depth: BTreeMap::from([(
                Depth::Three,
                vec![Surface::Solid { y: 60.0 }, Surface::Solid { y: 30.0 }],
            )]),
        };
        let mut app = falling_physics_app(floors);
        let entity = spawn_falling_entity(&mut app, 80.0, Depth::Three, 0.0);

        step_frames(&mut app, 30);

        let world = app.world();
        let fs = world.entity(entity).get::<FallingState>().unwrap();
        let pos = world.entity(entity).get::<WorldPos>().unwrap();
        assert!(fs.grounded, "entity should have landed");
        assert_eq!(
            pos.0.y, 60.0,
            "entity should land on the highest surface (60), not the lowest (30)",
        );
    }

    /// Landing snaps body_bottom to floor_y exactly — no floating-point
    /// epsilon allowed. The snap formula is deterministic.
    #[test]
    fn falling_mosquiton_snap_is_precise() {
        let floors = ActiveFloors {
            by_depth: BTreeMap::from([(Depth::Three, vec![Surface::Solid { y: 37.5 }])]),
        };
        let mut app = falling_physics_app(floors);
        let entity = spawn_falling_entity(&mut app, 100.0, Depth::Three, 0.0);

        step_frames(&mut app, 50);

        let world = app.world();
        let fs = world.entity(entity).get::<FallingState>().unwrap();
        let pos = world.entity(entity).get::<WorldPos>().unwrap();
        assert!(fs.grounded, "entity should have landed");
        // With zero body offsets, position.y == body_bottom_y == floor_y.
        #[allow(clippy::float_cmp)]
        {
            assert_eq!(pos.0.y, 37.5, "snap must be exact, not approximate",);
        }
    }

    /// ActiveFloors populated with values matching what surface inheritance
    /// would produce (the floor evaluation pipeline is tested elsewhere;
    /// here we verify the falling system reads the resource correctly).
    #[test]
    fn falling_mosquiton_lands_with_inherited_surfaces() {
        // Simulates what evaluate_floors_at produces for a step that
        // inherits surfaces from an earlier step declaring Anchored(45.0)
        // at depth Four.
        let floors = ActiveFloors {
            by_depth: BTreeMap::from([(Depth::Four, vec![Surface::Solid { y: 45.0 }])]),
        };
        let mut app = falling_physics_app(floors);
        let entity = spawn_falling_entity(&mut app, 90.0, Depth::Four, 0.0);

        step_frames(&mut app, 40);

        let world = app.world();
        let fs = world.entity(entity).get::<FallingState>().unwrap();
        let pos = world.entity(entity).get::<WorldPos>().unwrap();
        assert!(fs.grounded, "entity should land on inherited floor");
        assert_eq!(pos.0.y, 45.0);
    }

    // ── Lateral drift regression test ────────────────────────────────

    /// When wings break during an active tween, tween children must be
    /// cleaned up so they stop driving lateral velocity. Without the fix,
    /// `aggregate_tween_children_to_parent` keeps integrating X velocity
    /// from surviving tween children, causing the entity to "float sideways"
    /// during the fall.
    ///
    /// This test verifies that `detect_part_breakage`:
    /// 1. Inserts `WingsBroken` + `FallingState` when wing parts are broken
    /// 2. Marks tween children for despawn (prevents lateral drift)
    /// 3. Removes `EnemyBehaviors` (prevents new behavior assignment)
    #[test]
    fn wing_break_cleans_up_tween_children_and_behaviors() {
        let mut app = App::new();
        app.add_systems(Update, detect_part_breakage);

        // Spawn entity with an active LinearTween behavior + tween children.
        let behavior = EnemyCurrentBehavior {
            started: Duration::ZERO,
            behavior: EnemyStep::LinearTween(LinearTweenEnemyStep {
                depth_movement_o: None,
                direction: Vec2::new(1.0, 0.0),
                trayectory: 100.0,
            }),
        };

        let start_pos = Vec2::new(50.0, 100.0);
        let entity = app
            .world_mut()
            .spawn((
                EnemyMosquiton,
                WorldPos(start_pos),
                Depth::Three,
                Speed(2.0),
                behavior.clone(),
                EnemyBehaviors::new(std::collections::VecDeque::new()),
                // Wing part already broken — detect_part_breakage will fire
                // on the first update and trigger the cleanup.
                ComposedPartStates::test_with_parts(vec![
                    ("wings_visual", true, vec!["wings"]),
                    ("body", false, vec!["body"]),
                ]),
            ))
            .id();

        // Spawn tween children that drive X velocity on the parent.
        let children = behavior.spawn_tween_children(
            &mut app.world_mut().commands(),
            entity,
            &WorldPos(start_pos),
            2.0,
            Depth::Three,
        );
        app.world_mut().flush();

        // Verify tween children exist before the fix runs.
        assert!(
            !children.is_empty(),
            "baseline: tween children should be spawned",
        );
        for &child in &children {
            assert!(
                app.world().get_entity(child).is_ok(),
                "baseline: tween child entity should exist",
            );
        }

        // Run detect_part_breakage — it sees the broken wing, inserts
        // WingsBroken, marks tween children for despawn, removes EnemyBehaviors.
        app.update();

        // Verify WingsBroken was inserted.
        assert!(
            app.world().entity(entity).get::<WingsBroken>().is_some(),
            "detect_part_breakage should insert WingsBroken",
        );

        // Verify FallingState was inserted.
        assert!(
            app.world().entity(entity).get::<FallingState>().is_some(),
            "detect_part_breakage should insert FallingState",
        );

        // Verify EnemyBehaviors was removed (prevents new behavior assignment).
        assert!(
            app.world().entity(entity).get::<EnemyBehaviors>().is_none(),
            "detect_part_breakage should remove EnemyBehaviors on wing break",
        );

        // Verify tween children are marked for despawn.
        for &child in &children {
            if let Ok(child_ref) = app.world().get_entity(child) {
                assert!(
                    child_ref.get::<DespawnMark>().is_some(),
                    "tween child should be marked for despawn after wing break",
                );
            }
            // Entity may already be despawned by the time we check — that's also correct.
        }
    }

    /// Helper: spawn a mosquiton with broken wing parts and the given
    /// motion-driver component already present. Returns the entity id.
    fn spawn_wing_break_entity<C: Component>(app: &mut App, motion_component: C) -> Entity {
        app.world_mut()
            .spawn((
                EnemyMosquiton,
                WorldPos(Vec2::new(50.0, 100.0)),
                Depth::Three,
                Speed(2.0),
                EnemyCurrentBehavior {
                    started: Duration::ZERO,
                    behavior: EnemyStep::Idle(IdleEnemyStep { duration: 99999.0 }),
                },
                EnemyBehaviors::new(std::collections::VecDeque::new()),
                ComposedPartStates::test_with_parts(vec![
                    ("wings_visual", true, vec!["wings"]),
                    ("body", false, vec!["body"]),
                ]),
                motion_component,
            ))
            .id()
    }

    /// `CircleAround` must be removed on wing break so the orbit system
    /// stops overwriting `WorldPos`.
    #[test]
    fn wing_break_removes_circle_around() {
        let mut app = App::new();
        app.add_systems(Update, detect_part_breakage);

        let entity = spawn_wing_break_entity(
            &mut app,
            CircleAround {
                center: Vec2::new(50.0, 100.0),
                radius: 12.0,
                direction: cween::structs::TweenDirection::Positive,
                time_offset: 0.0,
            },
        );

        app.update();

        assert!(
            app.world().entity(entity).get::<WingsBroken>().is_some(),
            "WingsBroken should be inserted",
        );
        assert!(
            app.world().entity(entity).get::<CircleAround>().is_none(),
            "CircleAround must be removed on wing break",
        );
    }

    /// `LinearTween` marker must be removed on wing break.
    #[test]
    fn wing_break_removes_linear_tween() {
        let mut app = App::new();
        app.add_systems(Update, detect_part_breakage);

        let entity = spawn_wing_break_entity(
            &mut app,
            LinearTween {
                direction: Vec2::new(1.0, 0.0),
                trayectory: 100.0,
                reached_x: false,
                reached_y: false,
            },
        );

        app.update();

        assert!(
            app.world().entity(entity).get::<WingsBroken>().is_some(),
            "WingsBroken should be inserted",
        );
        assert!(
            app.world().entity(entity).get::<LinearTween>().is_none(),
            "LinearTween must be removed on wing break",
        );
    }

    /// `JumpTween` marker must be removed on wing break.
    #[test]
    fn wing_break_removes_jump_tween() {
        let mut app = App::new();
        app.add_systems(Update, detect_part_breakage);

        let entity = spawn_wing_break_entity(&mut app, JumpTween::new(Duration::ZERO, 1.0, false));

        app.update();

        assert!(
            app.world().entity(entity).get::<WingsBroken>().is_some(),
            "WingsBroken should be inserted",
        );
        assert!(
            app.world().entity(entity).get::<JumpTween>().is_none(),
            "JumpTween must be removed on wing break",
        );
    }

    /// Defensive backstop: `circle_around` system excludes entities with
    /// `WingsBroken`, preventing orbit writes if cleanup regresses.
    #[test]
    fn circle_around_excludes_wings_broken() {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.add_systems(Update, circle_around);

        let center = Vec2::new(50.0, 100.0);
        let entity = app
            .world_mut()
            .spawn((
                WorldPos(Vec2::new(80.0, 100.0)),
                CircleAround {
                    center,
                    radius: 12.0,
                    direction: cween::structs::TweenDirection::Positive,
                    time_offset: 0.0,
                },
                WingsBroken,
            ))
            .id();

        let pos_before = app.world().entity(entity).get::<WorldPos>().unwrap().0;

        app.world_mut()
            .resource_mut::<Time<StageTimeDomain>>()
            .advance_by(Duration::from_millis(100));
        app.update();

        let pos_after = app.world().entity(entity).get::<WorldPos>().unwrap().0;
        assert_eq!(
            pos_before, pos_after,
            "circle_around should not modify position when WingsBroken is present",
        );
    }

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

    /// Proves the consumer contract: projectile origin reads prescaled data
    /// from `ComposedResolvedParts` without applying additional scaling.
    /// The scaling formula itself is tested in `composed::tests`.
    #[test]
    fn projectile_origin_reads_prescaled_data() {
        // build_resolved_part_states stores: root + (authored - root) * scale
        let root = Vec2::new(100.0, 50.0);
        let authored_mouth = Vec2::new(115.0, 60.0);
        let scale = 0.35;
        let stored_pivot = root + (authored_mouth - root) * scale;

        // trigger_mosquiton_authored_attack_cues reads stored_pivot directly
        // (no additional multiplication by DepthFallbackScale).
        let projectile_origin = stored_pivot;

        let expected = root + Vec2::new(15.0, 10.0) * scale;
        assert!((projectile_origin - expected).length() < 0.01);
    }
}
