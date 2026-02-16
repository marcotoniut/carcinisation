use std::time::Duration;

use crate::stage::{
    components::{
        StageEntity,
        interactive::{ColliderData, Flickerer, Health, Hittable},
        placement::{Depth, Speed},
    },
    enemy::{
        components::{Enemy, behavior::EnemyBehaviors},
        entity::EnemyType,
    },
};
use bevy::prelude::*;
use derive_new::new;
use seldom_pixel::position::PxSubPosition;

pub const ENEMY_TARDIGRADE_RADIUS: f32 = 9.0;
pub const ENEMY_TARDIGRADE_BASE_HEALTH: u32 = 240;

// Tardigrade
#[derive(Clone, Component, Debug, Reflect)]
pub struct EnemyTardigrade;

impl EnemyTardigrade {
    pub fn kill_score(&self) -> u32 {
        7
    }
}

// TODO review
#[derive(Clone, Component, Debug, Reflect)]
pub enum EnemyTardigradeAnimation {
    Idle,
    Attack,
    Sucking,
}

// TODO could generalise
#[derive(new, Clone, Component, Debug, Default, Reflect)]
pub struct EnemyTardigradeAttacking {
    #[new(default)]
    pub attack: bool,
    #[new(default)]
    pub last_attack_started: Duration,
}

#[derive(Bundle, Debug)]
pub struct TardigradeDefaultBundle {
    pub enemy: Enemy,
    pub enemy_type: EnemyTardigrade,
    pub enemy_type_attacking: EnemyTardigradeAttacking,
    pub flickerer: Flickerer,
    pub name: Name,
    pub health: Health,
    pub hittable: Hittable,
    pub stage_entity: StageEntity,
}

impl Default for TardigradeDefaultBundle {
    fn default() -> Self {
        Self {
            enemy: Enemy,
            enemy_type: EnemyTardigrade,
            enemy_type_attacking: EnemyTardigradeAttacking::new(),
            flickerer: Flickerer,
            health: Health(ENEMY_TARDIGRADE_BASE_HEALTH),
            hittable: Hittable,
            name: EnemyType::Tardigrade.get_name(),
            stage_entity: StageEntity,
        }
    }
}

#[derive(Bundle, Debug)]
pub struct TardigradeBundle {
    pub behaviors: EnemyBehaviors,
    pub collider_data: ColliderData,
    pub depth: Depth,
    pub position: PxSubPosition,
    pub speed: Speed,
    pub default: TardigradeDefaultBundle,
}
