//! Gallery-specific marker components.

use bevy::prelude::*;

/// Tags every entity spawned by the gallery for bulk cleanup.
#[derive(Component, Debug)]
pub struct GalleryEntity;

/// Tags the currently displayed character entity in the gallery viewport.
#[derive(Component, Debug)]
pub struct GalleryDisplayCharacter;

/// Drives animation assignment for simple-sprite enemies in gallery mode,
/// bypassing the normal `EnemyCurrentBehavior` pipeline.
#[derive(Component, Clone, Debug)]
pub struct GalleryAnimationOverride {
    pub animation_name: String,
}
