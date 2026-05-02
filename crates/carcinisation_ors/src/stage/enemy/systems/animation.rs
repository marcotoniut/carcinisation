use crate::stage::{
    components::interactive::{BurningCorpse, Dead},
    enemy::{
        components::Enemy, composed::ComposedEnemyVisual, mosquito::entity::EnemyMosquitoAnimation,
        mosquiton::entity::EnemyMosquitonAnimation, tardigrade::entity::EnemyTardigradeAnimation,
    },
    messages::DepthChangedMessage,
};
use bevy::prelude::*;

/// @system Re-triggers enemy animation selection when depth changes.
pub fn on_enemy_depth_changed(
    mut reader: MessageReader<DepthChangedMessage>,
    mut commands: Commands,
    query: Query<(Entity, &Enemy), (Without<Dead>, Without<BurningCorpse>)>,
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
    query: Query<
        (Entity, &Enemy),
        (
            With<ComposedEnemyVisual>,
            Without<Dead>,
            Without<BurningCorpse>,
        ),
    >,
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
