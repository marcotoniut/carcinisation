use bevy::{audio::PlaybackMode, prelude::*};

use super::audio::{AudioSystemBundle, AudioSystemType};

pub fn spawn_music(commands: &mut Commands, asset_server: &Res<AssetServer>, music_path: String) {
    let sound_effect = asset_server.load(music_path);
    commands.spawn((
        AudioBundle {
            source: sound_effect,
            settings: PlaybackSettings {
                mode: PlaybackMode::Loop,
                ..default()
            },
            ..default()
        },
        AudioSystemBundle {
            system_type: AudioSystemType::MUSIC,
        },
    ));
}
