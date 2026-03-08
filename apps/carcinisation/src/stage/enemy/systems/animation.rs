use crate::stage::{
    enemy::{
        components::Enemy, composed::ComposedEnemyVisual, mosquito::entity::EnemyMosquitoAnimation,
        mosquiton::entity::EnemyMosquitonAnimation, tardigrade::entity::EnemyTardigradeAnimation,
    },
    messages::DepthChangedMessage,
};
use bevy::prelude::*;

/// @system Re-triggers enemy animation selection when depth changes.
// TODO there's a bug that can happen when DepthChanged is sent on a Dead entity
pub fn on_enemy_depth_changed(
    mut reader: MessageReader<DepthChangedMessage>,
    mut commands: Commands,
    query: Query<(Entity, &Enemy)>,
) {
    for e in reader.read() {
        if query.get(e.entity).is_ok() {
            commands
                .entity(e.entity)
                .remove::<EnemyMosquitoAnimation>()
                .remove::<EnemyTardigradeAnimation>();
        }
    }
}

/// @system Re-triggers composed enemy animation selection when depth changes.
pub fn on_composed_enemy_depth_changed(
    mut reader: MessageReader<DepthChangedMessage>,
    mut commands: Commands,
    query: Query<(Entity, &Enemy), With<ComposedEnemyVisual>>,
) {
    for e in reader.read() {
        if query.get(e.entity).is_ok() {
            commands
                .entity(e.entity)
                .remove::<EnemyMosquitonAnimation>()
                .remove::<EnemyMosquitoAnimation>()
                .remove::<EnemyTardigradeAnimation>();
        }
    }
}
