use bevy::prelude::*;
use bevy::render::color::Color;
use bevy::sprite::Anchor;
use carcinisation::globals::SCREEN_RESOLUTION;

use crate::components::{ActLabel, CutsceneImage, Draggable, LetterboxLabel};
use crate::events::CutsceneLoadedEvent;
use assert_assets_path::assert_assets_path;
use bevy_prototype_lyon::prelude::*;
use carcinisation::letterbox::events::LetterboxMove;

pub fn display_cutscene_acts(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut event_reader: EventReader<CutsceneLoadedEvent>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    let act_offset = 200.0;
    let label_height_offset = 50.0;

    for e in event_reader.read() {
        let mut camera_transform = camera_query.single_mut();
        camera_transform.translation.x = act_offset * e.data.steps.len() as f32 / 2.0;

        let mut previous_position: Option<Vec3> = None;

        for (act_index, act) in e.data.steps.iter().enumerate() {
            let act_position = Vec3::new(act_offset * act_index as f32, 0.0, 0.0);

            commands
                .spawn((
                    Name::new(format!("Act {}", act_index.to_string(),)),
                    // TODO replace with CutsceneAct (make that into a component?)
                    // Or nest it in parent structure
                    ActLabel,
                    Draggable,
                    SpatialBundle::from_transform(Transform::from_translation(act_position)),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        LetterboxLabel,
                        Text2dBundle {
                            text: Text::from_section(
                                format!("Act: {}", act_index),
                                TextStyle {
                                    // TODO assert_assets_path! with right base path to assets
                                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
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
                            let transform =
                                Transform::from_xyz(0.0, 180.0 * image_index as f32, 0.0);

                            parent.spawn((
                                Name::new(format!(
                                    "Act {} : Image {}",
                                    act_index.to_string(),
                                    image_index.to_string()
                                )),
                                CutsceneImage,
                                SpriteBundle {
                                    texture: asset_server.load(&image_spawn.image_path),
                                    transform,
                                    ..Default::default()
                                },
                            ));
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
                            Name::new(format!("Label : Act {} Letterbox", act_index)),
                        ));
                    }

                    // if let Some(prev_pos) = previous_position {
                    //     let mut path_builder = PathBuilder::new();
                    //     path_builder.move_to(prev_pos.truncate());
                    //     path_builder.move_to(act_position.truncate());
                    //     let path = path_builder.build();
                    //     parent.spawn((
                    //         ShapeBundle {
                    //             path,
                    //             ..Default::default()
                    //         },
                    //         Stroke::new(Color::WHITE, 2.0),
                    //     ));
                    // }
                });

            previous_position = Some(act_position);
        }
    }
}
