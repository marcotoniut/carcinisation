use super::entity::{EnemyMosquiton, EnemyMosquitonAnimation};
use crate::{
    components::DespawnMark,
    game::score::components::Score,
    stage::{
        components::{interactive::Dead, placement::Depth},
        enemy::{
            components::behavior::EnemyCurrentBehavior,
            composed::ComposedAnimationState,
            data::{
                mosquiton::{
                    TAG_IDLE_FLY, TAG_MELEE_FLY, TAG_SHOOT_FLY, apply_mosquiton_animation_state,
                },
                steps::{EnemyStep, JumpEnemyStep},
            },
            mosquito::entity::{EnemyMosquitoAttack, EnemyMosquitoAttacking},
        },
    },
};
use bevy::prelude::*;

/// Mosquiton keeps its wing flap loop sourced from `idle_fly` while the body
/// track switches between airborne action tags.
///
/// The composed renderer resolves that request generically via part-tag
/// overrides; this system only selects semantic animation sources.
pub fn assign_mosquiton_animation(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &EnemyCurrentBehavior,
            &EnemyMosquitoAttacking,
            Option<&EnemyMosquitonAnimation>,
            &mut ComposedAnimationState,
            &Depth,
        ),
        With<EnemyMosquiton>,
    >,
) {
    for (entity, behavior, attacking, current_animation, mut animation_state, _depth) in &mut query
    {
        let (next_animation, next_tag) = match attacking.attack {
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
    query: Query<(Entity, &EnemyMosquiton), Added<Dead>>,
) {
    for (entity, mosquiton) in &query {
        commands.entity(entity).insert(DespawnMark);
        score.add_u(mosquiton.kill_score());
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
