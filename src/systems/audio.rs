use bevy::prelude::*;

#[derive(Component)]
pub enum AudioSystemType {
    SFX,
    MUSIC,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct VolumeSettings(
    ///Master Volume
    pub f32,
    ///Music Volume
    pub f32,
    ///SFX Volume
    pub f32,
);

#[derive(Bundle)]
pub struct AudioSystemBundle {
    pub system_type: AudioSystemType,
}
