use bevy::prelude::*;
use seldom_pixel::position::PxSubPosition;
use std::time::Duration;

use crate::stage::{
    components::{
        interactive::{ColliderData, Flickerer, Health, Hittable},
        placement::{Depth, Speed},
        StageEntity,
    },
    enemy::{
        components::{behavior::EnemyBehaviors, Enemy},
        data::steps::EnemyStep,
        entity::EnemyType,
    },
};

pub const ENEMY_MOSQUITO_RADIUS: f32 = 7.0;
pub const ENEMY_MOSQUITO_BASE_HEALTH: u32 = 40;

#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct EnemyMosquito;

impl EnemyMosquito {
    pub fn kill_score(&self) -> u32 {
        10
    }
}

#[derive(Clone, Component, Debug, Default, Reflect)]
pub struct EnemyMosquitoAttacking {
    pub attack: Option<EnemyMosquitoAttack>,
    pub last_attack_started: Duration,
}

impl EnemyMosquitoAttacking {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

#[derive(Clone, Component, Debug, Reflect)]
pub enum EnemyMosquitoAttack {
    Ranged,
    Melee,
}

// TODO review
#[derive(Clone, Component, Debug, Reflect)]
pub enum EnemyMosquitoAnimation {
    Idle,
    Attack,
    Fly,
}

#[derive(Clone, Component, Debug, Default, Reflect)]
pub struct CurrentEnemyMosquitoStep(EnemyStep);

#[derive(Bundle, Debug)]
pub struct MosquitoDefaultBundle {
    pub enemy: Enemy,
    pub enemy_mosquito: EnemyMosquito,
    pub enemy_mosquito_attacking: EnemyMosquitoAttacking,
    pub flickerer: Flickerer,
    pub name: Name,
    pub health: Health,
    pub hittable: Hittable,
    pub stage_entity: StageEntity,
}

impl Default for MosquitoDefaultBundle {
    fn default() -> Self {
        Self {
            enemy: Enemy,
            enemy_mosquito: EnemyMosquito,
            enemy_mosquito_attacking: EnemyMosquitoAttacking::new(),
            flickerer: Flickerer,
            health: Health(ENEMY_MOSQUITO_BASE_HEALTH),
            hittable: Hittable,
            name: EnemyType::Mosquito.get_name(),
            stage_entity: StageEntity,
        }
    }
}

#[derive(Bundle, Debug)]
pub struct MosquitoBundle {
    pub behaviors: EnemyBehaviors,
    pub collider_data: ColliderData,
    pub depth: Depth,
    pub position: PxSubPosition,
    pub speed: Speed,
    pub default: MosquitoDefaultBundle,
}
