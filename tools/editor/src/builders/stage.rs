use std::time::Duration;

use crate::constants::EditorColor;
use crate::inspector::utils::{StageDataUtils, StageSpawnUtils};
use crate::timeline::{
    cinematic_duration, stop_duration, tween_travel_duration, StageTimelineConfig,
};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy_prototype_lyon::{prelude::*, shapes};
use carcinisation::globals::SCREEN_RESOLUTION;
use carcinisation::stage::data::{StageData, StageStep};

use crate::components::{
    AnimationIndices, AnimationTimer, Draggable, SceneItem, StageSpawnLabel, StageSpawnRef,
};
use crate::constants::FONT_PATH;
use crate::resources::StageControlsUI;

const SKYBOX_Z: f32 = -11.0;
const BACKGROUND_Z: f32 = -10.0;
const CAMERA_POSITION_Z: f32 = 9.9;
const PATH_Z: f32 = 10.0;

/// Spawns the elapsed camera path overlay for the stage.
pub fn spawn_path(
    commands: &mut Commands,
    stage_data: &StageData,
    stage_controls_ui: &Res<StageControlsUI>,
) {
    let screen_resolution = SCREEN_RESOLUTION.as_vec2();
    let h_screen_resolution = screen_resolution / 2.0;

    let camera_position = stage_data.calculate_camera_position(stage_controls_ui.elapsed_duration);
    let camera_shape = shapes::Polygon {
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
        ShapeBuilder::with(&camera_shape)
            .stroke((Color::WHITE, 1.0))
            .build(),
        Transform {
            translation: camera_position.extend(CAMERA_POSITION_Z),
            ..default()
        },
    ));

    let mut path = ShapePath::new().move_to(stage_data.start_coordinates + h_screen_resolution);

    let mut current_position = stage_data.start_coordinates;
    let mut current_elapsed: Duration = Duration::ZERO;
    let timeline_config = StageTimelineConfig::SLIDER;

    for (index, step) in stage_data.steps.iter().enumerate() {
        match step {
            StageStep::Cinematic(s) => {
                current_elapsed += cinematic_duration(s, timeline_config);
            }
            StageStep::Tween(s) => {
                path = path.line_to(s.coordinates + h_screen_resolution);

                let direction = (current_position - s.coordinates).normalize_or_zero();
                let angle = direction.y.atan2(direction.x);

                let arrow_shape = shapes::Polygon {
                    points: vec![
                        Vec2::new(0.0, 0.0),
                        Vec2::new(6.0, -3.0),
                        Vec2::new(6.0, 3.0),
                    ],
                    closed: true,
                };
                commands.spawn((
                    Name::new(format!("Elapsed Path Tween Arrow {}", index)),
                    SceneItem,
                    ShapeBuilder::with(&arrow_shape).fill(Color::CYAN).build(),
                    Transform {
                        translation: (current_position + h_screen_resolution).extend(PATH_Z),
                        rotation: Quat::from_rotation_z(angle),
                        ..default()
                    },
                    GlobalTransform::default(),
                ));

                let time_to_move = tween_travel_duration(current_position, s);
                current_position = s.coordinates;
                current_elapsed += time_to_move;
            }
            StageStep::Stop(s) => {
                current_elapsed += stop_duration(s, timeline_config);

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
        ShapeBuilder::with(&path).stroke((Color::CYAN, 1.0)).build(),
        Transform::from_xyz(0.0, 0.0, PATH_Z),
        GlobalTransform::default(),
    ));
}

/// Spawns stage background/skybox, spawns, and optional path overlay.
pub fn spawn_stage(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    stage_controls_ui: &Res<StageControlsUI>,
    stage_data: &StageData,
    texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
) {
    if stage_controls_ui.background_is_visible() {
        let sprite = Sprite::from_image(asset_server.load(stage_data.background_path.clone()));

        commands.spawn((
            Name::new("SG Background"),
            SceneItem,
            sprite,
            Transform::from_xyz(0.0, 0.0, BACKGROUND_Z),
            Anchor::BOTTOM_LEFT,
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

        let sprite = Sprite::from_atlas_image(
            asset_server.load(stage_data.skybox.path.clone()),
            TextureAtlas {
                layout: layout_handle.clone(),
                index: 0,
            },
        );

        let camera_position =
            stage_data.calculate_camera_position(stage_controls_ui.elapsed_duration);
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
            Anchor::BOTTOM_LEFT,
        ));
    }

    for (index, spawn) in stage_data
        .spawns
        .iter()
        .filter(|x| stage_controls_ui.depth_is_visible(x.get_depth()))
        .enumerate()
    {
        let (image_path, rect) = spawn.get_thumbnail();
        let mut sprite = Sprite::from_image(asset_server.load(image_path));
        sprite.rect = rect;

        commands.spawn((
            spawn.get_editor_name_component(index),
            StageSpawnLabel,
            StageSpawnRef::Static { index },
            Draggable,
            SceneItem,
            sprite,
            Transform::from_translation(
                spawn
                    .get_coordinates()
                    .extend(spawn.get_depth_editor_z_index()),
            ),
            Anchor::BOTTOM_CENTER,
        ));
    }

    let mut current_position = stage_data.start_coordinates;
    let mut current_elapsed: Duration = Duration::ZERO;
    let timeline_config = StageTimelineConfig::SLIDER;
    for (index, step) in stage_data.steps.iter().enumerate() {
        match step {
            StageStep::Cinematic(s) => {
                current_elapsed += cinematic_duration(s, timeline_config);
            }
            StageStep::Tween(s) => {
                let step_started = stage_controls_ui.elapsed_duration >= current_elapsed;
                if step_started {
                    for (spawn_index, spawn) in s.spawns.iter().enumerate() {
                        if stage_controls_ui.depth_is_visible(spawn.get_depth()) {
                            let v = current_position + *spawn.get_coordinates();
                            let (image_path, rect) = spawn.get_thumbnail();
                            let mut sprite = Sprite::from_image(asset_server.load(image_path));
                            sprite.rect = rect;

                            commands.spawn((
                                spawn.get_editor_name_component(index),
                                StageSpawnLabel,
                                StageSpawnRef::Step {
                                    step_index: index,
                                    spawn_index,
                                    step_origin: current_position,
                                },
                                Draggable,
                                SceneItem,
                                sprite,
                                Transform::from_translation(
                                    v.extend(spawn.get_depth_editor_z_index()),
                                ),
                                Anchor::BOTTOM_CENTER,
                            ));
                        }
                    }
                }

                let time_to_move = tween_travel_duration(current_position, s);
                current_position = s.coordinates;
                current_elapsed += time_to_move;
            }
            StageStep::Stop(s) => {
                let step_started = stage_controls_ui.elapsed_duration >= current_elapsed;
                if step_started {
                    for (spawn_index, spawn) in s.spawns.iter().enumerate() {
                        if stage_controls_ui.depth_is_visible(spawn.get_depth()) {
                            let v = current_position + *spawn.get_coordinates();
                            let (image_path, rect) = spawn.get_thumbnail();
                            let mut sprite = Sprite::from_image(asset_server.load(image_path));
                            sprite.rect = rect;

                            commands.spawn((
                                spawn.get_editor_name_component(index),
                                StageSpawnLabel,
                                StageSpawnRef::Step {
                                    step_index: index,
                                    spawn_index,
                                    step_origin: current_position,
                                },
                                Draggable,
                                SceneItem,
                                sprite,
                                Transform::from_translation(
                                    v.extend(spawn.get_depth_editor_z_index()),
                                ),
                                Anchor::BOTTOM_CENTER,
                            ));
                        }
                    }
                }
                current_elapsed += stop_duration(s, timeline_config);
            }
        }
    }

    let info_text = format!(
        "Stage: {}\nMusic: {}\nStart Coordinates: {}\nSteps: {}\nStatic Spawns: {}\nDynamic Spawns: {}",
        stage_data.name,
        stage_data.music_path,
        stage_data.start_coordinates,
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
        Anchor::TOP_LEFT,
    ));

    if stage_controls_ui.path_is_visible() {
        spawn_path(commands, stage_data, stage_controls_ui);
    }
}
