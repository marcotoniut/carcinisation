pub mod behavior;

use std::collections::VecDeque;
use std::time::Duration;

use bevy::prelude::*;

use crate::plugins::movement::structs::MovementDirection;
use crate::stage::data::EnemyStep;

use super::data::blood_attack::BLOOD_ATTACK_ANIMATIONS;
use super::data::boulder_attack::BOULDER_ATTACK_ANIMATIONS;
use super::data::HoveringAttackAnimations;

pub const BLOOD_ATTACK_DEPTH_SPEED: f32 = 4.;
pub const BLOOD_ATTACK_LINE_SPEED: f32 = 25.;
pub const BLOOD_ATTACK_DAMAGE: u32 = 20;

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct EnemyAttack;

#[derive(Component)]
pub enum EnemyHoveringAttackType {
    BloodShot,
    BoulderThrow,
}

impl EnemyHoveringAttackType {
    pub fn get_name(&self) -> String {
        match self {
            EnemyHoveringAttackType::BloodShot => "Blood Shot".to_string(),
            EnemyHoveringAttackType::BoulderThrow => "Boulder Throw".to_string(),
        }
    }

    pub fn get_animations(&self) -> &'static HoveringAttackAnimations {
        match self {
            EnemyHoveringAttackType::BloodShot => &BLOOD_ATTACK_ANIMATIONS,
            EnemyHoveringAttackType::BoulderThrow => &BOULDER_ATTACK_ANIMATIONS,
        }
    }
}

#[derive(Component, Clone, Debug)]
pub struct CircleAround {
    pub radius: f32,
    pub center: Vec2,
    pub time_offset: f32,
    pub direction: MovementDirection,
}

#[derive(Component, Clone, Debug)]
pub struct LinearMovement {
    pub direction: Vec2,
    pub trayectory: f32,
}

// Enemies

pub const ENEMY_MOSQUITO_RADIUS: f32 = 7.0;
pub const ENEMY_MOSQUITO_BASE_HEALTH: u32 = 40;

pub const ENEMY_TARDIGRADE_RADIUS: f32 = 9.0;
pub const ENEMY_TARDIGRADE_BASE_HEALTH: u32 = 240;

#[derive(Component, Clone, Debug)]
pub struct EnemyMosquito {
    pub steps: VecDeque<EnemyStep>,
    // pub state: EnemyMosquitoState,
}

impl EnemyMosquito {
    pub fn kill_score(&self) -> u32 {
        10
    }
}

#[derive(Clone, Component, Debug, Default)]
pub struct EnemyMosquitoAttacking {
    pub attack: Option<EnemyMosquitoAttack>,
    pub last_attack_started: Duration,
}

#[derive(Clone, Component, Debug)]
pub enum EnemyMosquitoAttack {
    Ranged,
    Melee,
}

// TODO review
#[derive(Component, Clone, Debug)]
pub enum EnemyMosquitoAnimation {
    Idle,
    Attack,
    Fly,
}

#[derive(Clone, Component, Debug, Default)]
pub struct CurrentEnemyMosquitoStep(EnemyStep);

// Tardigrade
#[derive(Component, Clone, Debug)]
pub struct EnemyTardigrade {
    pub steps: VecDeque<EnemyStep>,
}

impl EnemyTardigrade {
    pub fn kill_score(&self) -> u32 {
        7
    }
}

// TODO review
#[derive(Component, Clone, Debug)]
pub enum EnemyTardigradeAnimation {
    Idle,
    Attack,
    Sucking,
}

#[derive(Clone, Component, Debug, Default)]
pub struct EnemyTardigradeAttacking {
    pub attack: bool,
    pub last_attack_started: Duration,
}

#[derive(Component)]
pub struct EnemySpidey {}

// Bosses

#[derive(Component)]
pub struct EnemyMarauder {}

#[derive(Component)]
pub struct EnemySpidomonsta {}

#[derive(Component)]
pub struct EnemyKyle {}
