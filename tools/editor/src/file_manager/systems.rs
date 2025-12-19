use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use carcinisation::stage::data::StageData;
use carcinisation::CutsceneData;
use futures_lite::future;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use super::components::SelectedFile;
use super::constants::recent_file_path;
use super::events::WriteRecentFilePathEvent;
use crate::components::ScenePath;
use crate::constants::assets_root;
use crate::resources::{CutsceneAssetHandle, StageAssetHandle};

pub fn load_recent_file(mut commands: Commands) {
    let thread_pool = AsyncComputeTaskPool::get();
    let task = thread_pool.spawn(async move {
        let recent_path = recent_file_path();
        if let Ok(mut file) = File::open(&recent_path) {
            let mut path = String::new();
            if file.read_to_string(&mut path).is_ok() && !path.trim().is_empty() {
                let path_buf = PathBuf::from(path.trim());
                println!("Loading recent file: {:?}", path_buf);
                Some(path_buf)
            } else {
                None
            }
        } else {
            None
        }
    });
    commands.spawn(SelectedFile(task));
}

fn asset_relative_path(path: &Path) -> Option<String> {
    let assets_root = assets_root();
    let assets_root = assets_root.canonicalize().unwrap_or(assets_root);
    let path = path.canonicalize().ok()?;
    let relative = path.strip_prefix(&assets_root).ok()?;
    Some(relative.to_string_lossy().replace('\\', "/"))
}

pub fn poll_selected_file(
    mut commands: Commands,
    mut selected_files: Query<(Entity, &mut SelectedFile)>,
    asset_server: Res<AssetServer>,
) {
    for (entity, mut selected_file) in selected_files.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut selected_file.0)) {
            if let Some(path) = result {
                println!("Selected file: {:?}", path);
                let file_path = path.to_string_lossy().to_string();
                let asset_path = asset_relative_path(&path);

                if file_path.ends_with(".cs.ron") {
                    if let Some(asset_path) = asset_path {
                        let handle = asset_server.load::<CutsceneData>(asset_path);
                        commands.insert_resource(CutsceneAssetHandle {
                            handle,
                            path: file_path,
                        });
                    } else {
                        eprintln!("Selected file is outside the assets root: {:?}", file_path);
                    }
                } else if file_path.ends_with(".sg.ron") {
                    if let Some(asset_path) = asset_path {
                        let handle = asset_server.load::<StageData>(asset_path);
                        commands.insert_resource(StageAssetHandle {
                            handle,
                            path: file_path,
                        });
                    } else {
                        eprintln!("Selected file is outside the assets root: {:?}", file_path);
                    }
                } else {
                    eprintln!("Unsupported file type: {:?}", file_path);
                };
            }
            commands.entity(entity).remove::<SelectedFile>();
        }
    }
}

pub fn on_write_recent_file_path(
    _trigger: On<WriteRecentFilePathEvent>,
    scene_path: Res<ScenePath>,
) {
    let path = scene_path.0.clone();
    AsyncComputeTaskPool::get()
        .spawn(async move {
            match File::create(recent_file_path()) {
                Ok(mut file) => {
                    println!("{}", path);
                    if let Err(e) = writeln!(file, "{}", path) {
                        eprintln!("Failed to write to recent file path: {:?}", e);
                    }
                    let _ = file.flush();
                }
                Err(e) => {
                    eprintln!("Failed to create recent file path: {:?}", e);
                }
            }
        })
        .detach();
}
