use super::entity::{EnemyMosquiton, EnemyMosquitonAnimation};
use crate::{
    components::DespawnMark,
    game::score::components::Score,
    stage::{
        components::{interactive::Dead, placement::Depth},
        enemy::{
            components::behavior::EnemyCurrentBehavior,
            composed::ComposedEnemyVisual,
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
            &mut ComposedEnemyVisual,
            &Depth,
        ),
        With<EnemyMosquiton>,
    >,
) {
    for (entity, _behavior, attacking, current_animation, mut visual, _depth) in &mut query {
        let (next_animation, next_tag) = match attacking.attack {
            Some(EnemyMosquitoAttack::Melee | EnemyMosquitoAttack::Ranged) => {
                (EnemyMosquitonAnimation::ShootStand, "shoot_stand")
            }
            None => (EnemyMosquitonAnimation::IdleStand, "idle_stand"),
        };

        if current_animation != Some(&next_animation) {
            commands.entity(entity).insert(next_animation);
        }
        if visual.requested_tag != next_tag {
            visual.requested_tag = next_tag.to_string();
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
