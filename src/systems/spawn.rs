use crate::components::{AudioSystemBundle, AudioSystemType, Music, VolumeSettings};
use bevy::{audio::PlaybackMode, prelude::*};

pub fn make_music_bundle(
    asset_server: &Res<AssetServer>,
    volume_settings: &Res<VolumeSettings>,
    music_path: String,
    mode: PlaybackMode,
) -> (
    AudioPlayer<AudioSource>,
    PlaybackSettings,
    AudioSystemBundle,
    Music,
) {
    let source = asset_server.load(music_path);
    (
        AudioPlayer::new(source),
        PlaybackSettings {
            mode,
            volume: volume_settings.music.clone(),
            ..default()
        },
        AudioSystemBundle {
            system_type: AudioSystemType::MUSIC,
        },
        Music,
    )
}
