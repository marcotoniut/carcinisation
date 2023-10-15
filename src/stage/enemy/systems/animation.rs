use bevy::prelude::*;
use seldom_pixel::{prelude::PxAssets, sprite::PxSprite};

use crate::stage::{
    enemy::components::{Enemy, EnemyMosquitoAnimation, EnemyTardigradeAnimation},
    events::DepthChangedEvent,
};

/**
 * TODO there's a bug that can happen when DepthChanged is sent on a Dead entity, I suppose
 */
pub fn read_enemy_depth_changed(
    mut commands: Commands,
    mut event_reader: EventReader<DepthChangedEvent>,
    query: Query<(Entity, &Enemy)>,
) {
    for event in event_reader.iter() {
        for (entity, attack_type) in &query {
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
