use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use rfd::FileDialog;
use ron::extensions::Extensions;
use ron::ser::{to_string_pretty, PrettyConfig};
use serde::Serialize;
use std::fs::File;
use std::io::Write;

use crate::components::SceneData;
use crate::constants::assets_root;

use super::components::SelectedFile;
use super::events::WriteRecentFilePathEvent;

/// Opens a native file picker and stores the async task.
pub fn request_file_picker(world: &mut World) {
    let task = AsyncComputeTaskPool::get().spawn(async move {
        FileDialog::new()
            .add_filter("RON Files", &["ron"])
            .set_directory(assets_root())
            .pick_file()
    });
    world.spawn(SelectedFile(task));
}

/// Saves the current scene data to disk and updates the recent file path.
pub fn save_scene(world: &mut World, scene_path: &str, scene_data: &SceneData) {
    let path = scene_path.to_string();
    match scene_data {
        SceneData::Cutscene(data) => save_ron(data.clone(), path),
        SceneData::Stage(data) => save_ron(data.clone(), path),
    }

    world.trigger(WriteRecentFilePathEvent);
}

fn save_ron<T: Serialize + Send + 'static>(data: T, path: String) {
    AsyncComputeTaskPool::get()
        .spawn(async move {
            let pretty_config: PrettyConfig = PrettyConfig::new()
                .struct_names(true)
                .extensions(Extensions::all());
            match to_string_pretty(&data, pretty_config) {
                Ok(ron_string) => match File::create(&path) {
                    Ok(mut file) => {
                        if let Err(error) = file.write_all(ron_string.as_bytes()) {
                            eprintln!("Failed to write scene data to {}: {:?}", path, error);
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to create scene file at {}: {:?}", path, error);
                    }
                },
                Err(error) => {
                    eprintln!("Failed to serialize scene data to RON: {:?}", error);
                }
            }
        })
        .detach();
}
