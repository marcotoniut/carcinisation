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

pub fn setup_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    // selected_file_text: Res<SelectedFileText>,
) {
    let button_style = Style {
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        padding: UiRect::axes(Val::Px(15.0), Val::Px(7.0)),
        ..default()
    };

    let h1_text_style = TextStyle {
        font: asset_server.load(FONT_PATH),
        font_size: 16.0,
        color: COLOR_SELECT_FILE,
    };

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Start,
                ..default()
            },
            background_color: Color::NONE.into(),
            ..default()
        })
        .with_children(|p0| {
            p0.spawn((
                SelectFileButton,
                ButtonBundle {
                    style: button_style.clone(),
                    background_color: COLOR_NORMAL.into(),
                    ..default()
                },
            ))
            .with_children(|p1| {
                p1.spawn((
                    TextBundle {
                        text: Text::from_section("Select File", h1_text_style.clone()),
                        ..default()
                    },
                    Label,
                ));
            });

            p0.spawn((
                SaveButton,
                ButtonBundle {
                    style: button_style.clone(),
                    background_color: COLOR_NORMAL.into(),
                    ..default()
                },
            ))
            .with_children(|p1| {
                p1.spawn((
                    TextBundle {
                        text: Text::from_section("Save", h1_text_style.clone()),
                        ..default()
                    },
                    Label,
                ));
            });
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
    scene_path: Res<ScenePath>,
    scene_data: Res<SceneData>,
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<SaveButton>)>,
    mut write_recent_file_path_event_writer: EventWriter<WriteRecentFilePathEvent>,
) {
    for interaction in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                write_recent_file_path_event_writer.send(WriteRecentFilePathEvent);
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
            }
            _ => {}
        }
    }
}

pub fn poll_selected_file(
    mut commands: Commands,
    mut selected_files: Query<(Entity, &mut SelectedFile)>,
    asset_server: Res<AssetServer>,
    mut write_recent_file_path_event_writer: EventWriter<WriteRecentFilePathEvent>,
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
                write_recent_file_path_event_writer.send(WriteRecentFilePathEvent);
            }
            commands.entity(entity).remove::<SelectedFile>();
        }
    }
}

pub fn on_create_recent_file(
    scene_path: Res<ScenePath>,
    mut write_recent_file_path_event_reader: EventReader<WriteRecentFilePathEvent>,
) {
    for _ in write_recent_file_path_event_reader.read() {
        let path = scene_path.0.clone();
        AsyncComputeTaskPool::get()
            .spawn(async move {
                if let Ok(mut file) = File::create(RECENT_FILE_PATH) {
                    if let Err(e) = writeln!(file, "{}", path) {
                        eprintln!("Failed to write to recent file path: {:?}", e);
                    }
                } else {
                    eprintln!("Failed to create recent file path");
                }
            })
            .detach();
    }
}
