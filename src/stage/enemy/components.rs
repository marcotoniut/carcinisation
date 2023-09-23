use bevy::prelude::*;

use crate::stage::data::EnemyStep;

pub const SCORE_RANGED_REGULAR_HIT: u32 = 1;
pub const SCORE_RANGED_CRITICAL_HIT: u32 = 4;
pub const SCORE_MELEE_REGULAR_HIT: u32 = 3;
pub const SCORE_MELEE_CRITICAL_HIT: u32 = 10;

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

#[derive(Component, Clone, Debug)]
pub struct EnemyMosquito {
    pub base_speed: f32,
    pub steps: Vec<EnemyStep>,
    // pub state: EnemyMosquitoState,
}

impl EnemyMosquito {
    pub fn kill_score(&self) -> u32 {
        10
    }

    pub fn current_step(&self) -> &EnemyStep {
        // TODO temporary
        self.steps
            .first()
            .unwrap_or(&EnemyStep::Idle { duration: 999. })
    }
}

#[derive(Component, Clone, Debug)]
pub struct EnemyMosquitoAttacking {
    pub attack: Option<EnemyMosquitoAttack>,
}

#[derive(Component, Clone, Debug)]
pub enum EnemyMosquitoAttack {
    Ranged,
    Melee,
}

#[derive(Component, Clone, Debug)]
pub enum EnemyMosquitoAnimation {
    Idle,
    Attack,
    Movement,
    Circle,
}

#[derive(Clone, Component, Debug, Default)]
pub struct CurrentEnemyMosquitoStep(EnemyStep);

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
