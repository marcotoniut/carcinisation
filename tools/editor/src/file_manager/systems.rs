use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use carcinisation::stage::data::StageData;
use carcinisation::CutsceneData;
use futures_lite::future;
use rfd::FileDialog;
use ron::extensions::Extensions;
use ron::ser::{to_string_pretty, PrettyConfig};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use super::components::{SaveButton, SelectFileButton, SelectedFile};
use super::constants::RECENT_FILE_PATH;
use super::events::WriteRecentFilePathEvent;
use crate::components::{SceneData, ScenePath};
use crate::constants::{ASSETS_PATH, FONT_PATH};
use crate::resources::{CutsceneAssetHandle, StageAssetHandle};
use crate::ui::styles::*;

pub fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_handle = asset_server.load(FONT_PATH);

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Start,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    SelectFileButton,
                    Button,
                    Node {
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(15.0), Val::Px(7.0)),
                        ..default()
                    },
                    BackgroundColor(COLOR_NORMAL),
                ))
                .with_child((
                    Text::new("Select File"),
                    TextFont {
                        font: font_handle.clone().into(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(COLOR_SELECT_FILE.into()),
                    Label,
                ));

            parent
                .spawn((
                    SaveButton,
                    Button,
                    Node {
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(15.0), Val::Px(7.0)),
                        ..default()
                    },
                    BackgroundColor(COLOR_NORMAL),
                ))
                .with_child((
                    Text::new("Save"),
                    TextFont {
                        font: font_handle.into(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(COLOR_SELECT_FILE.into()),
                    Label,
                ));
        });
}

pub fn load_recent_file(mut commands: Commands) {
    let thread_pool = AsyncComputeTaskPool::get();
    let task = thread_pool.spawn(async move {
        if let Ok(mut file) = File::open(RECENT_FILE_PATH) {
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

pub fn on_button_interaction(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut background_color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *background_color = COLOR_PRESSED.into();
            }
            Interaction::Hovered => {
                *background_color = COLOR_HOVERED.into();
            }
            Interaction::None => {
                *background_color = COLOR_NORMAL.into();
            }
        }
    }
}

pub fn on_select_file_button_pressed(
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<SelectFileButton>)>,
    mut commands: Commands,
) {
    for interaction in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                let thread_pool = AsyncComputeTaskPool::get();
                let task = thread_pool.spawn(async move {
                    FileDialog::new()
                        .add_filter("RON Files", &["ron"])
                        .set_directory(ASSETS_PATH.to_string())
                        .pick_file()
                });
                commands.spawn(SelectedFile(task));
            }
            _ => {}
        }
    }
}

pub fn on_save_button_pressed(
    mut commands: Commands,
    scene_path: Res<ScenePath>,
    scene_data: Res<SceneData>,
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<SaveButton>)>,
) {
    for interaction in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                match scene_data.to_owned() {
                    SceneData::Cutscene(data) => {
                        let path = scene_path.0.clone();
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
                        let path = scene_path.0.clone();
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
                commands.trigger(WriteRecentFilePathEvent);
            }
            _ => {}
        }
    }
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
                let path_str = path.to_str().unwrap().to_string();

                if path_str.ends_with(".cs.ron") {
                    let handle = asset_server.load::<CutsceneData>(path_str.clone());
                    commands.insert_resource(CutsceneAssetHandle {
                        handle,
                        path: path_str,
                    });
                } else if path_str.ends_with(".sg.ron") {
                    let handle = asset_server.load::<StageData>(path_str.clone());
                    commands.insert_resource(StageAssetHandle {
                        handle,
                        path: path_str,
                    });
                } else {
                    eprintln!("Unsupported file type: {:?}", path_str);
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
            match File::create(RECENT_FILE_PATH) {
                Ok(mut file) => {
                    println!("{}", path);
                    if let Err(e) = writeln!(file, "{}", path) {
                        eprintln!("Failed to write to recent file path: {:?}", e);
                    }
                    file.flush();
                }
                Err(e) => {
                    eprintln!("Failed to create recent file path: {:?}", e);
                }
            }
        })
        .detach();
}
