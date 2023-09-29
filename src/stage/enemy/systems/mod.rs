pub mod attacks;
pub mod behaviors;
pub mod mosquito;
pub mod tardigrade;

use super::components::*;
use bevy::prelude::*;
use seldom_pixel::{asset::*, prelude::*};

use crate::{
    stage::{
        components::{Collision, Dead, Health, Hittable, SpawnDrop},
        data::ContainerSpawn,
        player::components::{
            PlayerAttack, UnhittableList, Weapon, ATTACK_GUN_DAMAGE, ATTACK_PINCER_DAMAGE,
        },
        score::components::Score,
        systems::spawn::{spawn_enemy, spawn_pickup},
    },
    systems::camera::CameraPos,
};

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

// DEPRECATED
// pub fn placeholder_tick_enemy_spawn_timer(mut timer: ResMut<EnemySpawnTimer>, time: Res<Time>) {
//     timer.timer.tick(time.delta());
// }

// pub fn placeholder_spawn_enemies_over_time(
//     mut commands: Commands,
//     mut assets_sprite: PxAssets<PxSprite>,
//     enemy_spawn_timer: Res<EnemySpawnTimer>,
// ) {
//     if enemy_spawn_timer.timer.finished() {
//         commands.spawn(make_enemy_bundle(&mut assets_sprite));
//     }
// }

pub fn check_dead_drop(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut attack_query: Query<&mut UnhittableList, With<PlayerAttack>>,
    query: Query<(&SpawnDrop, &PxSubPosition), Added<Dead>>,
) {
    for (spawn_drop, position) in &mut query.iter() {
        let entity = match spawn_drop.contains.clone() {
            ContainerSpawn::Pickup(mut spawn) => {
                spawn.coordinates = position.0;
                spawn_pickup(&mut commands, &mut assets_sprite, Vec2::ZERO, &spawn)
            }
            ContainerSpawn::Enemy(mut spawn) => {
                spawn.coordinates = position.0;
                spawn_enemy(&mut commands, Vec2::ZERO, &spawn)
            }
        };

        for mut unhittable_list in &mut attack_query.iter_mut() {
            if unhittable_list.0.contains(&spawn_drop.entity) {
                unhittable_list.0.insert(entity);
            }
        }
    }
}
