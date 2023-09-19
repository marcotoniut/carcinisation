use bevy::{audio::*, prelude::*};
use seldom_pixel::{asset::*, prelude::*, sprite::PxSpriteData};

use crate::{
    globals::*,
    stage::player::components::{PlayerAttack, Weapon},
};

use super::{bundles::*, components::*, resources::*};

pub fn spawn_enemies(mut commands: Commands, mut assets_sprite: PxAssets<PxSprite>) {
    for _ in 0..NUMBER_OF_ENEMIES {
        commands.spawn(make_enemy_bundle(&mut assets_sprite));
    }
}

pub fn despawn_enemies(mut commands: Commands, query: Query<Entity, With<Enemy>>) {
    for entity in &mut query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn enemy_movement(mut query: Query<(&mut PxSubPosition, &Enemy)>, time: Res<Time>) {
    for (mut position, enemy) in &mut query {
        let direction = Vec2::new(enemy.direction.x, enemy.direction.y);
        position.0 += direction * ENEMY_SPEED * time.delta_seconds();
    }
}

pub fn update_enemy_direction(
    mut query: Query<(&mut PxSubPosition, &mut Enemy)>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let half_size = ENEMY_SIZE / 2.0;
    let x_min = half_size;
    let x_max = SCREEN_RESOLUTION.x as f32 - half_size;
    let y_min = HUD_HEIGHT as f32 + half_size;
    let y_max = SCREEN_RESOLUTION.y as f32 - half_size;

    for (position, mut enemy) in &mut query {
        let translation = position.0;

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

pub fn confine_enemy_movement(mut enemy_query: Query<&mut PxSubPosition, With<Enemy>>) {
    let half_size = ENEMY_SIZE / 2.0;
    let x_min = half_size;
    let x_max = SCREEN_RESOLUTION.x as f32 - half_size;
    let y_min = HUD_HEIGHT as f32 + half_size;
    let y_max = SCREEN_RESOLUTION.y as f32 - half_size;

    for mut position in &mut enemy_query {
        let mut translation = position.0;

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

        position.0 = translation;
    }
}

pub fn check_enemy_got_hit(
    attack_query: Query<&PlayerAttack>,
    enemy_query: Query<(Entity, &Enemy, &Handle<PxAsset<PxSpriteData>>)>,
) {
    for attack in &mut attack_query.iter() {
        for (entity, enemy, data) in &mut enemy_query.iter() {
            match attack.weapon {
                Weapon::Pincer => {
                    let distance = 1;
                    // let x = data.get_field("").unwrap();
                }
                Weapon::Gun => {
                    let distance = 1;
                }
            }
        }
    }
}

pub fn tick_enemy_spawn_timer(mut timer: ResMut<EnemySpawnTimer>, time: Res<Time>) {
    timer.timer.tick(time.delta());
}

pub fn spawn_enemies_over_time(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    enemy_spawn_timer: Res<EnemySpawnTimer>,
) {
    if enemy_spawn_timer.timer.finished() {
        commands.spawn(make_enemy_bundle(&mut assets_sprite));
    }
}
