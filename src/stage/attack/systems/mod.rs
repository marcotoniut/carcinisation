pub mod hovering;
pub mod player;

use bevy::prelude::*;
use seldom_pixel::{prelude::PxAssets, sprite::PxSprite};

use crate::{
    components::DespawnMark,
    plugins::movement::linear::components::{LinearTargetReached, TargetingPositionZ},
    stage::{
        components::{
            interactive::{Dead, Health},
            placement::InView,
        },
        events::DepthChangedEvent,
        player::components::PLAYER_DEPTH,
        resources::StageTime,
    },
};

use super::components::{
    bundles::make_hovering_attack_animation_bundle, EnemyAttack, EnemyHoveringAttackType,
};

// TODO remove in favor of damage taken?
pub fn check_health_at_0(mut commands: Commands, query: Query<(Entity, &Health), Without<Dead>>) {
    for (entity, health) in &mut query.iter() {
        if health.0 == 0 {
            commands.entity(entity).insert(Dead);
        }
    }
}

pub fn miss_on_reached(
    mut commands: Commands,
    query: Query<
        Entity,
        (
            Added<LinearTargetReached<StageTime, TargetingPositionZ>>,
            With<EnemyAttack>,
            Without<InView>,
        ),
    >,
) {
    for entity in &mut query.iter() {
        commands.entity(entity).insert(DespawnMark);
    }
}

/**
 * TODO there's a bug that can happen when DepthChanged is sent on a Dead entity, I suppose
 */
pub fn read_enemy_attack_depth_changed(
    mut commands: Commands,
    mut event_reader: EventReader<DepthChangedEvent>,
    mut assets_sprite: PxAssets<PxSprite>,
    query: Query<(Entity, &EnemyHoveringAttackType)>,
) {
    for event in event_reader.iter() {
        if (event.depth.0 as f32) < PLAYER_DEPTH {
            for (entity, attack_type) in &query {
                if entity == event.entity {
                    let (sprite_bundle, animation_bundle, collision_o) =
                        make_hovering_attack_animation_bundle(
                            &mut assets_sprite,
                            attack_type,
                            event.depth.clone(),
                        );

                    // TODO could probably unify the use of this with the ones under spawns
                    let mut entity_commands = commands.entity(event.entity);

                    entity_commands.insert((sprite_bundle, animation_bundle));
                    if let Some(collision) = collision_o {
                        entity_commands.insert(collision);
                    }

                    break;
                }
            }
        }
    }
}

pub fn despawn_dead_attacks(
    mut commands: Commands,
    query: Query<(Entity, &EnemyAttack), Added<Dead>>,
) {
    for (entity, _) in query.iter() {
        commands.entity(entity).insert(DespawnMark);
    }
}
