pub mod behaviors;
pub mod damage;
pub mod mosquito;
pub mod tardigrade;

use bevy::prelude::*;
use seldom_pixel::{asset::*, prelude::*};

use crate::stage::{
    components::{interactive::Dead, SpawnDrop},
    data::ContainerSpawn,
    player::components::{PlayerAttack, UnhittableList},
    systems::spawn::{spawn_enemy, spawn_pickup},
};

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
