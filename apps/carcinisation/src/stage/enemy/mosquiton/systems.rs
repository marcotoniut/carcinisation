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
                mosquiton::{TAG_IDLE_FLY, TAG_MELEE_FLY, TAG_SHOOT_FLY},
                steps::{EnemyStep, JumpEnemyStep},
            },
            mosquito::entity::{EnemyMosquitoAttack, EnemyMosquitoAttacking},
        },
    },
};
use bevy::prelude::*;

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
        if animation_state.requested_tag != next_tag {
            animation_state.requested_tag = next_tag.to_string();
        }
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
