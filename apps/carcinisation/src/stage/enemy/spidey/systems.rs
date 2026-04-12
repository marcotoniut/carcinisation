use super::entity::{EnemySpidey, EnemySpideyAnimation, EnemySpideyAttacking};
use crate::{
    components::DespawnMark,
    game::score::components::Score,
    stage::{
        components::interactive::Dead,
        enemy::{
            components::{
                behavior::{EnemyCurrentBehavior, JumpTween},
                composed_state::Dying,
            },
            composed::{ComposedAnimationState, ComposedPartStates},
            data::{
                spidey::{
                    TAG_IDLE, TAG_JUMP, TAG_LANDING, TAG_LOUNGE, TAG_SHOOT,
                    apply_spidey_animation_state_with_hold,
                },
                steps::{EnemyStep, JumpEnemyStep},
            },
        },
        resources::StageTimeDomain,
    },
};
use bevy::prelude::*;

/// Selects the animation tag for a spidey based on its current behavior.
///
/// Dead spideys are excluded to prevent overriding death animations.
pub fn assign_spidey_animation(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<
        (
            Entity,
            &EnemyCurrentBehavior,
            Option<&EnemySpideyAttacking>,
            Option<&EnemySpideyAnimation>,
            Option<&JumpTween>,
            &mut ComposedAnimationState,
        ),
        (With<EnemySpidey>, Without<Dead>),
    >,
) {
    for (entity, behavior, attacking, current_animation, jump_tween, mut animation_state) in
        &mut query
    {
        let (next_animation, next_tag, hold_last_frame) = if attacking.is_some() {
            (EnemySpideyAnimation::Shoot, TAG_SHOOT, false)
        } else {
            match behavior.behavior {
                EnemyStep::Jump(JumpEnemyStep { .. }) => {
                    let jump_progress = jump_tween
                        .map(|jump| jump.progress_at(stage_time.elapsed()))
                        .unwrap_or(0.0);
                    if jump_progress < 0.5 {
                        (EnemySpideyAnimation::Jump, TAG_JUMP, true)
                    } else {
                        (EnemySpideyAnimation::Landing, TAG_LANDING, false)
                    }
                }
                EnemyStep::Idle { .. } => (EnemySpideyAnimation::Lounge, TAG_LOUNGE, false),
                EnemyStep::Attack { .. }
                | EnemyStep::Circle { .. }
                | EnemyStep::LinearTween { .. } => (EnemySpideyAnimation::Idle, TAG_IDLE, false),
            }
        };

        if current_animation != Some(&next_animation) {
            commands.entity(entity).insert(next_animation);
        }
        apply_spidey_animation_state_with_hold(&mut animation_state, next_tag, hold_last_frame);
    }
}

/// Awards score and begins the death effect when a spidey takes lethal damage.
pub fn despawn_dead_spideys(
    mut commands: Commands,
    mut score: ResMut<Score>,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<
        (
            Entity,
            &EnemySpidey,
            &mut ComposedAnimationState,
            &mut ComposedPartStates,
        ),
        Added<Dead>,
    >,
) {
    for (entity, spidey, _animation_state, mut part_states) in &mut query {
        info!("Spidey {:?} died - beginning death effect", entity);

        // Clear all active hit blinks to prevent flickering during death.
        for (_part_id, part_state) in part_states.iter_mut() {
            part_state.hit_blink = None;
        }

        commands.entity(entity).insert(Dying {
            started: stage_time.elapsed(),
        });

        score.add_u(spidey.kill_score());
    }
}

/// Applies a progressive pixel-disappearing effect to dying spideys.
///
/// Reuses the same visual pattern as the mosquiton death effect: parts
/// flicker with increasing frequency before being fully hidden, then the
/// entity is despawned.
///
/// # Panics
///
/// Panics if `stage_time.elapsed()` is less than `dying.started`, which should
/// never happen in normal operation as the death timestamp is set from the same clock.
pub fn update_spidey_death_effect(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<(Entity, &Dying, &mut ComposedPartStates), With<EnemySpidey>>,
) {
    use std::time::Duration;

    const DEATH_DURATION: Duration = Duration::from_millis(750);

    for (entity, dying, mut part_states) in &mut query {
        let elapsed = stage_time.elapsed().checked_sub(dying.started).unwrap();

        if elapsed >= DEATH_DURATION {
            info!("Spidey {:?} death effect complete - despawning", entity);
            commands.entity(entity).insert(DespawnMark);
            continue;
        }

        for (_part_id, part_state) in part_states.iter_mut() {
            let part_progress = elapsed.as_secs_f32() / DEATH_DURATION.as_secs_f32();

            if part_progress >= 1.0 {
                part_state.visible = false;
            } else if part_progress >= 0.9 {
                // Fast flicker - visible only 25% of the time.
                part_state.visible = (elapsed.as_millis() / 50).is_multiple_of(4);
            } else if part_progress >= 0.7 {
                // Medium flicker - visible 50% of the time.
                part_state.visible = (elapsed.as_millis() / 75).is_multiple_of(2);
            } else if part_progress >= 0.4 {
                // Slow flicker - visible 75% of the time.
                part_state.visible = (elapsed.as_millis() / 100) % 4 != 3;
            }
            // else: fully visible (part_progress < 0.4)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::enemy::data::spidey::{TAG_JUMP, TAG_LANDING};
    use std::time::Duration;

    #[test]
    fn jump_animation_holds_terminal_frame_then_switches_to_landing() {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.add_systems(Update, assign_spidey_animation);

        let entity = app
            .world_mut()
            .spawn((
                EnemySpidey,
                EnemyCurrentBehavior {
                    started: Duration::ZERO,
                    behavior: EnemyStep::Jump(JumpEnemyStep::base()),
                },
                JumpTween::new(Duration::ZERO, 1.0, false),
                ComposedAnimationState::new(TAG_IDLE),
            ))
            .id();

        app.update();
        {
            let world = app.world();
            let state = world
                .entity(entity)
                .get::<ComposedAnimationState>()
                .expect("spidey should have animation state");
            let animation = world
                .entity(entity)
                .get::<EnemySpideyAnimation>()
                .expect("spidey semantic animation should be assigned");
            assert_eq!(state.requested_tag, TAG_JUMP);
            assert!(state.hold_last_frame);
            assert_eq!(*animation, EnemySpideyAnimation::Jump);
        }

        app.world_mut()
            .resource_mut::<Time<StageTimeDomain>>()
            .advance_by(Duration::from_millis(600));
        app.update();

        let world = app.world();
        let state = world
            .entity(entity)
            .get::<ComposedAnimationState>()
            .expect("spidey should have animation state");
        let animation = world
            .entity(entity)
            .get::<EnemySpideyAnimation>()
            .expect("spidey semantic animation should be assigned");
        assert_eq!(state.requested_tag, TAG_LANDING);
        assert!(!state.hold_last_frame);
        assert_eq!(*animation, EnemySpideyAnimation::Landing);
    }
}
