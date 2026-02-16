pub mod hovering;
pub mod player;

use super::components::{
    EnemyAttack, EnemyHoveringAttackType, bundles::make_hovering_attack_animation_bundle,
};
use crate::pixel::PxAssets;
use crate::{
    components::DespawnMark,
    stage::{
        components::{
            interactive::{Dead, Health},
            placement::InView,
        },
        messages::DepthChangedMessage,
        player::components::PLAYER_DEPTH,
        resources::StageTimeDomain,
    },
};
use bevy::prelude::*;
use cween::linear::components::{LinearValueReached, TargetingValueZ};
use seldom_pixel::prelude::PxSprite;

/// @system Marks entities as `Dead` when their health reaches zero.
// TODO remove in favor of damage taken?
pub fn check_health_at_0(mut commands: Commands, query: Query<(Entity, &Health), Without<Dead>>) {
    for (entity, health) in &mut query.iter() {
        if health.0 == 0 {
            commands.entity(entity).insert(Dead);
        }
    }
}

/// @system Despawns enemy attacks that reached their target depth while off-screen.
pub fn miss_on_reached(
    mut commands: Commands,
    query: Query<
        Entity,
        (
            Added<LinearValueReached<StageTimeDomain, TargetingValueZ>>,
            With<EnemyAttack>,
            Without<InView>,
        ),
    >,
) {
    for entity in &mut query.iter() {
        commands.entity(entity).insert(DespawnMark);
    }
}

/// @system Updates the hovering-attack sprite when its depth layer changes.
// TODO there's a bug that can happen when DepthChanged is sent on a Dead entity
pub fn on_enemy_attack_depth_changed(
    mut commands: Commands,
    // TODO do I need an EventReader for this? Can't I just use a query that checks for Changed<Depth>?
    mut event_reader: MessageReader<DepthChangedMessage>,
    mut assets_sprite: PxAssets<PxSprite>,
    query: Query<(Entity, &EnemyHoveringAttackType)>,
) {
    for event in event_reader.read() {
        if event.depth > PLAYER_DEPTH {
            for (entity, attack_type) in &query {
                if entity == event.entity {
                    let (sprite_bundle, animation_bundle, collider_data) =
                        make_hovering_attack_animation_bundle(
                            &mut assets_sprite,
                            attack_type,
                            event.depth,
                        );

                    // TODO could probably unify the use of this with the ones under spawns
                    let mut entity_commands = commands.entity(event.entity);

                    entity_commands.insert((sprite_bundle, animation_bundle));
                    if !collider_data.0.is_empty() {
                        entity_commands.insert(collider_data);
                    }

                    break;
                }
            }
        }
    }
}

/// @system Despawns enemy attacks that have been marked dead.
pub fn despawn_dead_attacks(
    mut commands: Commands,
    query: Query<Entity, (Added<Dead>, With<EnemyAttack>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(DespawnMark);
    }
}
