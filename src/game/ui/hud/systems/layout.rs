use bevy::prelude::*;

use super::super::{components::*, styles::*};

pub fn spawn_hud(mut commands: Commands, asset_server: Res<AssetServer>) {
    let main_menu_entity = build_hud(&mut commands, &asset_server);
}

pub fn despawn_hud(mut commands: Commands, query: Query<Entity, With<Hud>>) {
    if let Ok(main_menu_entity) = query.get_single() {
        commands.entity(main_menu_entity).despawn_recursive();
    }
}

pub fn build_hud(commands: &mut Commands, asset_server: &Res<AssetServer>) -> Entity {
    let main_menu_entity = commands
        .spawn((
            NodeBundle {
                style: get_hud_style(),
                ..default()
            },
            Hud {},
        ))
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: get_hud_element_style(),
                    background_color: HUD_ELEMENT_BACKGROUND_COLOR.into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(ImageBundle {
                        style: get_hud_element_image_style(),
                        image: asset_server.load("sprites/star.png").into(),
                        ..default()
                    });
                    parent.spawn((
                        TextBundle {
                            text: Text {
                                sections: vec![TextSection::new(
                                    "0",
                                    get_hud_element_text_style(&asset_server),
                                )],
                                alignment: TextAlignment::Center,
                                ..default()
                            },
                            ..default()
                        },
                        ScoreText,
                    ));
                });

            parent
                .spawn(NodeBundle {
                    style: get_hud_element_style(),
                    background_color: HUD_ELEMENT_BACKGROUND_COLOR.into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(ImageBundle {
                        style: get_hud_element_image_style(),
                        image: asset_server.load("sprites/ball_red_large.png").into(),
                        ..default()
                    });
                    parent.spawn((
                        TextBundle {
                            text: Text {
                                sections: vec![TextSection::new(
                                    "0",
                                    get_hud_element_text_style(&asset_server),
                                )],
                                alignment: TextAlignment::Center,
                                ..default()
                            },
                            ..default()
                        },
                        EnemyText,
                    ));
                });
        })
        .id();

    return main_menu_entity;
}
