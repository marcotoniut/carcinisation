use std::time::Duration;

use crate::inspector::utils::{StageDataUtils, StageSpawnUtils};
use bevy::prelude::*;
use bevy::render::color::Color;
use bevy::sprite::Anchor;
use carcinisation::globals::SCREEN_RESOLUTION;
use carcinisation::stage::data::{StageData, StageStep};

use crate::components::{AnimationIndices, AnimationTimer, Draggable, SceneItem, StageSpawnLabel};
use crate::constants::FONT_PATH;
use crate::resources::StageControlsUI;

pub fn spawn_stage(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    stage_controls_ui: &Res<StageControlsUI>,
    stage_data: &StageData,
    texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
) {
    if stage_controls_ui.background_is_visible() {
        let texture = asset_server.load(&stage_data.background_path);

        commands.spawn((
            Name::new("SG Background"),
            SceneItem,
            SpriteBundle {
                texture,
                transform: Transform::from_xyz(0.0, 0.0, -10.0),
                sprite: Sprite {
                    anchor: Anchor::BottomLeft,
                    ..default()
                },
                ..default()
            },
        ));
    }

    // TODO Make skybox follow camera via elapsed
    // TODO Animate skybox
    if stage_controls_ui.skybox_is_visible() {
        let texture = asset_server.load(&stage_data.skybox.path);
        let texture_atlas_layout =
            TextureAtlasLayout::from_grid(SCREEN_RESOLUTION.as_vec2(), 1, 2, None, None);
        let layout = texture_atlas_layouts.add(texture_atlas_layout);

        commands.spawn((
            Name::new("SG Skybox"),
            SceneItem,
            SpriteBundle {
                sprite: Sprite {
                    anchor: Anchor::BottomLeft,
                    ..default()
                },
                texture,
                transform: Transform::from_xyz(0.0, 0.0, -11.0),
                ..default()
            },
            TextureAtlas { layout, index: 0 },
            AnimationIndices { first: 0, last: 1 },
            AnimationTimer(Timer::from_seconds(2.0, TimerMode::Repeating)),
        ));
    }

    let label_style = TextStyle {
        font: asset_server.load(FONT_PATH),
        font_size: 12.0,
        color: Color::WHITE,
    };

    for (index, spawn) in stage_data
        .spawns
        .iter()
        .filter(|x| stage_controls_ui.depth_is_visible(x.get_depth()))
        .enumerate()
    {
        if spawn.get_elapsed() <= stage_controls_ui.ElapsedDuration {
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
                for spawn in s
                    .spawns
                    .iter()
                    .filter(|x| stage_controls_ui.depth_is_visible(x.get_depth()))
                {
                    current_elapsed += spawn.get_elapsed();
                    let show = current_elapsed <= stage_controls_ui.ElapsedDuration;
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

                // TODO elapsed?
                for spawn in s
                    .spawns
                    .iter()
                    .filter(|x| stage_controls_ui.depth_is_visible(x.get_depth()))
                {
                    current_elapsed += spawn.get_elapsed();
                    let show = current_elapsed <= stage_controls_ui.ElapsedDuration;
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
            }
        }
    }

    commands.spawn((
        Name::new("SG Info"),
        SceneItem,
        Text2dBundle {
            text: Text::from_sections([
                TextSection::new("Stage: ", label_style.clone()),
                TextSection::new(&stage_data.name, label_style.clone()),
                TextSection::new("\nMusic: ", label_style.clone()),
                TextSection::new(&stage_data.music_path, label_style.clone()),
                TextSection::new("\nStart Coordinates: ", label_style.clone()),
                TextSection::new(
                    &stage_data
                        .start_coordinates
                        .unwrap_or_else(|| Vec2::ZERO)
                        .to_string(),
                    label_style.clone(),
                ),
                TextSection::new("\nSteps: ", label_style.clone()),
                TextSection::new(&stage_data.steps.len().to_string(), label_style.clone()),
                TextSection::new("\nStatic Spawns: ", label_style.clone()),
                TextSection::new(&stage_data.spawns.len().to_string(), label_style.clone()),
                TextSection::new("\nDynamic Spawns: ", label_style.clone()),
                TextSection::new(
                    &stage_data.dynamic_spawn_count().to_string(),
                    label_style.clone(),
                ),
            ]),
            transform: Transform::from_xyz(0.0, -15.0, 0.0),
            text_anchor: Anchor::TopLeft,
            ..default()
        },
    ));
}
