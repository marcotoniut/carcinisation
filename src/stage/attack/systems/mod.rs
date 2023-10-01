pub mod blood_shot;
pub mod player;

use bevy::prelude::*;
use seldom_pixel::{prelude::PxAssets, sprite::PxSprite};

use crate::{
    components::DespawnMark,
    plugins::movement::linear::components::{LinearTargetReached, ZAxisPosition},
    stage::{
        components::{
            interactive::{Dead, Health},
            placement::InView,
        },
        enemy::components::EnemyAttack,
        events::DepthChangedEvent,
        player::components::PLAYER_DEPTH,
        resources::StageTime,
    },
};

use super::components::blood_shot::make_blood_shot_attack_animation_bundle;

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
            Added<LinearTargetReached<StageTime, ZAxisPosition>>,
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
) {
    for event in event_reader.iter() {
        if (event.depth.0 as f32) < PLAYER_DEPTH {
            // TODO generalise via a component holding the animation data
            let (sprite_bundle, animation_bundle, collision) =
                make_blood_shot_attack_animation_bundle(&mut assets_sprite, event.depth.clone());

            commands
                .entity(event.entity)
                .insert(sprite_bundle)
                .insert(collision)
                .insert(animation_bundle);
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
