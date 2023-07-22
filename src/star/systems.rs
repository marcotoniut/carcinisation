use super::{components::*, resources::*};

use bevy::{prelude::*, window::PrimaryWindow};

pub fn spawn_stars(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
) {
    let window: &Window = window_query.get_single().unwrap();

    let texture = asset_server.load("sprites/star.png");

    for _ in 0..NUMBER_OF_STARS {
        commands.spawn((
            SpriteBundle {
                texture: texture.clone(),
                transform: Transform::from_xyz(
                    rand::random::<f32>() * window.width(),
                    rand::random::<f32>() * window.height(),
                    0.0,
                ),
                ..default()
            },
            Star {},
        ));
    }
}

pub fn tick_star_spawn_timer(mut star_spawn_timer: ResMut<StarSpawnTimer>, time: Res<Time>) {
    star_spawn_timer.timer.tick(time.delta());
}

pub fn spawn_stars_over_time(
    mut commands: Commands,
    star_spawn_timer: Res<StarSpawnTimer>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
) {
    if star_spawn_timer.timer.finished() {
        let window: &Window = window_query.get_single().unwrap();

        let texture = asset_server.load("sprites/star.png");

        commands.spawn((
            SpriteBundle {
                texture,
                transform: Transform::from_xyz(
                    rand::random::<f32>() * window.width(),
                    rand::random::<f32>() * window.height(),
                    0.0,
                ),
                ..default()
            },
            Star {},
        ));
    }
}
