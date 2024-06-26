use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use carcinisation::CutsceneData;
use futures_lite::future;
use rfd::FileDialog;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use super::components::{SaveButton, SelectFileButton, SelectedFile};
use super::constants::RECENT_FILE_PATH;
use crate::constants::{ASSETS_PATH, FONT_PATH};
use crate::resources::CutsceneAssetHandle;
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

pub fn on_select_file(
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

pub fn on_save(
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<SaveButton>)>,
    mut commands: Commands,
) {
    for interaction in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {}
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
                let handle = asset_server.load::<CutsceneData>(path_str.clone());
                commands.insert_resource(CutsceneAssetHandle { handle });

                if let Ok(mut file) = File::create(RECENT_FILE_PATH) {
                    if let Err(e) = writeln!(file, "{}", path_str) {
                        eprintln!("Failed to write to recent file path: {:?}", e);
                    }
                } else {
                    eprintln!("Failed to create recent file path");
                }
            }
            commands.entity(entity).remove::<SelectedFile>();
        }
    }
}
