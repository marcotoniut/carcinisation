use std::time::Duration;

use crate::inspector::utils::StageSpawnUtils;
use bevy::prelude::*;
use bevy::render::color::Color;
use bevy::sprite::Anchor;
use carcinisation::stage::data::{StageData, StageStep};

use crate::components::{Draggable, SceneItem, StageSpawnLabel};
use crate::constants::FONT_PATH;
use crate::resources::StageElapsedUI;

pub fn spawn_stage(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    stage_elapsed_ui: &Res<StageElapsedUI>,
    stage_data: &StageData,
) {
    let background_texture = asset_server.load(&stage_data.background_path);

    // Spawn background
    commands.spawn((
        Name::new("Stage Background"),
        SceneItem,
        SpriteBundle {
            texture: background_texture,
            transform: Transform::from_xyz(0.0, 0.0, -10.0),
            sprite: Sprite {
                anchor: Anchor::BottomLeft,

                ..default()
            },

            ..default()
        },
    ));

    let label_style = TextStyle {
        font: asset_server.load(FONT_PATH),
        font_size: 12.0,
        color: Color::WHITE,
    };

    for (index, spawn) in stage_data.spawns.iter().enumerate() {
        if spawn.get_elapsed() <= stage_elapsed_ui.0 {
            let thumbnail = spawn.get_thumbnail();
            commands.spawn((
                spawn.get_editor_name_component(index),
                StageSpawnLabel,
                Draggable,
                SceneItem,
                SpriteBundle {
                    texture: asset_server.load(&thumbnail.0),
                    transform: Transform::from_translation(
                        spawn
                            .get_coordinates()
                            .extend(spawn.get_depth_editor_z_index()),
                    ),
                    sprite: Sprite {
                        anchor: Anchor::BottomCenter,
                        rect: thumbnail.1,
                        ..default()
                    },
                    ..default()
                },
            ));
        }
    }

    let mut current_position = stage_data.start_coordinates.unwrap_or(Vec2::ZERO);
    let mut current_elapsed: Duration = Duration::ZERO;
    for (index, step) in stage_data.steps.iter().enumerate() {
        match step {
            StageStep::Cinematic(s) => {
                // TODO
            }
            StageStep::Movement(s) => {
                for spawn in s.spawns.iter() {
                    current_elapsed += spawn.get_elapsed();
                    let show = current_elapsed <= stage_elapsed_ui.0;
                    if show {
                        let v = current_position + *spawn.get_coordinates();
                        let thumbnail = spawn.get_thumbnail();
                        commands.spawn((
                            spawn.get_editor_name_component(index),
                            StageSpawnLabel,
                            Draggable,
                            SceneItem,
                            SpriteBundle {
                                texture: asset_server.load(&thumbnail.0),
                                transform: Transform::from_translation(
                                    v.extend(spawn.get_depth_editor_z_index()),
                                ),
                                sprite: Sprite {
                                    anchor: Anchor::BottomCenter,
                                    rect: thumbnail.1,
                                    ..default()
                                },
                                ..default()
                            },
                        ));
                    }
                }

                let distance = s.coordinates.distance(current_position);
                let time_to_move = distance / s.base_speed;
                current_position = s.coordinates;
                current_elapsed += Duration::from_secs_f32(time_to_move);
            }
            StageStep::Stop(s) => {
                current_elapsed += s.max_duration.unwrap_or(Duration::ZERO);
            }
        }
    }

    commands.spawn((
        Name::new("Stage Info"),
        SceneItem,
        Text2dBundle {
            text: Text::from_sections([
                TextSection::new("Stage: ", label_style.clone()),
                TextSection::new(&stage_data.name, label_style.clone()),
                TextSection::new("\nMusic: ", label_style.clone()),
                TextSection::new(&stage_data.music_path, label_style.clone()),
            ]),
            transform: Transform::from_xyz(-400.0, 300.0, 10.0),
            ..default()
        },
    ));
}
