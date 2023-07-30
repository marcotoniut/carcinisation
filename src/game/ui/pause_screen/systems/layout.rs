use bevy::prelude::*;

use super::super::{components::*, styles::*};

pub fn spawn_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    build_screen(&mut commands, &asset_server);
}

pub fn despawn_screen(mut commands: Commands, query: Query<Entity, With<PauseScreen>>) {
    if let Ok(main_menu_entity) = query.get_single() {
        commands.entity(main_menu_entity).despawn_recursive();
    }
}

pub fn build_screen(commands: &mut Commands, asset_server: &Res<AssetServer>) -> Entity {
    let main_menu_entity = commands
        .spawn((
            NodeBundle {
                style: get_screen_style(),
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.4).into(),
                ..default()
            },
            PauseScreen {},
        ))
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: get_score_style(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle {
                        text: Text {
                            sections: vec![TextSection::new(
                                "Score",
                                get_score_text_style(asset_server),
                            )],
                            ..default()
                        },
                        ..default()
                    });
                });

            parent
                .spawn((
                    ButtonBundle {
                        style: get_button_style(),
                        background_color: NORMAL_BUTTON_COLOR.into(),
                        ..default()
                    },
                    ResumeButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle {
                        text: Text {
                            sections: vec![TextSection::new(
                                "Play",
                                get_button_text_style(&asset_server),
                            )],
                            alignment: TextAlignment::Center,
                            ..default()
                        },
                        ..default()
                    });
                });

            parent
                .spawn((
                    ButtonBundle {
                        style: get_button_style(),
                        background_color: NORMAL_BUTTON_COLOR.into(),
                        ..default()
                    },
                    QuitToMainMenuButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle {
                        text: Text {
                            sections: vec![TextSection::new(
                                "Quit to main menu",
                                get_button_text_style(&asset_server),
                            )],
                            alignment: TextAlignment::Center,
                            ..default()
                        },
                        ..default()
                    });
                });

            parent
                .spawn((
                    ButtonBundle {
                        style: get_button_style(),
                        background_color: NORMAL_BUTTON_COLOR.into(),
                        ..default()
                    },
                    QuitButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle {
                        text: Text {
                            sections: vec![TextSection::new(
                                "Quit",
                                get_button_text_style(&asset_server),
                            )],
                            alignment: TextAlignment::Center,
                            ..default()
                        },
                        ..default()
                    });
                });
        })
        .id();

    return main_menu_entity;
}
