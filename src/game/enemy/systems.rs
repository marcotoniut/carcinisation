use bevy::{audio::*, prelude::*, window::PrimaryWindow};
use seldom_pixel::{asset::*, prelude::*, sprite::*};

use crate::Layer;

use super::{components::*, resources::*};

fn make_enemy_bundle(
    window: &Window,
    texture: Handle<PxAsset<PxSpriteData>>,
) -> (PxSpriteBundle<Layer>, Enemy) {
    return (
        PxSpriteBundle::<Layer> {
            sprite: texture.clone(),

            position: PxPosition(IVec2::new(
                rand::random::<i32>() % window.width() as i32,
                rand::random::<i32>() % window.height() as i32,
            )),

            // sprite: Sprite {
            //     custom_size: Some(Vec2::new(ENEMY_SIZE, ENEMY_SIZE)),
            //     ..default()
            // },
            // transform: Transform::from_xyz(
            //     rand::random::<f32>() * window.width(),
            //     rand::random::<f32>() * window.height(),
            //     0.0,
            // ),
            ..default()
        },
        Enemy {
            direction: Vec2::new(rand::random::<f32>(), rand::random::<f32>()).normalize(),
        },
    );
}

pub fn spawn_enemies(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
) {
    let window: &Window = window_query.get_single().unwrap();

    // let texture = asset_server.load("sprites/ball_red_large.png");
    let texture = asset_server.load("sprites/mage.png");

    for _ in 0..NUMBER_OF_ENEMIES {
        commands.spawn(make_enemy_bundle(window, texture.clone()));
    }
}

pub fn despawn_enemies(mut commands: Commands, query: Query<Entity, With<Enemy>>) {
    for entity in &mut query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn enemy_movement(mut query: Query<(&mut Transform, &Enemy)>, time: Res<Time>) {
    for (mut transform, enemy) in &mut query {
        let direction = Vec3::new(enemy.direction.x, enemy.direction.y, 0.0);
        transform.translation += direction * ENEMY_SPEED * time.delta_seconds();
    }
}

pub fn update_enemy_direction(
    mut query: Query<(&mut Transform, &mut Enemy)>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let window: &Window = window_query.get_single().unwrap();

    let half_player_size = ENEMY_SIZE / 2.0;
    let x_min = 0.0 + half_player_size;
    let x_max = window.width() - half_player_size;
    let y_min = 0.0 + half_player_size;
    let y_max = window.height() - half_player_size;

    for (transform, mut enemy) in &mut query {
        let translation = transform.translation;

        let mut direction_changed = false;

        if translation.x <= x_min || translation.x >= x_max {
            enemy.direction.x *= -1.0;
            direction_changed = true;
        }

        if translation.y <= y_min || translation.y >= y_max {
            enemy.direction.y *= -1.0;
            direction_changed = true;
        }

        if direction_changed {
            let sound_effect_1 = asset_server.load("audio/pluck_001.ogg");
            let sound_effect_2 = asset_server.load("audio/pluck_001.ogg");
            let sound_effect = if rand::random::<f32>() > 0.5 {
                sound_effect_1
            } else {
                sound_effect_2
            };
            commands.spawn(AudioBundle {
                source: sound_effect,
                settings: PlaybackSettings {
                    mode: PlaybackMode::Despawn,
                    volume: Volume::new_relative(0.02),
                    ..default()
                },
                ..default()
            });
        }
    }
}

pub fn confine_enemy_movement(
    mut enemy_query: Query<&mut Transform, With<Enemy>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    let window: &Window = window_query.get_single().unwrap();

    let half_enemy_size = ENEMY_SIZE / 2.0;
    let x_min = 0.0 + half_enemy_size;
    let x_max = window.width() - half_enemy_size;
    let y_min = 0.0 + half_enemy_size;
    let y_max = window.height() - half_enemy_size;

    for mut transform in &mut enemy_query {
        let mut translation = transform.translation;

        if translation.x < x_min {
            translation.x = x_min;
        } else if translation.x > x_max {
            translation.x = x_max;
        }

        if translation.y < y_min {
            translation.y = y_min;
        } else if translation.y > y_max {
            translation.y = y_max;
        }

        transform.translation = translation;
    }
}

pub fn tick_enemy_spawn_timer(mut enemy_spawn_timer: ResMut<EnemySpawnTimer>, time: Res<Time>) {
    enemy_spawn_timer.timer.tick(time.delta());
}

pub fn spawn_enemies_over_time(
    mut commands: Commands,
    enemy_spawn_timer: Res<EnemySpawnTimer>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
) {
    if enemy_spawn_timer.timer.finished() {
        let window: &Window = window_query.get_single().unwrap();
        let texture = asset_server.load("sprites/ball_red_large.png");
        commands.spawn(make_enemy_bundle(window, texture.clone()));
    }
}
