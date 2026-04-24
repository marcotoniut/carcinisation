use crate::stage::{
    components::{
        interactive::Dead,
        placement::{Airborne, AnchorOffsets, Depth, Speed},
    },
    enemy::components::{
        CircleAround, Enemy, EnemyContinuousDepth,
        behavior::{
            BehaviorBundle, EnemyBehaviorTimer, EnemyBehaviors, EnemyCurrentBehavior,
            EnemyStepTweenChild, GroundedEnemyFall, resolve_jump_target_y,
        },
    },
    enemy::data::steps::EnemyStep,
    enemy::mosquito::entity::EnemyMosquitoAttacking,
    enemy::mosquiton::{
        entity::{EnemyMosquiton, MOSQUITON_MAX_RANGED_DEPTH},
        patterns::{mosquiton_approach_to_band, mosquiton_hold_and_shoot, mosquiton_retreat},
    },
    enemy::spidey::entity::{EnemySpidey, EnemySpideyBehaviorLoop},
    floors::ActiveFloors,
    resources::{StageGravity, StageTimeDomain},
};
use bevy::{ecs::hierarchy::ChildOf, prelude::*};
use carapace::prelude::WorldPos;
use cween::linear::components::{TargetingValueX, TargetingValueY, TargetingValueZ};
use std::time::Duration;

const MOSQUITON_RETREAT_AFTER_SHOT_WINDOW: Duration = Duration::from_millis(1600);

fn mosquiton_should_retreat(
    depth: Depth,
    attacking: &EnemyMosquitoAttacking,
    now: Duration,
) -> bool {
    depth == MOSQUITON_MAX_RANGED_DEPTH
        && attacking.last_attack_started > Duration::ZERO
        && now.saturating_sub(attacking.last_attack_started) <= MOSQUITON_RETREAT_AFTER_SHOT_WINDOW
}

fn refill_mosquiton_behaviors_if_empty(
    behaviors: &mut EnemyBehaviors,
    depth: Depth,
    attacking: &EnemyMosquitoAttacking,
    now: Duration,
) {
    if !behaviors.0.is_empty() {
        return;
    }

    behaviors.0 = if depth.to_i8() > MOSQUITON_MAX_RANGED_DEPTH.to_i8() {
        mosquiton_approach_to_band()
    } else if mosquiton_should_retreat(depth, attacking, now) {
        mosquiton_retreat(depth)
    } else {
        mosquiton_hold_and_shoot(depth)
    };
}

/// Seeds continuous enemy depth from the current gameplay bucket when missing.
///
/// Central spawn paths should already insert [`EnemyContinuousDepth`], but
/// this keeps direct debug/test spawns from silently falling back to the old
/// transient-only model.
pub fn ensure_enemy_continuous_depth(world: &mut World) {
    let mut query = world.query_filtered::<
        (Entity, &crate::stage::components::placement::Depth),
        (With<Enemy>, Without<EnemyContinuousDepth>),
    >();
    let missing_depths = query
        .iter(world)
        .map(|(entity, depth)| (entity, *depth))
        .collect::<Vec<_>>();

    for (entity, depth) in missing_depths {
        world
            .entity_mut(entity)
            .insert(EnemyContinuousDepth::from_depth(depth));
    }
}

/// @system Assigns the next behavior step to enemies with no active behavior.
#[allow(clippy::too_many_lines)]
pub fn check_no_behavior(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut EnemyBehaviors,
            &WorldPos,
            &Speed,
            &EnemyContinuousDepth,
            &Depth,
            Option<&AnchorOffsets>,
            Option<&EnemyMosquiton>,
            Option<&EnemyMosquitoAttacking>,
            Option<&EnemySpidey>,
            Option<&EnemySpideyBehaviorLoop>,
            Has<GroundedEnemyFall>,
        ),
        (With<Enemy>, Without<EnemyCurrentBehavior>),
    >,
    floors: Res<ActiveFloors>,
    stage_gravity: Res<StageGravity>,
    stage_time: Res<Time<StageTimeDomain>>,
) {
    for (
        entity,
        mut behaviors,
        position,
        speed,
        continuous_depth,
        depth,
        anchor_offsets,
        mosquiton,
        attacking,
        spidey,
        spidey_loop,
        grounded_fall,
    ) in &mut query
    {
        if grounded_fall {
            continue;
        }

        if mosquiton.is_some() {
            refill_mosquiton_behaviors_if_empty(
                &mut behaviors,
                *depth,
                attacking.expect("mosquitons should always have mosquito attack state"),
                stage_time.elapsed(),
            );
        }
        if spidey.is_some()
            && behaviors.0.is_empty()
            && let Some(spidey_loop) = spidey_loop
        {
            behaviors.0 = spidey_loop.0.clone();
        }

        let behavior = behaviors.next_step();

        let duration_o = behavior.get_duration_o();

        let current_behavior = EnemyCurrentBehavior {
            started: stage_time.elapsed(),
            behavior,
        };

        let jump_target_y = match current_behavior.behavior {
            EnemyStep::Jump(step) => Some(resolve_jump_target_y(
                step,
                *continuous_depth,
                &floors,
                anchor_offsets.map_or(0.0, |offsets| offsets.ground),
            )),
            _ => None,
        };

        let bundles = current_behavior.get_bundles(
            stage_time.elapsed(),
            position,
            speed.0,
            *continuous_depth,
            stage_gravity.acceleration,
            jump_target_y,
        );
        match bundles {
            BehaviorBundle::Idle | BehaviorBundle::Attack => {}
            BehaviorBundle::Jump(jump_movement) => {
                let has_depth_motion = matches!(
                    current_behavior.behavior,
                    EnemyStep::Jump(crate::stage::enemy::data::steps::JumpEnemyStep {
                        depth_movement: Some(_),
                        ..
                    })
                );
                commands.entity(entity).insert((
                    TargetingValueX::new(position.0.x),
                    TargetingValueY::new(position.0.y),
                ));
                if has_depth_motion {
                    commands
                        .entity(entity)
                        .insert(TargetingValueZ::new(continuous_depth.clamped_value()));
                }
                commands.entity(entity).insert(Airborne);
                // Spawn tween children to drive the jump arc movement.
                current_behavior.spawn_tween_children(
                    &mut commands,
                    entity,
                    position,
                    speed.0,
                    *continuous_depth,
                    stage_gravity.acceleration,
                    jump_target_y,
                );
                commands.entity(entity).insert(jump_movement);
            }
            BehaviorBundle::LinearTween(linear_movement) => {
                let has_depth_motion = matches!(
                    current_behavior.behavior,
                    EnemyStep::LinearTween(
                        crate::stage::enemy::data::steps::LinearTweenEnemyStep {
                            depth_movement_o: Some(_),
                            ..
                        }
                    )
                );
                commands.entity(entity).insert((
                    linear_movement,
                    TargetingValueX::new(position.0.x),
                    TargetingValueY::new(position.0.y),
                ));
                if has_depth_motion {
                    commands
                        .entity(entity)
                        .insert(TargetingValueZ::new(continuous_depth.clamped_value()));
                }

                // Spawn tween children to actually drive the movement
                current_behavior.spawn_tween_children(
                    &mut commands,
                    entity,
                    position,
                    speed.0,
                    *continuous_depth,
                    stage_gravity.acceleration,
                    None,
                );
            }
            BehaviorBundle::Circle(bundles) => {
                commands.entity(entity).insert(bundles);
            }
        }

        commands
            .entity(entity)
            .insert(current_behavior)
            .with_children(|p0| {
                if let Some(duration) = duration_o {
                    p0.spawn(EnemyBehaviorTimer::new(entity, duration));
                }
            });
    }
}

/// Applies gravity to grounded enemies that started above a floor and are not in a jump tween.
pub fn apply_grounded_enemy_fall(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    stage_gravity: Res<StageGravity>,
    floors: Res<ActiveFloors>,
    mut query: Query<
        (
            Entity,
            &mut WorldPos,
            &mut GroundedEnemyFall,
            &Depth,
            Option<&AnchorOffsets>,
        ),
        (With<Enemy>, Without<Dead>),
    >,
) {
    const TERMINAL_VELOCITY: f32 = 600.0;

    let delta = stage_time.delta_secs();
    let gravity = stage_gravity.acceleration;

    for (entity, mut position, mut falling, depth, anchor_offsets) in &mut query {
        let ground_anchor = anchor_offsets.map_or(0.0, |offsets| offsets.ground);
        let pre_contact_y = position.0.y - ground_anchor;
        let floor_y = floors.highest_solid_y_at_or_below(*depth, pre_contact_y);

        falling.vertical_velocity -= gravity * delta;
        falling.vertical_velocity = falling.vertical_velocity.max(-TERMINAL_VELOCITY);
        position.0.y += falling.vertical_velocity * delta;

        let post_contact_y = position.0.y - ground_anchor;
        let Some(floor_y) = floor_y else {
            continue;
        };

        if post_contact_y <= floor_y {
            position.0.y += floor_y - post_contact_y;
            commands
                .entity(entity)
                .remove::<GroundedEnemyFall>()
                .remove::<Airborne>();
        }
    }
}

/// @system Ticks behavior timers and clears the current behavior when time expires.
pub fn tick_enemy_behavior_timer(
    mut commands: Commands,
    mut query: Query<&mut EnemyBehaviorTimer>,
    stage_time: Res<Time<StageTimeDomain>>,
) {
    for mut behavior in &mut query {
        behavior.timer.tick(stage_time.delta());
        if behavior.timer.just_finished() {
            commands
                .entity(behavior.entity)
                .remove::<EnemyCurrentBehavior>();
        }
    }
}

/// @system Removes `CircleAround` when the owning behavior ends.
// TODO could this be made into a generic?
pub fn tied_components_enemy_current_behavior_circle_around(
    mut commands: Commands,
    query: Query<Entity, (With<CircleAround>, Without<EnemyCurrentBehavior>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).remove::<CircleAround>();
    }
}

/// @system Despawns tween children when their parent enemy's behavior ends.
///
/// Tween children (`EnemyStepTweenChild`) are spawned to drive `LinearTween` movement
/// but are not part of Bevy's hierarchy system. Without cleanup, they orphan when
/// behaviors transition, causing memory leaks.
///
/// This system:
/// 1. Queries all tween children with their parent reference (`ChildOf`)
/// 2. Checks if parent still has `EnemyCurrentBehavior`
/// 3. Despawns orphaned children whose parent behavior ended
pub fn cleanup_orphaned_tween_children(
    mut commands: Commands,
    tween_children_query: Query<(Entity, &ChildOf), With<EnemyStepTweenChild>>,
    parent_query: Query<(), With<EnemyCurrentBehavior>>,
) {
    for (child_entity, child_of) in &tween_children_query {
        // Check if parent still has active behavior
        if parent_query.get(child_of.0).is_err() {
            // Parent behavior ended, despawn orphaned tween child
            commands.entity(child_entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::{
        components::placement::{AnchorOffsets, Depth, Speed},
        enemy::{
            data::steps::{EnemyStep, IdleEnemyStep, JumpEnemyStep},
            mosquito::entity::EnemyMosquitoAttacking,
            mosquiton::entity::EnemyMosquiton,
        },
        floors::{ActiveFloors, Surface},
        resources::StageGravity,
    };
    use cween::linear::components::{LinearTargetValue, TargetingValueY};
    use std::{collections::VecDeque, time::Duration};

    #[test]
    fn ensure_enemy_continuous_depth_is_available_same_update() {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.insert_resource(StageGravity::standard());
        app.insert_resource(ActiveFloors::default());
        app.add_systems(
            Update,
            (
                ensure_enemy_continuous_depth.before(check_no_behavior),
                check_no_behavior,
            ),
        );

        let entity = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyBehaviors::new(VecDeque::from([EnemyStep::Idle(
                    IdleEnemyStep::base().with_duration(Duration::ZERO.as_secs_f32()),
                )])),
                WorldPos::default(),
                Speed(1.0),
                Depth::Three,
            ))
            .id();

        app.update();

        let entity_ref = app.world().entity(entity);
        assert!(
            (entity_ref
                .get::<EnemyContinuousDepth>()
                .expect("continuous depth should be seeded immediately")
                .0
                - Depth::Three.to_f32())
            .abs()
                < f32::EPSILON
        );
        assert!(
            entity_ref.get::<EnemyCurrentBehavior>().is_some(),
            "behavior assignment should still happen on the same update"
        );
    }

    #[test]
    fn queue_empty_mosquiton_above_attack_band_chooses_approach_pattern() {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.insert_resource(StageGravity::standard());
        app.insert_resource(ActiveFloors::default());
        app.add_systems(
            Update,
            (
                ensure_enemy_continuous_depth.before(check_no_behavior),
                check_no_behavior,
            ),
        );

        let entity = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyMosquiton,
                crate::stage::enemy::mosquito::entity::EnemyMosquito,
                EnemyMosquitoAttacking::default(),
                EnemyBehaviors::new(VecDeque::new()),
                WorldPos::default(),
                Speed(1.0),
                Depth::Seven,
            ))
            .id();

        app.update();

        let behavior = app
            .world()
            .entity(entity)
            .get::<EnemyCurrentBehavior>()
            .expect("mosquiton should receive a replacement pattern");
        assert!(
            matches!(behavior.behavior, EnemyStep::LinearTween(_)),
            "deep mosquitons should approach before they can hold/shoot"
        );
    }

    #[test]
    fn queue_empty_mosquiton_in_attack_band_chooses_hold_pattern() {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.insert_resource(StageGravity::standard());
        app.insert_resource(ActiveFloors::default());
        app.add_systems(
            Update,
            (
                ensure_enemy_continuous_depth.before(check_no_behavior),
                check_no_behavior,
            ),
        );

        let entity = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyMosquiton,
                crate::stage::enemy::mosquito::entity::EnemyMosquito,
                EnemyMosquitoAttacking::default(),
                EnemyBehaviors::new(VecDeque::new()),
                WorldPos::default(),
                Speed(1.0),
                Depth::Six,
            ))
            .id();

        app.update();

        let behavior = app
            .world()
            .entity(entity)
            .get::<EnemyCurrentBehavior>()
            .expect("mosquiton should receive a replacement pattern");
        assert!(
            matches!(behavior.behavior, EnemyStep::Circle(_)),
            "in-band mosquitons should hold long enough to shoot"
        );
    }

    #[test]
    fn queue_empty_mosquiton_recently_shot_at_band_edge_chooses_retreat_pattern() {
        let mut app = App::new();
        let mut stage_time = Time::<StageTimeDomain>::default();
        stage_time.advance_by(Duration::from_secs(5));
        app.insert_resource(stage_time);
        app.insert_resource(StageGravity::standard());
        app.insert_resource(ActiveFloors::default());
        app.add_systems(
            Update,
            (
                ensure_enemy_continuous_depth.before(check_no_behavior),
                check_no_behavior,
            ),
        );

        let entity = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyMosquiton,
                crate::stage::enemy::mosquito::entity::EnemyMosquito,
                EnemyMosquitoAttacking {
                    attack: None,
                    last_attack_started: Duration::from_millis(3500),
                },
                EnemyBehaviors::new(VecDeque::new()),
                WorldPos::default(),
                Speed(1.0),
                Depth::Six,
            ))
            .id();

        app.update();

        let behavior = app
            .world()
            .entity(entity)
            .get::<EnemyCurrentBehavior>()
            .expect("mosquiton should receive a replacement pattern");
        assert!(
            matches!(behavior.behavior, EnemyStep::LinearTween(_)),
            "recently-shooting mosquitons at the band edge should retreat"
        );
    }

    #[test]
    fn queue_empty_mosquiton_recent_shot_deeper_than_band_still_approaches() {
        let mut app = App::new();
        let mut stage_time = Time::<StageTimeDomain>::default();
        stage_time.advance_by(Duration::from_secs(5));
        app.insert_resource(stage_time);
        app.insert_resource(StageGravity::standard());
        app.insert_resource(ActiveFloors::default());
        app.add_systems(
            Update,
            (
                ensure_enemy_continuous_depth.before(check_no_behavior),
                check_no_behavior,
            ),
        );

        let entity = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyMosquiton,
                crate::stage::enemy::mosquito::entity::EnemyMosquito,
                EnemyMosquitoAttacking {
                    attack: None,
                    last_attack_started: Duration::from_millis(4500),
                },
                EnemyBehaviors::new(VecDeque::new()),
                WorldPos::default(),
                Speed(1.0),
                Depth::Seven,
            ))
            .id();

        app.update();

        let behavior = app
            .world()
            .entity(entity)
            .get::<EnemyCurrentBehavior>()
            .expect("mosquiton should receive a replacement pattern");
        assert!(
            matches!(behavior.behavior, EnemyStep::LinearTween(_)),
            "deep mosquitons should still approach instead of retreating farther away"
        );
    }

    #[test]
    fn queue_empty_non_mosquiton_still_falls_back_to_idle() {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.insert_resource(StageGravity::standard());
        app.insert_resource(ActiveFloors::default());
        app.add_systems(
            Update,
            (
                ensure_enemy_continuous_depth.before(check_no_behavior),
                check_no_behavior,
            ),
        );

        let entity = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyBehaviors::new(VecDeque::new()),
                WorldPos::default(),
                Speed(1.0),
                Depth::Six,
            ))
            .id();

        app.update();

        let behavior = app
            .world()
            .entity(entity)
            .get::<EnemyCurrentBehavior>()
            .expect("non-mosquitons should still get the default fallback");
        assert!(
            matches!(behavior.behavior, EnemyStep::Idle(_)),
            "phase 2 should not change other enemy species"
        );
    }

    #[test]
    fn jump_behavior_targets_resolved_floor_instead_of_authored_y() {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.insert_resource(StageGravity::standard());
        let mut floors = ActiveFloors::default();
        floors
            .by_depth
            .insert(Depth::Four, vec![Surface::Solid { y: 30.0 }]);
        app.insert_resource(floors);
        app.add_systems(
            Update,
            (
                ensure_enemy_continuous_depth.before(check_no_behavior),
                check_no_behavior,
            ),
        );

        let entity = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyBehaviors::new(VecDeque::from([EnemyStep::Jump(JumpEnemyStep {
                    attacking: false,
                    coordinates: Vec2::new(88.0, 999.0),
                    depth_movement: Some(-2),
                    speed: 1.0,
                })])),
                WorldPos(Vec2::new(40.0, 70.0)),
                Speed(1.0),
                Depth::Six,
                AnchorOffsets::default(),
            ))
            .id();

        app.update();

        let target_y = {
            let world = app.world_mut();
            let mut query = world.query::<(
                &ChildOf,
                &LinearTargetValue<StageTimeDomain, TargetingValueY>,
            )>();
            query
                .iter(world)
                .find_map(|(child_of, target)| (child_of.0 == entity).then_some(target.value))
                .expect("jump tween child should target floor-resolved Y")
        };

        assert!(
            (target_y - 30.0).abs() < 0.01,
            "jump should land on floor, got {target_y}"
        );
        assert!(
            app.world().entity(entity).contains::<Airborne>(),
            "jumping enemy should be marked airborne during the arc"
        );
    }

    #[test]
    fn grounded_fall_blocks_behavior_assignment_until_landed() {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.insert_resource(StageGravity::standard());
        let mut floors = ActiveFloors::default();
        floors
            .by_depth
            .insert(Depth::Six, vec![Surface::Solid { y: 70.0 }]);
        app.insert_resource(floors);
        app.add_systems(
            Update,
            (
                ensure_enemy_continuous_depth.before(check_no_behavior),
                check_no_behavior,
            ),
        );

        let entity = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyBehaviors::new(VecDeque::from([EnemyStep::Idle(
                    IdleEnemyStep::base().with_duration(1.0),
                )])),
                WorldPos(Vec2::new(40.0, 100.0)),
                Speed(1.0),
                Depth::Six,
                GroundedEnemyFall {
                    vertical_velocity: 0.0,
                },
                Airborne,
            ))
            .id();

        app.update();

        assert!(
            app.world()
                .entity(entity)
                .get::<EnemyCurrentBehavior>()
                .is_none(),
            "falling enemies should not advance their behavior queue mid-fall"
        );
    }

    #[test]
    fn grounded_fall_snaps_to_floor_and_clears_airborne() {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.insert_resource(StageGravity::standard());
        let mut floors = ActiveFloors::default();
        floors
            .by_depth
            .insert(Depth::Six, vec![Surface::Solid { y: 70.0 }]);
        app.insert_resource(floors);
        app.add_systems(Update, apply_grounded_enemy_fall);

        let entity = app
            .world_mut()
            .spawn((
                Enemy,
                WorldPos(Vec2::new(40.0, 100.0)),
                Depth::Six,
                GroundedEnemyFall {
                    vertical_velocity: 0.0,
                },
                Airborne,
            ))
            .id();

        for _ in 0..30 {
            app.world_mut()
                .resource_mut::<Time<StageTimeDomain>>()
                .advance_by(Duration::from_millis(16));
            app.update();
        }

        let entity_ref = app.world().entity(entity);
        let pos = entity_ref
            .get::<WorldPos>()
            .expect("falling enemy should keep world position");
        assert!(
            (pos.0.y - 70.0).abs() < 0.01,
            "fall should land on floor, got {}",
            pos.0.y
        );
        assert!(
            entity_ref.get::<GroundedEnemyFall>().is_none(),
            "landing should clear fall state"
        );
        assert!(
            entity_ref.get::<Airborne>().is_none(),
            "landing should clear airborne marker"
        );
    }
}
