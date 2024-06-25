use bevy::prelude::*;
use bevy::render::color::Color;
use bevy::sprite::Anchor;
use bevy_inspector_egui::egui::Stroke;
use bevy_prototype_lyon::draw::Fill;
use bevy_prototype_lyon::entity::{Path, ShapeBundle};
use bevy_prototype_lyon::geometry::GeometryBuilder;
use bevy_prototype_lyon::path::PathBuilder;
use carcinisation::globals::SCREEN_RESOLUTION;

use crate::components::{
    CutsceneActConnection, CutsceneActNode, CutsceneImage, CutsceneImageLabel, Draggable,
    LetterboxLabel,
};
use crate::constants::FONT_PATH;
use crate::events::CutsceneLoadedEvent;
use carcinisation::letterbox::events::LetterboxMove;

pub fn display_cutscene_acts(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut event_reader: EventReader<CutsceneLoadedEvent>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    let act_offset = 200.0;

    for e in event_reader.read() {
        let mut camera_transform = camera_query.single_mut();
        camera_transform.translation.x = act_offset * e.data.steps.len() as f32 / 2.0;

        let mut previous_entity_o: Option<Entity> = None;

        let mut connection_bundles = Vec::new();

        for (act_index, act) in e.data.steps.iter().enumerate() {
            let act_position = Vec3::new(act_offset * act_index as f32, 0.0, 0.0);

            let mut entity_commands = commands.spawn((
                Name::new(format!("Act {}", act_index.to_string(),)),
                // TODO replace with CutsceneAct (make that into a component?)
                // Or nest it in parent structure
                CutsceneActNode,
                Draggable,
                SpatialBundle::from_transform(Transform::from_translation(act_position)),
            ));
            entity_commands.with_children(|parent| {
                parent.spawn((
                    LetterboxLabel,
                    Text2dBundle {
                        text: Text::from_section(
                            format!("Act: {}", act_index),
                            TextStyle {
                                font: asset_server.load(FONT_PATH),
                                font_size: 16.0,
                                color: Color::WHITE,
                            },
                        ),
                        transform: Transform::from_xyz(
                            0.0,
                            SCREEN_RESOLUTION.y as f32 / 2.0 + 25.0,
                            0.0,
                        ),
                        text_anchor: Anchor::BottomCenter,
                        ..Default::default()
                    },
                    Name::new(format!("Label : Act {}", act_index)),
                ));

                if let Some(spawn_images) = &act.spawn_images_o {
                    for (image_index, image_spawn) in spawn_images.spawns.iter().enumerate() {
                        let transform = Transform::from_xyz(0.0, 180.0 * image_index as f32, 0.0);

                        let texture = asset_server.load(&image_spawn.image_path);

                        parent
                            .spawn((
                                Name::new(format!(
                                    "Act {} : Image {}",
                                    act_index.to_string(),
                                    image_index.to_string()
                                )),
                                CutsceneImage,
                                SpriteBundle {
                                    texture,
                                    transform,
                                    ..Default::default()
                                },
                            ))
                            .with_children(|p2| {
                                p2.spawn((
                                    CutsceneImageLabel,
                                    Text2dBundle {
                                        text: Text::from_section(
                                            format!(
                                                "Image {}: {}",
                                                image_index.to_string(),
                                                &image_spawn.image_path
                                            ),
                                            TextStyle {
                                                // TODO assert_assets_path! with right base path to assets
                                                font: asset_server.load(FONT_PATH),
                                                font_size: 14.0,
                                                color: Color::WHITE,
                                            },
                                        ),
                                        text_anchor: Anchor::TopLeft,
                                        ..Default::default()
                                    },
                                    Name::new(format!("Label - Image {}", image_index.to_string())),
                                ));
                            });
                    }
                }

                if let Some(letterbox_move) = &act.letterbox_move_o {
                    let instruction = match letterbox_move {
                        LetterboxMove::Open => "Open".to_string(),
                        LetterboxMove::Hide => "Hide".to_string(),
                        LetterboxMove::Show => "Show".to_string(),
                        LetterboxMove::Close => "Close".to_string(),
                        LetterboxMove::To(x) => format!("To {}", x),
                        LetterboxMove::ToAt(x, y) => format!("ToAt {} {}", x, y),
                    };

                    parent.spawn((
                        LetterboxLabel,
                        Text2dBundle {
                            text: Text::from_section(
                                format!("Letterbox: {}", instruction),
                                TextStyle {
                                    // TODO assert_assets_path! with right base path to assets
                                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                    font_size: 16.0,
                                    color: Color::WHITE,
                                },
                            ),
                            transform: Transform::from_xyz(
                                0.0,
                                SCREEN_RESOLUTION.y as f32 / 2.0 + 10.0,
                                0.0,
                            ),
                            text_anchor: Anchor::BottomCenter,
                            ..Default::default()
                        },
                        Name::new(format!("Label - Act {} Letterbox", act_index)),
                    ));
                }

                if let Some(previous_entity) = previous_entity_o {
                    let path_builder = PathBuilder::new();
                    let shape = path_builder.build();

                    let current_entity = parent.parent_entity();

                    connection_bundles.push((
                        Name::new(format!(
                            "Act Connection {} {}",
                            previous_entity.index(),
                            current_entity.index(),
                        )),
                        CutsceneActConnection {
                            origin: previous_entity,
                            target: parent.parent_entity(),
                        },
                        ShapeBundle {
                            path: GeometryBuilder::build_as(&shape),
                            ..Default::default()
                        },
                    ));
                }
            });

            previous_entity_o = Some(entity_commands.id());
        }

        for bundle in connection_bundles {
            commands.spawn(bundle);
        }
    }
}

pub fn update_cutscene_act_connections(
    mut commands: Commands,
    mut cutscene_act_connections_query: Query<(Entity, &mut CutsceneActConnection, &mut Path)>,
    mut cutscene_act_node_query: Query<&Transform, With<CutsceneActNode>>,
) {
    for (connection_entity, mut connection, mut path) in cutscene_act_connections_query.iter_mut() {
        match (
            cutscene_act_node_query.get(connection.origin),
            cutscene_act_node_query.get(connection.target),
        ) {
            (Ok(origin_transform), Ok(target_transform)) => {
                let origin_position = origin_transform.translation;
                let target_position = target_transform.translation;

                // Create a new path from the origin to the target
                let mut path_builder = PathBuilder::new();
                path_builder.move_to(origin_position.truncate());
                path_builder.line_to(target_position.truncate());
                let shape = path_builder.build();

                // Update the path of the connection
                // *path = GeometryBuilder::build_as(&shape);
                commands.entity(connection_entity).insert(shape);
            }
            _ => {
                // If either the origin or target entity is not found, despawn the connection entity
                // commands.entity(connection_entity).despawn();
            }
        };
    }
}
