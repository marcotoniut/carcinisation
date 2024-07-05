use std::time::Duration;

use bevy::prelude::*;
use bevy::render::color::Color;
use bevy::sprite::Anchor;
use bevy_prototype_lyon::geometry::GeometryBuilder;
use bevy_prototype_lyon::shapes;
use carcinisation::stage::data::{ObjectType, PickupType, StageData, StageSpawn, StageStep};
use carcinisation::stage::destructible::components::DestructibleType;
use carcinisation::stage::enemy::entity::EnemyType;

use crate::components::{Draggable, SceneItem, StageSpawnLabel};
use crate::constants::FONT_PATH;
use crate::resources::StageElapsedUI;

use super::thumbnail::{
    get_destructible_thumbnail, get_enemy_thumbnail, get_object_thumbnail, get_pickup_thumbnail,
};

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

    let mut elapsed: f32 = 0.0;

    for (spawn_index, spawn) in stage_data.spawns.iter().enumerate() {
        let ((position, name, show), thumbnail) = match spawn {
            StageSpawn::Object(x) => (
                match x.object_type {
                    ObjectType::BenchBig => (x.coordinates, x.show_type(), true),
                    ObjectType::BenchSmall => (x.coordinates, x.show_type(), true),
                    ObjectType::Fibertree => (x.coordinates, x.show_type(), true),
                    ObjectType::RugparkSign => (x.coordinates, x.show_type(), true),
                },
                get_object_thumbnail(x.object_type.clone()),
            ),
            StageSpawn::Destructible(x) => (
                match x.destructible_type {
                    DestructibleType::Lamp => (x.coordinates, x.show_type(), true),
                    DestructibleType::Trashcan => (x.coordinates, x.show_type(), true),
                    DestructibleType::Crystal => (x.coordinates, x.show_type(), true),
                    DestructibleType::Mushroom => (x.coordinates, x.show_type(), true),
                },
                get_destructible_thumbnail(x.destructible_type.clone()),
            ),
            StageSpawn::Pickup(x) => {
                elapsed += x.elapsed;
                let show = stage_elapsed_ui.0.as_secs_f32() <= elapsed;
                (
                    match x.pickup_type {
                        PickupType::SmallHealthpack => (x.coordinates, x.show_type(), show),
                        PickupType::BigHealthpack => (x.coordinates, x.show_type(), show),
                    },
                    get_pickup_thumbnail(x.pickup_type.clone()),
                )
            }
            StageSpawn::Enemy(x) => {
                elapsed += x.elapsed;
                let show = stage_elapsed_ui.0.as_secs_f32() <= elapsed;
                (
                    match x.enemy_type {
                        EnemyType::Mosquito => (x.coordinates, x.enemy_type.show_type(), show),
                        EnemyType::Spidey => (x.coordinates, x.enemy_type.show_type(), show),
                        EnemyType::Tardigrade => (x.coordinates, x.enemy_type.show_type(), show),
                        EnemyType::Marauder => (x.coordinates, x.enemy_type.show_type(), show),
                        EnemyType::Spidomonsta => (x.coordinates, x.enemy_type.show_type(), show),
                        EnemyType::Kyle => (x.coordinates, x.enemy_type.show_type(), show),
                    },
                    get_enemy_thumbnail(x.enemy_type.clone()),
                )
            }
        };

        if show {
            commands
                .spawn((
                    Name::new(format!("Stage spawn {}: {}", spawn_index, name)),
                    StageSpawnLabel,
                    Draggable,
                    SceneItem,
                    SpriteBundle {
                        texture: asset_server.load(&thumbnail.0),
                        transform: Transform::from_xyz(position.x, position.y, 0.0),
                        sprite: Sprite {
                            anchor: Anchor::BottomCenter,
                            ..default()
                        },
                        ..default()
                    },
                ))
                .with_children(|parent| {
                    // Spawn a colored circle for the spawn point
                    parent.spawn((
                        // FillMode::color(color), Transform::default()
                        GeometryBuilder::new()
                            .add(&shapes::Circle {
                                radius: 5.0,
                                center: Vec2::ZERO,
                            })
                            .build(),
                        Name::new(format!("Circle {}", spawn_index)),
                    ));

                    parent.spawn(Text2dBundle {
                        text: Text::from_section(name, label_style.clone()),
                        transform: Transform::from_xyz(0.0, 10.0, 0.0),
                        ..default()
                    });
                });
        }
    }

    let mut position = Vec2::ZERO;
    let mut elapsed: f32 = 0.0;
    for (spawn_index, step) in stage_data.steps.iter().enumerate() {
        match step {
            StageStep::Cinematic(s) => {
                // TODO
            }
            StageStep::Movement(s) => {}
            StageStep::Stop(s) => {
                elapsed += s
                    .max_duration
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs_f32();
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
