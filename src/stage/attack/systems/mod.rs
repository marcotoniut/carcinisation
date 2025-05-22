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
use seldom_pixel::prelude::PxSprite;

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
pub fn on_enemy_attack_depth_changed(
    mut commands: Commands,
    // TODO do I need an EventReader for this? Can't I just use a query that checks for Changed<Depth>?
    mut event_reader: EventReader<DepthChangedEvent>,
    asset_server: Res<AssetServer>,
    query: Query<(Entity, &EnemyHoveringAttackType)>,
) {
    for event in event_reader.read() {
        if event.depth > PLAYER_DEPTH {
            for (entity, attack_type) in &query {
                if entity == event.entity {
                    let (sprite_bundle, animation_bundle, collider_data) =
                        make_hovering_attack_animation_bundle(
                            &asset_server,
                            attack_type,
                            event.depth.clone(),
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

pub fn despawn_dead_attacks(
    mut commands: Commands,
    query: Query<Entity, (Added<Dead>, With<EnemyAttack>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(DespawnMark);
    }
}
