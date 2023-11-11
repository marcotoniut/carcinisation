use bevy::prelude::*;
use seldom_pixel::prelude::*;

use crate::{
    game::score::components::Score,
    stage::{
        attack::components::*,
        components::{
            interactive::{Collision, CollisionData, CollisionShape, Hittable},
            placement::Depth,
        },
        events::DamageEvent,
        player::components::{
            PlayerAttack, UnhittableList, Weapon, ATTACK_GUN_DAMAGE, ATTACK_PINCER_DAMAGE,
        },
    },
    systems::camera::CameraPos,
};

const CRITICAL_THRESHOLD: f32 = 0.5;

/**
 * Could split between box and circle collision
 */
pub fn check_got_hit(
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    mut event_writer: EventWriter<DamageEvent>,
    mut attack_query: Query<(&PlayerAttack, &mut UnhittableList)>,
    mut hittable_query: Query<(Entity, &PxSubPosition, &CollisionData, &Depth), With<Hittable>>,
    mut score: ResMut<Score>,
) {
    let camera_pos = camera_query.get_single().unwrap();
    for (attack, mut hit_list) in attack_query.iter_mut() {
        for (entity, position, collision_data, depth) in hittable_query.iter_mut() {
            if hit_list.0.contains(&entity) == false {
                hit_list.0.insert(entity);

                let attack_position = camera_pos.0 + attack.position;
                match attack.weapon {
                    Weapon::Pincer => {
                        if let Some(collision) =
                            collision_data.point_collides(position.0, attack_position)
                        {
                            event_writer.send(DamageEvent::new(
                                entity,
                                (ATTACK_PINCER_DAMAGE as f32 / collision.defense) as u32,
                            ));
                            if collision.defense <= CRITICAL_THRESHOLD {
                                score.add_u(SCORE_MELEE_CRITICAL_HIT);
                                info!("Entity got hit by Pincer! ***CRITICAL***");
                            } else {
                                score.add_u(SCORE_MELEE_REGULAR_HIT);
                                info!("Entity got hit by Pincer!");
                            }
                        }
                    }
                    Weapon::Gun => {
                        if let Some(collision) =
                            collision_data.point_collides(position.0, attack_position)
                        {
                            event_writer.send(DamageEvent::new(
                                entity,
                                (ATTACK_GUN_DAMAGE as f32 / collision.defense) as u32,
                            ));
                            if collision.defense <= CRITICAL_THRESHOLD {
                                score.add_u(SCORE_RANGED_CRITICAL_HIT);
                                info!("Entity got hit by Gun! ***CRITICAL***");
                            } else {
                                score.add_u(SCORE_RANGED_REGULAR_HIT);
                                info!("Entity got hit by Gun!");
                            }
                        }
                    }
                }
            }
        }
    }
}
