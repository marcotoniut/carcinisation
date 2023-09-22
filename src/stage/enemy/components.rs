use bevy::prelude::*;

use crate::stage::data::EnemyStep;

pub const PLACEHOLDER_ENEMY_SPEED: f32 = 10.0;

pub const PLACEHOLDER_ENEMY_SIZE: f32 = 6.0;
pub const PLACEHOLDER_NUMBER_OF_ENEMIES: usize = 2;

pub const PLACEHOLDER_ENEMY_SPAWN_TIME: f32 = 8.0;

#[derive(Component)]
pub struct PlaceholderEnemy {
    pub direction: Vec2,
}

// #[derive(Component)]
// pub struct LayerPlacement {}

#[derive(Component)]
pub struct Enemy {}

// Enemies

pub const ENEMY_MOSQUITO_RADIUS: f32 = 7.0;
pub const ENEMY_MOSQUITO_BASE_HEALTH: u32 = 40;

pub const ENEMY_MOSQUITO_IDLE_FRAMES: usize = 3;
pub const ENEMY_MOSQUITO_IDLE_ANIMATION_SPEED: u64 = 500;
pub const PATH_SPRITES_ENEMY_MOSQUITO_IDLE_1: &str = "sprites/enemies/mosquito_idle_1.png";

#[derive(Component, Debug, Clone)]
pub struct EnemyMosquito {
    pub base_speed: f32,
    pub steps: Vec<EnemyStep>,
}

impl EnemyMosquito {
    pub fn kill_score(&self) -> u32 {
        10
    }
}

#[derive(Debug, Clone)]
pub enum EnemyInstance {
    EnemyMosquito(EnemyMosquito),
}

#[derive(Component)]
pub struct EnemySpidey {}

#[derive(Component)]
pub struct EnemyTardigrade {}

// Bosses

#[derive(Component)]
pub struct EnemyMarauder {}

#[derive(Component)]
pub struct EnemySpidomonsta {}

#[derive(Component)]
pub struct EnemyKyle {}
