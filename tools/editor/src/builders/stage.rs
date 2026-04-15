use std::time::Duration;

use crate::builders::thumbnail::resolve_stage_spawn_thumbnail;
use crate::constants::EditorColor;
use crate::inspector::utils::{StageDataUtils, StageSpawnUtils};
use crate::resources::{EditorState, ThumbnailCache};
use crate::timeline::{
    StageTimelineConfig, cinematic_duration, stop_duration, tween_travel_duration,
};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy_prototype_lyon::{prelude::*, shapes};
use carcinisation::globals::SCREEN_RESOLUTION;
use carcinisation::stage::data::{StageData, StageSpawn, StageStep};
use carcinisation::stage::depth_scale::DepthScaleConfig;
use carcinisation::stage::projection::{
    GridParams, build_perspective_grid, evaluate_projection_at,
};

use crate::components::{
    AnimationIndices, AnimationTimer, Draggable, PathOverlay, SceneItem, StageSpawnLabel,
    StageSpawnRef, StartCoordinatesNode, TweenPathNode,
};
use crate::constants::FONT_PATH;
use crate::resources::StageControlsUI;

const SKYBOX_Z: f32 = -11.0;
const BACKGROUND_Z: f32 = -10.0;

// --- Projection overlay constants ---
const PROJECTION_GRID_Z: f32 = 9.0;
const PROJECTION_MARKER_Z: f32 = 9.2;
const PROJECTION_LABEL_Z: f32 = 9.3;

const HORIZON_COLOR: Color = Color::srgba(0.3, 0.5, 1.0, 0.7);
const FLOOR_BASE_COLOR: Color = Color::srgba(0.7, 0.3, 1.0, 0.7);
const DEPTH_LABEL_FONT_SIZE: f32 = 10.0;
const DEPTH_LABEL_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.5);
const DEPTH_LABEL_X_INSET: f32 = 4.0;
/// Minimum vertical spacing (world pixels) between depth labels.  Labels
/// closer than this are culled to prevent overlap near the horizon.
const DEPTH_LABEL_MIN_SPACING: f32 = 6.0;
const CAMERA_POSITION_Z: f32 = 9.9;
const PATH_Z: f32 = 10.0;
const PATH_NODE_Z: f32 = 10.1;
/// Radius of the draggable tween path node handles.
pub const PATH_NODE_RADIUS: f32 = 5.0;
/// Scale factor applied to hovered path nodes.
pub const PATH_NODE_HOVER_SCALE: f32 = 1.5;

/// Spawns the camera path overlay. When `skip_nodes` is true, only decorative geometry
/// (polyline, arrows, camera rect) is created — node handles are omitted to avoid
/// duplicating the actively dragged handle.
pub fn spawn_path(
    commands: &mut Commands,
    stage_data: &StageData,
    stage_controls_ui: &Res<StageControlsUI>,
    skip_nodes: bool,
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
        PathOverlay,
        ShapeBuilder::with(&camera_shape)
            .stroke((Color::WHITE, 1.0))
            .build(),
        Transform {
            translation: camera_position.extend(CAMERA_POSITION_Z),
            ..default()
        },
    ));

    if !skip_nodes {
        let start_node_shape = shapes::Circle {
            radius: PATH_NODE_RADIUS,
            center: Vec2::ZERO,
        };
        let start_transform = Transform::from_translation(
            (stage_data.start_coordinates + h_screen_resolution).extend(PATH_NODE_Z),
        );
        commands.spawn((
            Name::new("Start Coordinates Node"),
            SceneItem,
            PathOverlay,
            StartCoordinatesNode,
            Draggable,
            ShapeBuilder::with(&start_node_shape)
                .fill(Color::srgb(0.2, 1.0, 0.2))
                .stroke((Color::WHITE, 1.0))
                .build(),
            start_transform,
            GlobalTransform::from(start_transform),
        ));
    }

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
                    Name::new(format!("Elapsed Path Tween Arrow {index}")),
                    SceneItem,
                    PathOverlay,
                    ShapeBuilder::with(&arrow_shape).fill(Color::CYAN).build(),
                    Transform {
                        translation: (current_position + h_screen_resolution).extend(PATH_Z),
                        rotation: Quat::from_rotation_z(angle),
                        ..default()
                    },
                    GlobalTransform::default(),
                ));

                if !skip_nodes {
                    // Draggable handle at the tween target position.
                    let node_shape = shapes::Circle {
                        radius: PATH_NODE_RADIUS,
                        center: Vec2::ZERO,
                    };
                    let node_transform = Transform::from_translation(
                        (s.coordinates + h_screen_resolution).extend(PATH_NODE_Z),
                    );
                    commands.spawn((
                        Name::new(format!("Tween Node {index}")),
                        SceneItem,
                        PathOverlay,
                        TweenPathNode { step_index: index },
                        Draggable,
                        ShapeBuilder::with(&node_shape)
                            .fill(Color::WHITE)
                            .stroke((Color::CYAN, 1.0))
                            .build(),
                        node_transform,
                        GlobalTransform::from(node_transform),
                    ));
                }

                let time_to_move = tween_travel_duration(current_position, s);
                current_position = s.coordinates;
                current_elapsed += time_to_move;
            }
            StageStep::Stop(s) => {
                current_elapsed += stop_duration(s, timeline_config);

                // TODO elapsed?
                for spawn in &s.spawns {
                    current_elapsed += spawn.get_elapsed();
                }
            }
        }
    }

    commands.spawn((
        Name::new("Elapsed Path"),
        SceneItem,
        PathOverlay,
        ShapeBuilder::with(&path).stroke((Color::CYAN, 1.0)).build(),
        Transform::from_xyz(0.0, 0.0, PATH_Z),
        GlobalTransform::default(),
    ));
}

/// Spawn the perspective grid overlay at the current scrub position.
///
/// Reuses the shared [`build_perspective_grid`] geometry builder.  All rendering
/// is in editor world space (1:1 carapace pixels, no VIEWPORT_MULTIPLIER).
fn spawn_projection_grid(
    commands: &mut Commands,
    asset_server: &AssetServer,
    stage_data: &StageData,
    controls: &StageControlsUI,
) {
    let profile = evaluate_projection_at(stage_data, controls.elapsed_duration);
    let camera_pos = stage_data.calculate_camera_position(controls.elapsed_duration);
    let screen = SCREEN_RESOLUTION.as_vec2();

    // Compute floor positions from profile, sorted depth-9-first.
    let mut floors: Vec<(i8, f32)> = (1..=9).map(|d| (d, profile.floor_y_for_depth(d))).collect();
    floors.sort_by_key(|&(d, _)| std::cmp::Reverse(d));

    // Viewport in editor world space.
    let viewport = Rect::new(
        camera_pos.x,
        camera_pos.y,
        camera_pos.x + screen.x,
        camera_pos.y + screen.y,
    );
    let vanish_x = camera_pos.x + screen.x * 0.5;

    if controls.projection_grid {
        let grid = build_perspective_grid(&floors, viewport, vanish_x, &GridParams::default());

        // Horizontal depth lines.
        for seg in &grid.depth_lines {
            let color = Color::srgba(
                seg.start_rgba[0],
                seg.start_rgba[1],
                seg.start_rgba[2],
                seg.start_rgba[3],
            );
            let path = ShapePath::new().move_to(seg.start).line_to(seg.end);
            commands.spawn((
                SceneItem,
                PathOverlay,
                ShapeBuilder::with(&path).stroke((color, 1.0)).build(),
                Transform::from_xyz(0.0, 0.0, PROJECTION_GRID_Z),
            ));
        }

        // Guide ray segments — batch per ray for fewer entities.
        // Each segment uses the average of start/end colour (lyon doesn't support
        // per-vertex gradient on strokes; the brightness difference between
        // adjacent depths is ~0.1, so the approximation is nearly invisible).
        for seg in &grid.guide_ray_segments {
            let avg = [
                (seg.start_rgba[0] + seg.end_rgba[0]) * 0.5,
                (seg.start_rgba[1] + seg.end_rgba[1]) * 0.5,
                (seg.start_rgba[2] + seg.end_rgba[2]) * 0.5,
                (seg.start_rgba[3] + seg.end_rgba[3]) * 0.5,
            ];
            let color = Color::srgba(avg[0], avg[1], avg[2], avg[3]);
            let path = ShapePath::new().move_to(seg.start).line_to(seg.end);
            commands.spawn((
                SceneItem,
                PathOverlay,
                ShapeBuilder::with(&path).stroke((color, 1.0)).build(),
                Transform::from_xyz(0.0, 0.0, PROJECTION_GRID_Z),
            ));
        }

        // Depth labels at left edge of each floor line.
        // Near the horizon, cubic bias compresses depths tightly — labels for
        // d7/d8/d9 can overlap at < 1px spacing.  Cull labels that are too
        // close to the previous visible label, working from depth 1 (foreground,
        // most important) toward depth 9 (horizon).
        {
            let label_x = viewport.min.x + DEPTH_LABEL_X_INSET;
            let mut last_label_y = f32::NEG_INFINITY;
            for d in 1..=9i8 {
                let y = profile.floor_y_for_depth(d);
                if (y - last_label_y).abs() < DEPTH_LABEL_MIN_SPACING {
                    continue; // too close to previous label
                }
                last_label_y = y;
                commands.spawn((
                    SceneItem,
                    PathOverlay,
                    Text2d::new(d.to_string()),
                    TextFont {
                        font: asset_server.load(FONT_PATH),
                        font_size: DEPTH_LABEL_FONT_SIZE,
                        ..default()
                    },
                    TextColor(DEPTH_LABEL_COLOR),
                    Transform::from_xyz(label_x, y, PROJECTION_LABEL_Z),
                    Anchor::CENTER_LEFT,
                ));
            }
        }
    }

    // Horizon and floor-base reference lines.
    if controls.projection_markers {
        // Horizon line (blue) — full viewport width.
        let h_path = ShapePath::new()
            .move_to(Vec2::new(viewport.min.x, profile.horizon_y))
            .line_to(Vec2::new(viewport.max.x, profile.horizon_y));
        commands.spawn((
            SceneItem,
            PathOverlay,
            ShapeBuilder::with(&h_path)
                .stroke((HORIZON_COLOR, 1.5))
                .build(),
            Transform::from_xyz(0.0, 0.0, PROJECTION_MARKER_Z),
        ));

        // Floor-base line (purple) — full viewport width.
        let f_path = ShapePath::new()
            .move_to(Vec2::new(viewport.min.x, profile.floor_base_y))
            .line_to(Vec2::new(viewport.max.x, profile.floor_base_y));
        commands.spawn((
            SceneItem,
            PathOverlay,
            ShapeBuilder::with(&f_path)
                .stroke((FLOOR_BASE_COLOR, 1.5))
                .build(),
            Transform::from_xyz(0.0, 0.0, PROJECTION_MARKER_Z),
        ));
    }
}

/// Spawns stage background/skybox, spawns, and optional path overlay.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub fn spawn_stage(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    editor_state: &EditorState,
    stage_controls_ui: &Res<StageControlsUI>,
    stage_data: &StageData,
    image_assets: &mut Assets<Image>,
    texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
    thumbnail_cache: &mut ThumbnailCache,
    depth_scale_config: &DepthScaleConfig,
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
        let preview = resolve_stage_spawn_thumbnail(
            spawn,
            asset_server,
            image_assets,
            thumbnail_cache,
            depth_scale_config,
            None,
        );

        let depth_scale = editor_depth_scale(editor_state, depth_scale_config, spawn.get_depth());
        let total_scale = depth_scale * preview.fallback_scale;
        commands.spawn((
            spawn.get_editor_name_component(index),
            StageSpawnLabel,
            StageSpawnRef::Static { index },
            Draggable,
            SceneItem,
            preview.sprite,
            Transform::from_translation(
                spawn
                    .get_coordinates()
                    .extend(spawn.get_depth_editor_z_index()),
            )
            .with_scale(Vec3::splat(total_scale)),
            preview.anchor,
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
                let ghost = !step_started && stage_controls_ui.show_all_spawns;
                if step_started || ghost {
                    spawn_step_entities(
                        commands,
                        asset_server,
                        stage_controls_ui,
                        image_assets,
                        thumbnail_cache,
                        &s.spawns,
                        index,
                        current_position,
                        ghost,
                        editor_state,
                        depth_scale_config,
                    );
                }

                let time_to_move = tween_travel_duration(current_position, s);
                current_position = s.coordinates;
                current_elapsed += time_to_move;
            }
            StageStep::Stop(s) => {
                let step_started = stage_controls_ui.elapsed_duration >= current_elapsed;
                let ghost = !step_started && stage_controls_ui.show_all_spawns;
                if step_started || ghost {
                    spawn_step_entities(
                        commands,
                        asset_server,
                        stage_controls_ui,
                        image_assets,
                        thumbnail_cache,
                        &s.spawns,
                        index,
                        current_position,
                        ghost,
                        editor_state,
                        depth_scale_config,
                    );
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

    // Projection grid and markers (before path, so path renders on top).
    if stage_controls_ui.projection_grid || stage_controls_ui.projection_markers {
        spawn_projection_grid(commands, asset_server, stage_data, stage_controls_ui);
    }

    if stage_controls_ui.path_is_visible() {
        spawn_path(commands, stage_data, stage_controls_ui, false);
    }
}

const GHOST_ALPHA: f32 = 0.3;

/// Spawns entities for a step's spawns list, optionally as 30% opacity ghosts.
#[allow(clippy::too_many_arguments)]
fn spawn_step_entities(
    commands: &mut Commands,
    asset_server: &AssetServer,
    stage_controls_ui: &StageControlsUI,
    image_assets: &mut Assets<Image>,
    thumbnail_cache: &mut ThumbnailCache,
    spawns: &[StageSpawn],
    step_index: usize,
    step_origin: Vec2,
    ghost: bool,
    editor_state: &EditorState,
    depth_scale_config: &DepthScaleConfig,
) {
    for (spawn_index, spawn) in spawns.iter().enumerate() {
        if !stage_controls_ui.depth_is_visible(spawn.get_depth()) {
            continue;
        }
        let v = step_origin + *spawn.get_coordinates();
        let mut preview = resolve_stage_spawn_thumbnail(
            spawn,
            asset_server,
            image_assets,
            thumbnail_cache,
            depth_scale_config,
            None,
        );

        if ghost {
            preview.sprite.color = preview.sprite.color.with_alpha(GHOST_ALPHA);
        }

        let depth_scale = editor_depth_scale(editor_state, depth_scale_config, spawn.get_depth());
        let total_scale = depth_scale * preview.fallback_scale;
        commands.spawn((
            spawn.get_editor_name_component(step_index),
            StageSpawnLabel,
            StageSpawnRef::Step {
                step_index,
                spawn_index,
                step_origin,
            },
            Draggable,
            SceneItem,
            preview.sprite,
            Transform::from_translation(v.extend(spawn.get_depth_editor_z_index()))
                .with_scale(Vec3::splat(total_scale)),
            preview.anchor,
        ));
    }
}

fn editor_depth_scale(
    editor_state: &EditorState,
    depth_scale_config: &DepthScaleConfig,
    depth: carcinisation::stage::components::placement::Depth,
) -> f32 {
    if editor_state.depth_preview_enabled {
        depth_scale_config.scale_for(depth).unwrap_or(1.0)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use carcinisation::stage::components::placement::Depth;

    #[test]
    fn authoring_mode_keeps_background_and_spawns_at_identity_scale() {
        let editor_state = EditorState::default();
        let config = DepthScaleConfig::default();

        assert_eq!(Transform::from_xyz(0.0, 0.0, BACKGROUND_Z).scale, Vec3::ONE);
        assert_eq!(
            Transform::from_translation(Vec2::ZERO.extend(0.0))
                .with_scale(Vec3::splat(editor_depth_scale(
                    &editor_state,
                    &config,
                    Depth::One,
                )))
                .scale,
            Vec3::ONE
        );
        assert_eq!(
            Transform::from_translation(Vec2::ZERO.extend(0.0))
                .with_scale(Vec3::splat(editor_depth_scale(
                    &editor_state,
                    &config,
                    Depth::Nine,
                )))
                .scale,
            Vec3::ONE
        );
    }

    #[test]
    fn depth_preview_mode_applies_configured_depth_scales() {
        let editor_state = EditorState {
            depth_preview_enabled: true,
        };
        let config = DepthScaleConfig::default();

        let shallow = editor_depth_scale(&editor_state, &config, Depth::One);
        let deep = editor_depth_scale(&editor_state, &config, Depth::Nine);

        assert_eq!(shallow, config.scale_for(Depth::One).unwrap());
        assert_eq!(deep, config.scale_for(Depth::Nine).unwrap());
        assert_eq!(
            Transform::from_translation(Vec2::ZERO.extend(0.0))
                .with_scale(Vec3::splat(shallow))
                .scale,
            Vec3::splat(config.scale_for(Depth::One).unwrap())
        );
        assert_eq!(
            Transform::from_translation(Vec2::ZERO.extend(0.0))
                .with_scale(Vec3::splat(deep))
                .scale,
            Vec3::splat(config.scale_for(Depth::Nine).unwrap())
        );
        assert_ne!(shallow, deep);
    }
}
