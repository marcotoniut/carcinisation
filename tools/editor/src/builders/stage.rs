use bevy::prelude::*;
use bevy::render::color::Color;
use bevy_prototype_lyon::geometry::GeometryBuilder;
use bevy_prototype_lyon::shapes;
use carcinisation::stage::data::{StageData, StageSpawn};

use crate::components::{Draggable, SceneItem, StageSpawnLabel};
use crate::constants::FONT_PATH;

pub fn spawn_stage(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    stage_data: &StageData,
) {
    let background_texture = asset_server.load(&stage_data.background_path);

    // Spawn background
    commands.spawn((
        Name::new("Stage Background"),
        SceneItem,
        SpriteBundle {
            texture: background_texture,
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
    ));

    let label_style = TextStyle {
        font: asset_server.load(FONT_PATH),
        font_size: 12.0,
        color: Color::WHITE,
    };

    // Spawn all stage elements
    for (spawn_index, spawn) in stage_data.spawns.iter().enumerate() {
        let (position, name, color) = match spawn {
            StageSpawn::Object(obj) => (obj.coordinates, obj.show_type(), Color::BLUE),
            StageSpawn::Destructible(dest) => (dest.coordinates, dest.show_type(), Color::GREEN),
            StageSpawn::Pickup(pickup) => (pickup.coordinates, pickup.show_type(), Color::YELLOW),
            StageSpawn::Enemy(enemy) => {
                (enemy.coordinates, enemy.enemy_type.show_type(), Color::RED)
            }
        };

        commands
            .spawn((
                Name::new(format!("Spawn {}: {}", spawn_index, name)),
                StageSpawnLabel,
                Draggable,
                SceneItem,
                SpatialBundle::from_transform(Transform::from_translation(position.extend(1.0))),
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

                // Spawn text label
                parent.spawn(Text2dBundle {
                    text: Text::from_section(name, label_style.clone()),
                    transform: Transform::from_xyz(0.0, 10.0, 0.0),
                    ..default()
                });
            });
    }

    // Spawn stage info text
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
