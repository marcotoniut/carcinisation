use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use rfd::FileDialog;
use ron::extensions::Extensions;
use ron::ser::{to_string_pretty, PrettyConfig};
use std::fs::File;
use std::io::Write;

use crate::components::SceneData;
use crate::constants::assets_root;

use super::components::SelectedFile;
use super::events::WriteRecentFilePathEvent;

pub fn request_file_picker(world: &mut World) {
    let task = AsyncComputeTaskPool::get().spawn(async move {
        FileDialog::new()
            .add_filter("RON Files", &["ron"])
            .set_directory(assets_root())
            .pick_file()
    });
    world.spawn(SelectedFile(task));
}

pub fn save_scene(world: &mut World, scene_path: &str, scene_data: &SceneData) {
    let path = scene_path.to_string();
    match scene_data {
        SceneData::Cutscene(data) => {
            let data = data.clone();
            AsyncComputeTaskPool::get()
                .spawn(async move {
                    let pretty_config: PrettyConfig = PrettyConfig::new()
                        .struct_names(true)
                        .extensions(Extensions::all());
                    let ron_string = to_string_pretty(&data, pretty_config).unwrap();
                    let mut file = File::create(path).unwrap();
                    file.write_all(ron_string.as_bytes()).unwrap();
                })
                .detach();
        }
        SceneData::Stage(data) => {
            let data = data.clone();
            AsyncComputeTaskPool::get()
                .spawn(async move {
                    let pretty_config: PrettyConfig = PrettyConfig::new()
                        .struct_names(true)
                        .extensions(Extensions::all());
                    let ron_string = to_string_pretty(&data, pretty_config).unwrap();
                    let mut file = File::create(path).unwrap();
                    file.write_all(ron_string.as_bytes()).unwrap();
                })
                .detach();
        }
    }

    world.trigger(WriteRecentFilePathEvent);
}
