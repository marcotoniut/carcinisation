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
pub fn read_enemy_depth_changed(
    mut commands: Commands,
    mut event_reader: EventReader<DepthChangedEvent>,
    query: Query<(Entity, &Enemy)>,
) {
    for event in event_reader.read() {
        for (entity, _) in &query {
            if entity == event.entity {
                // TODO hacked with hardcoded enemy references
                commands
                    .entity(entity)
                    .remove::<EnemyMosquitoAnimation>()
                    .remove::<EnemyTardigradeAnimation>();

                break;
            }
        }
    }
}
