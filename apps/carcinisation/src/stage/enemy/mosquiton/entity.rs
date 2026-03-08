use crate::stage::{
    components::{
        StageEntity,
        interactive::{ColliderData, Flickerer, Health, Hittable},
        placement::{Depth, Speed},
    },
    enemy::{
        components::{Enemy, behavior::EnemyBehaviors},
        composed::ComposedEnemyVisual,
        entity::EnemyType,
        mosquito::entity::{ENEMY_MOSQUITO_BASE_HEALTH, EnemyMosquito, EnemyMosquitoAttacking},
    },
};
use bevy::prelude::*;
use seldom_pixel::position::PxSubPosition;

#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct EnemyMosquiton;

impl EnemyMosquiton {
    #[must_use]
    pub fn kill_score(&self) -> u32 {
        10
    }
}

#[derive(Clone, Component, Debug, PartialEq, Eq, Reflect)]
pub enum EnemyMosquitonAnimation {
    IdleStand,
    ShootStand,
}

#[derive(Bundle, Debug)]
pub struct MosquitonDefaultBundle {
    pub enemy: Enemy,
    pub enemy_mosquito: EnemyMosquito,
    pub enemy_mosquito_attacking: EnemyMosquitoAttacking,
    pub enemy_mosquiton: EnemyMosquiton,
    pub flickerer: Flickerer,
    pub name: Name,
    pub health: Health,
    pub hittable: Hittable,
    pub stage_entity: StageEntity,
}

impl Default for MosquitonDefaultBundle {
    fn default() -> Self {
        Self {
            enemy: Enemy,
            enemy_mosquito: EnemyMosquito,
            enemy_mosquito_attacking: EnemyMosquitoAttacking::default(),
            enemy_mosquiton: EnemyMosquiton,
            flickerer: Flickerer,
            name: EnemyType::Mosquiton.get_name(),
            health: Health(ENEMY_MOSQUITO_BASE_HEALTH),
            hittable: Hittable,
            stage_entity: StageEntity,
        }
    }
}

#[derive(Bundle, Debug)]
pub struct MosquitonBundle {
    pub behaviors: EnemyBehaviors,
    pub collider_data: ColliderData,
    pub composed_visual: ComposedEnemyVisual,
    pub depth: Depth,
    pub position: PxSubPosition,
    pub speed: Speed,
    pub default: MosquitonDefaultBundle,
}
