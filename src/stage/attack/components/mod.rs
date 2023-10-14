use bevy::prelude::*;

use super::data::{
    blood_shot::BLOOD_ATTACK_ANIMATIONS, boulder_throw::BOULDER_ATTACK_ANIMATIONS,
    HoveringAttackAnimations,
};

pub mod bundles;

pub const SCORE_RANGED_REGULAR_HIT: u32 = 1;
pub const SCORE_RANGED_CRITICAL_HIT: u32 = 4;
pub const SCORE_MELEE_REGULAR_HIT: u32 = 3;
pub const SCORE_MELEE_CRITICAL_HIT: u32 = 10;

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