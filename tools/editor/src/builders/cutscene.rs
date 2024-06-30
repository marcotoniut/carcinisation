use bevy::prelude::*;
use bevy::render::color::Color;
use bevy::sprite::Anchor;
use bevy_prototype_lyon::draw::Stroke;
use bevy_prototype_lyon::entity::ShapeBundle;
use bevy_prototype_lyon::geometry::GeometryBuilder;
use bevy_prototype_lyon::path::PathBuilder;
use carcinisation::globals::SCREEN_RESOLUTION;
use carcinisation::CutsceneData;

use crate::components::{
    CutsceneActConnection, CutsceneActNode, CutsceneImage, CutsceneImageLabel, Draggable,
    LetterboxLabel, SceneItem,
};
use crate::constants::FONT_PATH;
use carcinisation::letterbox::events::LetterboxMove;

const ACT_OFFSET: f32 = 200.0;

pub fn spawn_cutscene(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    data: &CutsceneData, // assuming CutsceneData is the type of data in LoadedScene::Cutscene
                         // camera_query: &mut Query<&mut Transform, With<Camera>>,
) {
    // let mut camera_transform = camera_query.single_mut();
    // camera_transform.translation.x = ACT_OFFSET * data.steps.len() as f32 / 2.0;

    let h1_text_style = TextStyle {
        font: asset_server.load(FONT_PATH),
        font_size: 16.0,
        color: Color::WHITE,
    };
    let h2_text_style = TextStyle {
        font: asset_server.load(FONT_PATH),
        font_size: 14.0,
        color: Color::WHITE,
    };

    let mut previous_entity_o: Option<Entity> = None;

    let mut connection_bundles = Vec::new();

    for (act_index, act) in data.steps.iter().enumerate() {
        let act_position = Vec3::new(ACT_OFFSET * act_index as f32, 0.0, 0.0);

        let mut entity_commands = commands.spawn((
            Name::new(format!("Act {}", act_index)),
            CutsceneActNode { act_index },
            Draggable,
            SceneItem,
            SpatialBundle::from_transform(Transform::from_translation(act_position)),
        ));
        entity_commands.with_children(|p0| {
            p0.spawn(Text2dBundle {
                text: Text::from_section(
                    format!("Act {} ({}s)", act_index, act.elapse.as_secs_f32()),
                    h1_text_style.clone(),
                ),
                transform: Transform::from_xyz(0.0, SCREEN_RESOLUTION.y as f32 / 2.0 + 25.0, 0.0),
                ..default()
            });

            if let Some(spawn_images) = &act.spawn_images_o {
                for (image_index, image_spawn) in spawn_images.spawns.iter().enumerate() {
                    let transform = Transform::from_xyz(0.0, 180.0 * image_index as f32, 0.0);

                    let texture = asset_server.load(&image_spawn.image_path);

                    p0.spawn((
                        Name::new(format!("Act {} : Image {}", act_index, image_index)),
                        CutsceneImage,
                        SpriteBundle {
                            texture,
                            transform,
                            ..default()
                        },
                    ))
                    .with_children(|p1| {
                        p1.spawn((
                            CutsceneImageLabel,
                            Text2dBundle {
                                text: Text::from_section(
                                    &image_spawn.image_path,
                                    h2_text_style.clone(),
                                ),
                                text_anchor: Anchor::TopLeft,
                                ..default()
                            },
                            Name::new(format!("Label - Image {}", image_index)),
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

                p0.spawn((
                    LetterboxLabel,
                    Name::new("Letterbox Header"),
                    NodeBundle {
                        style: Style {
                            row_gap: Val::Px(10.0),
                            ..default()
                        },
                        transform: Transform::from_xyz(
                            0.0,
                            SCREEN_RESOLUTION.y as f32 / 2.0 + 10.0,
                            0.0,
                        ),
                        ..default()
                    },
                ))
                .with_children(|p1| {
                    p1.spawn(Text2dBundle {
                        text: Text::from_section(
                            format!("Letterbox {}", instruction),
                            h1_text_style.clone(),
                        ),
                        ..default()
                    });
                });
            }

            if let Some(previous_entity) = previous_entity_o {
                let path_builder = PathBuilder::new();
                let shape = path_builder.build();

                let current_entity = p0.parent_entity();

                connection_bundles.push((
                    Name::new(format!(
                        "Act Connection {} {}",
                        previous_entity.index(),
                        current_entity.index(),
                    )),
                    CutsceneActConnection {
                        origin: previous_entity,
                        target: p0.parent_entity(),
                    },
                    SceneItem,
                    ShapeBundle {
                        path: GeometryBuilder::build_as(&shape),
                        ..default()
                    },
                    Stroke::new(Color::WHITE, 2.0),
                ));
            }
        });

        previous_entity_o = Some(entity_commands.id());
    }

    for bundle in connection_bundles {
        commands.spawn(bundle);
    }
}
