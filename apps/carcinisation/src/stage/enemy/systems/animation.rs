use crate::stage::{
    enemy::{
        components::Enemy, mosquito::entity::EnemyMosquitoAnimation,
        tardigrade::entity::EnemyTardigradeAnimation,
    },
    messages::DepthChangedMessage,
};
use bevy::prelude::*;

/**
 * TODO there's a bug that can happen when DepthChanged is sent on a Dead entity, I suppose
 */
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
