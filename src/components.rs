use std::time::Duration;

use bevy::prelude::*;

#[derive(Component)]
pub struct Tag(pub String);

#[derive(Component)]
pub struct CutsceneElapsedStarted(pub Duration);

#[derive(Component)]
pub struct DespawnMark;

#[derive(Component)]
pub struct Music;

#[derive(Component)]
pub struct DelayedDespawnOnPxAnimationFinished(pub Duration);

impl DelayedDespawnOnPxAnimationFinished {
    pub fn from_secs_f32(secs: f32) -> Self {
        Self(Duration::from_secs_f32(secs))
    }
}

#[derive(Component)]
pub struct DespawnAfterDelay {
    pub elapsed: Duration,
    pub duration: Duration,
}

pub fn despawn_step<C: Component>(commands: &mut Commands, entity: Entity) {
    commands
        .entity(entity)
        .remove::<C>()
        .remove::<CutsceneElapsedStarted>();
}
