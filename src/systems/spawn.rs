use bevy::{
    audio::{PlaybackMode, Volume},
    prelude::*,
};

use super::audio::{AudioSystemBundle, AudioSystemType, VolumeSettings};

pub fn spawn_music(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
    music_path: String,
) {
    let sound_effect = asset_server.load(music_path);
    commands.spawn((
        AudioBundle {
            source: sound_effect,
            settings: PlaybackSettings {
                mode: PlaybackMode::Loop,
                volume: Volume::new_relative(volume_settings.1 * 1.0),
                ..default()
            },
            ..default()
        },
        AudioSystemBundle {
            system_type: AudioSystemType::MUSIC,
        },
    ));
}
