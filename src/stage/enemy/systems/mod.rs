pub mod mosquito;

use bevy::{audio::*, prelude::*};
use seldom_pixel::{asset::*, prelude::*, sprite::PxSpriteData};

use crate::{
    globals::*,
    stage::{
        components::{Collision, Dead, Health, Hittable, SpawnDrop},
        data::ContainerSpawn,
        player::components::{
            PlayerAttack, UnhittableList, Weapon, ATTACK_GUN_DAMAGE, ATTACK_PINCER_DAMAGE,
        },
        score::components::Score,
        systems::spawn::{spawn_enemy, spawn_pickup},
    },
    systems::audio::{AudioSystemBundle, AudioSystemType, VolumeSettings},
    systems::camera::CameraPos,
};

use super::{bundles::*, components::*, resources::*};

pub fn spawn_enemies(mut commands: Commands, mut assets_sprite: PxAssets<PxSprite>) {
    for _ in 0..PLACEHOLDER_NUMBER_OF_ENEMIES {
        commands.spawn(make_enemy_bundle(&mut assets_sprite));
    }
}

pub fn despawn_enemies(mut commands: Commands, query: Query<Entity, With<PlaceholderEnemy>>) {
    for entity in &mut query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn enemy_movement(mut query: Query<(&mut PxSubPosition, &PlaceholderEnemy)>, time: Res<Time>) {
    for (mut position, enemy) in &mut query {
        let direction = Vec2::new(enemy.direction.x, enemy.direction.y);
        position.0 += direction * PLACEHOLDER_ENEMY_SPEED * time.delta_seconds();
    }
}

pub fn update_enemy_placeholder_direction(
    mut query: Query<(&mut PxSubPosition, &mut PlaceholderEnemy)>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    volume_settings: Res<VolumeSettings>,
) {
    let half_size = PLACEHOLDER_ENEMY_SIZE / 2.0;
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
            let sound_effect = asset_server.load("audio/sfx/typing_message.ogg");
            // let sound_effect_2 = asset_server.load("audio/pluck_001.ogg");
            // let sound_effect = if rand::random::<f32>() > 0.5 {
            //     sound_effect_1
            // } else {
            //     sound_effect_2
            // };
            let audio = commands
                .spawn(AudioBundle {
                    source: sound_effect,
                    settings: PlaybackSettings {
                        mode: PlaybackMode::Despawn,
                        volume: Volume::new_relative(volume_settings.2 * 1.0),
                        ..default()
                    },
                    ..default()
                })
                .id();
            commands.entity(audio).insert(AudioSystemBundle {
                system_type: AudioSystemType::SFX,
            });
        }
    }
}

pub fn confine_enemy_movement(mut enemy_query: Query<&mut PxSubPosition, With<PlaceholderEnemy>>) {
    let half_size = PLACEHOLDER_ENEMY_SIZE / 2.0;
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

pub fn check_health_at_0(mut commands: Commands, query: Query<(Entity, &Health), Without<Dead>>) {
    for (entity, health) in &mut query.iter() {
        if health.0 == 0 {
            commands.entity(entity).insert(Dead);
        }
    }
}

/**
 * Could split between box and circle collision
 */
pub fn check_got_hit(
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    mut attack_query: Query<(&PlayerAttack, &mut UnhittableList)>,
    mut hittable_query: Query<(Entity, &PxSubPosition, &Collision, &mut Health), With<Hittable>>,
    mut score: ResMut<Score>,
) {
    let camera_pos = camera_query.get_single().unwrap();
    for (attack, mut hit_list) in attack_query.iter_mut() {
        for (entity, position, collision, mut health) in hittable_query.iter_mut() {
            if hit_list.0.contains(&entity) == false {
                hit_list.0.insert(entity);
                let attack_position = camera_pos.0 + attack.position;
                match attack.weapon {
                    Weapon::Pincer => {
                        match collision {
                            Collision::Circle(radius) => {
                                let distance = attack_position.distance(position.0);
                                if distance < *radius {
                                    if distance * 2.5 < *radius {
                                        // TODO organise
                                        score.add_u(SCORE_MELEE_CRITICAL_HIT);
                                        health.0 =
                                            health.0.saturating_sub(ATTACK_PINCER_DAMAGE * 2);
                                        info!("Entity got hit by Pincer! ***CRITICAL***");
                                    } else {
                                        // TODO organise
                                        score.add_u(SCORE_MELEE_REGULAR_HIT);
                                        health.0 = health.0.saturating_sub(ATTACK_PINCER_DAMAGE);
                                        info!("Entity got hit by Pincer!");
                                    }
                                }
                            }
                            Collision::Box(v) => {
                                let distance = attack_position.distance(position.0);
                                if distance < v.x && distance < v.y {
                                    if distance * 2.5 < v.x && distance * 2.5 < v.y {
                                        // TODO organise
                                        score.add_u(SCORE_MELEE_CRITICAL_HIT);
                                        health.0 =
                                            health.0.saturating_sub(ATTACK_PINCER_DAMAGE * 2);
                                        info!("Entity got hit by Pincer! ***CRITICAL***");
                                    } else {
                                        // TODO organise
                                        score.add_u(SCORE_MELEE_REGULAR_HIT);
                                        health.0 = health.0.saturating_sub(ATTACK_PINCER_DAMAGE);
                                        info!("Entity got hit by Pincer!");
                                    }
                                }
                            }
                        }
                    }
                    Weapon::Gun => {
                        match collision {
                            Collision::Circle(radius) => {
                                let distance = attack_position.distance(position.0);
                                if distance < *radius {
                                    if distance * 2.5 < *radius {
                                        // TODO organise
                                        score.add_u(SCORE_RANGED_CRITICAL_HIT);
                                        health.0 = health.0.saturating_sub(ATTACK_GUN_DAMAGE * 2);
                                        info!("Entity got hit by Gun! ***CRITICAL***");
                                    } else {
                                        // TODO organise
                                        score.add_u(SCORE_RANGED_REGULAR_HIT);
                                        health.0 = health.0.saturating_sub(ATTACK_GUN_DAMAGE);
                                        info!("Entity got hit by Gun!");
                                    }
                                }
                            }
                            Collision::Box(v) => {
                                let distance = attack_position.distance(position.0);
                                if distance < v.x && distance < v.y {
                                    if distance * 2.5 < v.x && distance * 2.5 < v.y {
                                        // TODO organise
                                        score.add_u(SCORE_RANGED_CRITICAL_HIT);
                                        health.0 =
                                            health.0.saturating_sub(ATTACK_PINCER_DAMAGE * 2);
                                        info!("Entity got hit by Gun! ***CRITICAL***");
                                    } else {
                                        // TODO organise
                                        score.add_u(SCORE_RANGED_REGULAR_HIT);
                                        health.0 = health.0.saturating_sub(ATTACK_PINCER_DAMAGE);
                                        info!("Entity got hit by Gun!");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn placeholder_tick_enemy_spawn_timer(mut timer: ResMut<EnemySpawnTimer>, time: Res<Time>) {
    timer.timer.tick(time.delta());
}

pub fn placeholder_spawn_enemies_over_time(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    enemy_spawn_timer: Res<EnemySpawnTimer>,
) {
    if enemy_spawn_timer.timer.finished() {
        commands.spawn(make_enemy_bundle(&mut assets_sprite));
    }
}

pub fn check_dead_drop(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut attack_query: Query<(Entity, &PlayerAttack, &mut UnhittableList)>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    query: Query<(&mut SpawnDrop, &PxSubPosition), With<Dead>>,
) {
    let camera_pos = camera_query.get_single().unwrap();

    for (spawn_drop, position) in &mut query.iter() {
        let entity = match spawn_drop.contains.clone() {
            ContainerSpawn::Pickup(mut spawn) => {
                spawn.coordinates = position.0;
                spawn_pickup(&mut commands, &mut assets_sprite, &camera_pos, &spawn)
            }
            ContainerSpawn::Enemy(mut spawn) => {
                spawn.coordinates = position.0;
                spawn_enemy(&mut commands, &camera_pos, &spawn)
            }
        };

        for (attack, mut hit_list, mut unhittable_list) in &mut attack_query.iter_mut() {
            if unhittable_list.0.contains(&spawn_drop.entity) {
                unhittable_list.0.insert(entity);
            }
        }
    }
}
