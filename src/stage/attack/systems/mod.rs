pub mod hovering;
pub mod player;

use super::components::{
    bundles::make_hovering_attack_animation_bundle, EnemyAttack, EnemyHoveringAttackType,
};
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
use bevy::prelude::*;
use seldom_pixel::{prelude::PxAssets, sprite::PxSprite};

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
    for event in event_reader.read() {
        if event.depth > PLAYER_DEPTH {
            for (entity, attack_type) in &query {
                if entity == event.entity {
                    let (sprite_bundle, animation_bundle, collision_data) =
                        make_hovering_attack_animation_bundle(
                            &mut assets_sprite,
                            attack_type,
                            event.depth.clone(),
                        );

                    // TODO could probably unify the use of this with the ones under spawns
                    let mut entity_commands = commands.entity(event.entity);

                    entity_commands.insert((sprite_bundle, animation_bundle));
                    if !collision_data.0.is_empty() {
                        entity_commands.insert(collision_data);
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
