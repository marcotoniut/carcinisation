pub mod behavior;

use super::data::steps::EnemyStep;
use crate::plugins::movement::structs::MovementDirection;
use bevy::prelude::*;
use std::collections::VecDeque;
use std::time::Duration;

#[derive(Component)]
pub struct Enemy;

#[derive(Component, Clone, Debug, Reflect)]
pub struct CircleAround {
    pub radius: f32,
    pub center: Vec2,
    pub time_offset: f32,
    pub direction: MovementDirection,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct LinearMovement {
    pub direction: Vec2,
    pub trayectory: f32,
    // TODO replace with LinearMovement2DReached
    pub reached_x: bool,
    pub reached_y: bool,
}

// Enemies

pub const ENEMY_MOSQUITO_RADIUS: f32 = 7.0;
pub const ENEMY_MOSQUITO_BASE_HEALTH: u32 = 40;

pub const ENEMY_TARDIGRADE_RADIUS: f32 = 9.0;
pub const ENEMY_TARDIGRADE_BASE_HEALTH: u32 = 240;

#[derive(Component, Clone, Debug, Reflect)]
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
#[derive(Clone, Component, Debug, Default, Reflect)]
pub struct EnemyTardigradeAttacking {
    pub attack: bool,
    pub last_attack_started: Duration,
}

impl EnemyTardigradeAttacking {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

#[derive(Component)]
pub struct EnemySpidey;

// Bosses

#[derive(Component)]
pub struct EnemyMarauder;

#[derive(Component)]
pub struct EnemySpidomonsta {}

#[derive(Component)]
pub struct EnemyKyle {}
