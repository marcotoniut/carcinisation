use super::audio::{AudioSystemBundle, AudioSystemType, VolumeSettings};
use crate::components::Music;
use bevy::{
    audio::{PlaybackMode, Volume},
    prelude::*,
};

pub fn make_music_bundle(
    asset_server: &Res<AssetServer>,
    volume_settings: &Res<VolumeSettings>,
    music_path: String,
    mode: PlaybackMode,
) -> (AudioBundle, AudioSystemBundle, Music) {
    let source = asset_server.load(music_path);
    (
        AudioBundle {
            source,
            settings: PlaybackSettings {
                mode,
                volume: Volume::new(volume_settings.1 * 1.0),
                ..Default::default()
            },
            ..Default::default()
        },
        AudioSystemBundle {
            system_type: AudioSystemType::MUSIC,
        },
        Music,
    )
}
