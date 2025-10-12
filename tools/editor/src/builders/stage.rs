use std::time::Duration;

use crate::constants::EditorColor;
use crate::inspector::utils::{StageDataUtils, StageSpawnUtils};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy_prototype_lyon::draw::{Fill, Stroke};
use bevy_prototype_lyon::entity::ShapeBundle;
use bevy_prototype_lyon::geometry::GeometryBuilder;
use bevy_prototype_lyon::path::PathBuilder;
use bevy_prototype_lyon::shapes::Polygon;
use carcinisation::globals::SCREEN_RESOLUTION;
use carcinisation::stage::data::{StageData, StageStep};

use crate::components::{AnimationIndices, AnimationTimer, Draggable, SceneItem, StageSpawnLabel};
use crate::constants::FONT_PATH;
use crate::resources::StageControlsUI;

const SKYBOX_Z: f32 = -11.0;
const BACKGROUND_Z: f32 = -10.0;
const CAMERA_POSITION_Z: f32 = 9.9;
const PATH_Z: f32 = 10.0;

pub fn spawn_path(
    commands: &mut Commands,
    stage_data: &StageData,
    stage_controls_ui: &Res<StageControlsUI>,
) {
    let screen_resolution = SCREEN_RESOLUTION.as_vec2();
    let h_screen_resolution = screen_resolution / 2.0;

    let camera_position = stage_data.calculate_camera_position(stage_controls_ui.ElapsedDuration);
    let camera_shape = Polygon {
        points: vec![
            Vec2::ZERO,
            Vec2::new(screen_resolution.x, 0.0),
            screen_resolution,
            Vec2::new(0.0, screen_resolution.y),
        ],
        closed: true,
    };

    commands.spawn((
        Name::new("Camera Position"),
        SceneItem,
        ShapeBundle {
            path: GeometryBuilder::build_as(&camera_shape),
            transform: Transform {
                translation: camera_position.extend(CAMERA_POSITION_Z),
                ..default()
            },
            ..default()
        },
        Stroke::color(Color::WHITE),
    ));

    let mut path_builder = PathBuilder::new();
    path_builder.move_to(stage_data.start_coordinates.unwrap_or(Vec2::ZERO) + h_screen_resolution);

    let mut current_position = stage_data.start_coordinates.unwrap_or(Vec2::ZERO);
    let mut current_elapsed: Duration = Duration::ZERO;

    for (index, step) in stage_data.steps.iter().enumerate() {
        match step {
            StageStep::Cinematic(s) => {
                // TODO
            }
            StageStep::Movement(s) => {
                path_builder.line_to(s.coordinates + h_screen_resolution);

                let direction = (current_position - s.coordinates).normalize_or_zero();
                let angle = direction.y.atan2(direction.x);

                let arrow_shape = Polygon {
                    points: vec![
                        Vec2::new(0.0, 0.0),
                        Vec2::new(6.0, -3.0),
                        Vec2::new(6.0, 3.0),
                    ],
                    closed: true,
                };
                commands.spawn((
                    Name::new(format!("Elapsed Path Movement Arrow {}", index)),
                    SceneItem,
                    ShapeBundle {
                        path: GeometryBuilder::build_as(&arrow_shape),
                        transform: Transform {
                            translation: (current_position + h_screen_resolution).extend(PATH_Z),
                            rotation: Quat::from_rotation_z(angle),
                            ..default()
                        },
                        ..default()
                    },
                    Fill::color(Color::CYAN),
                ));

                let distance = s.coordinates.distance(current_position);
                let time_to_move = distance / s.base_speed;
                current_position = s.coordinates;
                current_elapsed += Duration::from_secs_f32(time_to_move);
            }
            StageStep::Stop(s) => {
                current_elapsed += s.max_duration.unwrap_or(Duration::ZERO);

                // TODO elapsed?
                for spawn in s.spawns.iter() {
                    current_elapsed += spawn.get_elapsed();
                }
            }
        }
    }

    commands.spawn((
        Name::new("Elapsed Path"),
        SceneItem,
        ShapeBundle {
            path: path_builder.build(),
            transform: Transform::from_xyz(0.0, 0.0, PATH_Z),
            ..default()
        },
        Stroke::new(Color::CYAN, 1.0),
    ));
}

pub fn spawn_stage(
    mut commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    stage_controls_ui: &Res<StageControlsUI>,
    stage_data: &StageData,
    texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
) {
    if stage_controls_ui.background_is_visible() {
        let mut sprite = Sprite::from_image(asset_server.load(stage_data.background_path.clone()));
        sprite.anchor = Anchor::BottomLeft;

        commands.spawn((
            Name::new("SG Background"),
            SceneItem,
            sprite,
            Transform::from_xyz(0.0, 0.0, BACKGROUND_Z),
        ));
    }

    if stage_controls_ui.skybox_is_visible() {
        let layout_handle = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
            SCREEN_RESOLUTION,
            1,
            2,
            None,
            None,
        ));

        let mut sprite = Sprite::from_atlas_image(
            asset_server.load(stage_data.skybox.path.clone()),
            TextureAtlas {
                layout: layout_handle.clone(),
                index: 0,
            },
        );
        sprite.anchor = Anchor::BottomLeft;

        let camera_position =
            stage_data.calculate_camera_position(stage_controls_ui.ElapsedDuration);
        commands.spawn((
            Name::new("SG Skybox"),
            SceneItem,
            sprite,
            Transform::from_translation(camera_position.extend(SKYBOX_Z)),
            AnimationIndices {
                first: 0,
                last: stage_data.skybox.frames.saturating_sub(1),
            },
            AnimationTimer(Timer::from_seconds(2.0, TimerMode::Repeating)),
        ));
    }

    for (index, spawn) in stage_data
        .spawns
        .iter()
        .filter(|x| stage_controls_ui.depth_is_visible(x.get_depth()))
        .enumerate()
    {
        if spawn.get_elapsed() <= stage_controls_ui.ElapsedDuration {
            let thumbnail = spawn.get_thumbnail();
            let (image_path, rect) = thumbnail;
            let mut sprite = Sprite::from_image(asset_server.load(image_path));
            sprite.anchor = Anchor::BottomCenter;
            sprite.rect = rect;

            commands.spawn((
                spawn.get_editor_name_component(index),
                StageSpawnLabel,
                Draggable,
                SceneItem,
                sprite,
                Transform::from_translation(
                    spawn
                        .get_coordinates()
                        .extend(spawn.get_depth_editor_z_index()),
                ),
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
                    if current_elapsed <= stage_controls_ui.ElapsedDuration
                        && stage_controls_ui.depth_is_visible(spawn.get_depth())
                    {
                        let v = current_position + *spawn.get_coordinates();
                        let (image_path, rect) = spawn.get_thumbnail();
                        let mut sprite = Sprite::from_image(asset_server.load(image_path));
                        sprite.anchor = Anchor::BottomCenter;
                        sprite.rect = rect;

                        commands.spawn((
                            spawn.get_editor_name_component(index),
                            StageSpawnLabel,
                            Draggable,
                            SceneItem,
                            sprite,
                            Transform::from_translation(v.extend(spawn.get_depth_editor_z_index())),
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
                for spawn in s.spawns.iter() {
                    current_elapsed += spawn.get_elapsed();
                    if current_elapsed <= stage_controls_ui.ElapsedDuration
                        && stage_controls_ui.depth_is_visible(spawn.get_depth())
                    {
                        let v = current_position + *spawn.get_coordinates();
                        let (image_path, rect) = spawn.get_thumbnail();
                        let mut sprite = Sprite::from_image(asset_server.load(image_path));
                        sprite.anchor = Anchor::BottomCenter;
                        sprite.rect = rect;

                        commands.spawn((
                            spawn.get_editor_name_component(index),
                            StageSpawnLabel,
                            Draggable,
                            SceneItem,
                            sprite,
                            Transform::from_translation(v.extend(spawn.get_depth_editor_z_index())),
                        ));
                    }
                }
            }
        }
    }

    let info_text = format!(
        "Stage: {}\nMusic: {}\nStart Coordinates: {}\nSteps: {}\nStatic Spawns: {}\nDynamic Spawns: {}",
        stage_data.name,
        stage_data.music_path,
        stage_data.start_coordinates.unwrap_or(Vec2::ZERO),
        stage_data.steps.len(),
        stage_data.spawns.len(),
        stage_data.dynamic_spawn_count(),
    );

    commands.spawn((
        Name::new("SG Info"),
        SceneItem,
        Text2d::new(info_text),
        TextFont {
            font: asset_server.load(FONT_PATH),
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(0.0, -15.0, 0.0),
        Anchor::TopLeft,
    ));

    if stage_controls_ui.path_is_visible() {
        spawn_path(&mut commands, &stage_data, &stage_controls_ui);
    }
}
