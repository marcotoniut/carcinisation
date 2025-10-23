//! Small Bevy helpers shared across plugins.

use bevy::prelude::*;

/// @system Recursively despawns all entities carrying component `T`.
pub fn despawn_entities<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
