use std::time::Duration;

use bevy::{audio::Volume, prelude::*};

#[derive(Component)]
pub enum AudioSystemType {
    SFX,
    MUSIC,
}

#[derive(Bundle)]
pub struct AudioSystemBundle {
    pub system_type: AudioSystemType,
}

#[derive(Component)]
pub struct Tag(pub String);

#[derive(Component)]
pub struct Cleared;

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

// TODO could probably split into different resources
#[derive(Resource, Clone, Copy, Debug)]
pub struct VolumeSettings {
    pub master: Volume,
    pub music: Volume,
    pub sfx: Volume,
}

impl Default for VolumeSettings {
    fn default() -> Self {
        Self {
            master: Volume::new(0.8),
            music: Volume::new(0.06),
            sfx: Volume::new(0.08),
        }
    }
}

// TODO why am I using this?
pub fn remove_step<C: Component>(commands: &mut Commands, entity: Entity) {
    commands
        .entity(entity)
        .remove::<C>()
        .remove::<CutsceneElapsedStarted>();
}
