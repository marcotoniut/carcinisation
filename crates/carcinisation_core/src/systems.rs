//! Shared utility systems for Carcinisation.

use bevy::prelude::*;

/// Recursively despawns all entities carrying component `T`.
pub fn despawn_entities<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in query.iter() {
        commands.entity(entity).try_despawn();
    }
}
