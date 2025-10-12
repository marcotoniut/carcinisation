use crate::stage::{
    enemy::{
        components::Enemy, mosquito::entity::EnemyMosquitoAnimation,
        tardigrade::entity::EnemyTardigradeAnimation,
    },
    events::DepthChangedEvent,
};
use bevy::prelude::*;

/**
 * TODO there's a bug that can happen when DepthChanged is sent on a Dead entity, I suppose
 */
pub fn on_enemy_depth_changed(
    mut reader: EventReader<DepthChangedEvent>,
    mut commands: Commands,
    query: Query<(Entity, &Enemy)>,
) {
    for e in reader.read() {
        if let Ok(_) = query.get(e.entity) {
            commands
                .entity(e.entity)
                .remove::<EnemyMosquitoAnimation>()
                .remove::<EnemyTardigradeAnimation>();
        }
    }
}
