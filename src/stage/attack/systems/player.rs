use bevy::prelude::*;
use seldom_pixel::prelude::*;

use crate::{
    game::score::components::Score,
    stage::{
        attack::components::*,
        components::{
            interactive::{Collider, ColliderData, ColliderShape, Hittable},
            placement::Depth,
        },
        events::DamageEvent,
        player::components::{
            PlayerAttack, UnhittableList, Weapon, ATTACK_GUN_DAMAGE, ATTACK_PINCER_DAMAGE,
        },
    },
    systems::camera::CameraPos,
};
use colored::*;

const CRITICAL_THRESHOLD: f32 = 0.5;

/**
 * Could split between box and circle collider
 */
pub fn check_got_hit(
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    mut event_writer: EventWriter<DamageEvent>,
    mut attack_query: Query<(&PlayerAttack, &mut UnhittableList)>,
    // mut attack_query: Query<(&PlayerAttack, &mut UnhittableList, Option<&Reach>)>,
    mut hittable_query: Query<(Entity, &PxSubPosition, &ColliderData, &Depth), With<Hittable>>,
    mut score: ResMut<Score>,
) {
    let camera_pos = camera_query.get_single().unwrap();
    for (attack, mut hit_list) in attack_query.iter_mut() {
        for (entity, position, collider_data, depth) in hittable_query.iter_mut() {
            if hit_list.0.contains(&entity) == false {
                hit_list.0.insert(entity);

                let attack_position = camera_pos.0 + attack.position;
                match attack.weapon {
                    Weapon::Pincer => {
                        if let Some(collider) =
                            collider_data.point_collides(position.0, attack_position)
                        {
                            event_writer.send(DamageEvent::new(
                                entity,
                                (ATTACK_PINCER_DAMAGE as f32 / collider.defense) as u32,
                            ));
                            if collider.defense <= CRITICAL_THRESHOLD {
                                score.add_u(SCORE_MELEE_CRITICAL_HIT);

                                #[cfg(debug_assertions)]
                                println!("{} Pincer ***CRITICAL***", "HIT".yellow());
                            } else {
                                score.add_u(SCORE_MELEE_REGULAR_HIT);

                                #[cfg(debug_assertions)]
                                println!("{} Pincer", "HIT".yellow());
                            }
                        }
                    }
                    Weapon::Gun => {
                        if let Some(collider) =
                            collider_data.point_collides(position.0, attack_position)
                        {
                            event_writer.send(DamageEvent::new(
                                entity,
                                (ATTACK_GUN_DAMAGE as f32 / collider.defense) as u32,
                            ));
                            if collider.defense <= CRITICAL_THRESHOLD {
                                score.add_u(SCORE_RANGED_CRITICAL_HIT);

                                #[cfg(debug_assertions)]
                                println!("{} Gun ***CRITICAL***", "HIT".yellow());
                            } else {
                                score.add_u(SCORE_RANGED_REGULAR_HIT);

                                #[cfg(debug_assertions)]
                                println!("{} Gun", "HIT".yellow());
                            }
                        }
                    }
                }
            }
        }
    }
}
