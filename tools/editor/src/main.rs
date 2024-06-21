mod events;
mod inspector;
mod resources;
mod systems;

use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy_common_assets::ron::RonAssetPlugin;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use carcinisation::cutscene::data::CutsceneData;
use events::{CutsceneLoadedEvent, CutsceneUnloadedEvent};
use futures_lite::future;
use inspector::InspectorPlugin;
use resources::CutsceneAssetHandle;
use rfd::FileDialog;
use std::path::PathBuf;
use systems::check_cutscene_data_loaded;

const COLOR_PRESSED: Color = Color::rgb(0.25, 0.25, 0.25);
const COLOR_HOVERED: Color = Color::rgb(0.35, 0.35, 0.35);
const COLOR_NORMAL: Color = Color::rgb(0.15, 0.15, 0.15);
const COLOR_SELECT_FILE: Color = Color::rgb(0.9, 0.9, 0.9);

const ASSETS_PATH: &str = "../../assets/";

#[derive(Component)]
struct SelectedFile(Task<Option<PathBuf>>);

fn main() {
    let title: String = "SCENE EDITOR".to_string();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: ASSETS_PATH.into(),
                    ..Default::default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title,
                        resizable: true,
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
        )
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(RonAssetPlugin::<CutsceneData>::new(&["cs.ron"]))
        .add_plugins(EguiPlugin)
        .add_plugins(InspectorPlugin)
        .add_event::<CutsceneLoadedEvent>()
        .add_event::<CutsceneUnloadedEvent>()
        .add_systems(Startup, setup_ui)
        .add_systems(Update, (file_selector_system, poll_selected_file))
        .add_systems(
            Update,
            (check_cutscene_data_loaded.run_if(resource_exists::<CutsceneAssetHandle>),),
        )
        .run();
}

fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Start,
                ..Default::default()
            },
            background_color: Color::NONE.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn(ButtonBundle {
                    style: Style {
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(15.0), Val::Px(7.0)),
                        ..Default::default()
                    },
                    background_color: COLOR_NORMAL.into(),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        TextBundle {
                            text: Text::from_section(
                                "Select File",
                                TextStyle {
                                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                    font_size: 16.0,
                                    color: COLOR_SELECT_FILE,
                                },
                            ),
                            ..Default::default()
                        },
                        Label,
                    ));
                });
        });
}

fn file_selector_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut commands: Commands,
) {
    for (interaction, mut background_color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *background_color = COLOR_PRESSED.into();
                let thread_pool = AsyncComputeTaskPool::get();
                let task = thread_pool.spawn(async move {
                    FileDialog::new()
                        .add_filter("RON Files", &["ron"])
                        .set_directory(ASSETS_PATH.to_string())
                        .pick_file()
                });
                commands.spawn(SelectedFile(task));
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

fn poll_selected_file(
    mut commands: Commands,
    mut selected_files: Query<(Entity, &mut SelectedFile)>,
    asset_server: Res<AssetServer>,
) {
    for (entity, mut selected_file) in selected_files.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut selected_file.0)) {
            if let Some(path) = result {
                println!("Selected file: {:?}", path);
                let handle = asset_server.load::<CutsceneData>(path);
                commands.insert_resource(CutsceneAssetHandle { handle });
            }
            commands.entity(entity).remove::<SelectedFile>();
        }
    }
}
