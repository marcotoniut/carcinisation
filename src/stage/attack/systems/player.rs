use bevy::prelude::*;
use seldom_pixel::prelude::*;

use crate::{
    stage::{
        attack::components::*,
        components::interactive::{Collision, CollisionData, Hittable},
        events::DamageEvent,
        player::components::{
            PlayerAttack, UnhittableList, Weapon, ATTACK_GUN_DAMAGE, ATTACK_PINCER_DAMAGE,
        },
        score::components::Score,
    },
    systems::camera::CameraPos,
};

/**
 * Could split between box and circle collision
 */
pub fn check_got_hit(
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    mut event_writer: EventWriter<DamageEvent>,
    mut attack_query: Query<(&PlayerAttack, &mut UnhittableList)>,
    mut hittable_query: Query<(Entity, &PxSubPosition, &CollisionData), With<Hittable>>,
    mut score: ResMut<Score>,
) {
    let camera_pos = camera_query.get_single().unwrap();
    for (attack, mut hit_list) in attack_query.iter_mut() {
        for (entity, position, collision_data) in hittable_query.iter_mut() {
            if hit_list.0.contains(&entity) == false {
                hit_list.0.insert(entity);
                let attack_position = camera_pos.0 + attack.position - collision_data.offset;
                match attack.weapon {
                    Weapon::Pincer => match collision_data.collision {
                        Collision::Circle(radius) => {
                            let distance = attack_position.distance(position.0);
                            if distance < radius {
                                if distance * 2.5 < radius {
                                    event_writer
                                        .send(DamageEvent::new(entity, ATTACK_PINCER_DAMAGE * 2));
                                    score.add_u(SCORE_MELEE_CRITICAL_HIT);
                                    info!("Entity got hit by Pincer! ***CRITICAL***");
                                } else {
                                    event_writer
                                        .send(DamageEvent::new(entity, ATTACK_PINCER_DAMAGE));
                                    score.add_u(SCORE_MELEE_REGULAR_HIT);
                                    info!("Entity got hit by Pincer!");
                                }
                            }
                        }
                        Collision::Box(v) => {
                            let distance = attack_position.distance(position.0);
                            if distance < v.x && distance < v.y {
                                if distance * 2.5 < v.x && distance * 2.5 < v.y {
                                    event_writer
                                        .send(DamageEvent::new(entity, ATTACK_PINCER_DAMAGE * 2));
                                    score.add_u(SCORE_MELEE_CRITICAL_HIT);
                                    info!("Entity got hit by Pincer! ***CRITICAL***");
                                } else {
                                    event_writer
                                        .send(DamageEvent::new(entity, ATTACK_PINCER_DAMAGE));
                                    score.add_u(SCORE_MELEE_REGULAR_HIT);
                                    info!("Entity got hit by Pincer!");
                                }
                            }
                        }
                    },
                    Weapon::Gun => match collision_data.collision {
                        Collision::Circle(radius) => {
                            let distance = attack_position.distance(position.0);
                            if distance < radius {
                                if distance * 2.5 < radius {
                                    event_writer
                                        .send(DamageEvent::new(entity, ATTACK_GUN_DAMAGE * 2));
                                    score.add_u(SCORE_RANGED_CRITICAL_HIT);
                                    info!("Entity got hit by Gun! ***CRITICAL***");
                                } else {
                                    event_writer.send(DamageEvent::new(entity, ATTACK_GUN_DAMAGE));
                                    score.add_u(SCORE_RANGED_REGULAR_HIT);
                                    info!("Entity got hit by Gun!");
                                }
                            }
                        }
                        Collision::Box(v) => {
                            let distance = attack_position.distance(position.0);
                            if distance < v.x && distance < v.y {
                                if distance * 2.5 < v.x && distance * 2.5 < v.y {
                                    event_writer
                                        .send(DamageEvent::new(entity, ATTACK_GUN_DAMAGE * 2));
                                    score.add_u(SCORE_RANGED_CRITICAL_HIT);
                                    info!("Entity got hit by Gun! ***CRITICAL***");
                                } else {
                                    event_writer.send(DamageEvent::new(entity, ATTACK_GUN_DAMAGE));
                                    score.add_u(SCORE_RANGED_REGULAR_HIT);
                                    info!("Entity got hit by Gun!");
                                }
                            }
                        }
                    },
                }
            }
        }
    }
}
