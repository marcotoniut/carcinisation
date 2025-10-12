pub mod bundles;

use super::data::{
    blood_shot::BLOOD_ATTACK_ANIMATIONS, boulder_throw::BOULDER_ATTACK_ANIMATIONS,
    HoveringAttackAnimations,
};
use crate::stage::components::placement::Depth;
use bevy::prelude::*;

pub const SCORE_RANGED_REGULAR_HIT: u32 = 1;
pub const SCORE_RANGED_CRITICAL_HIT: u32 = 4;
pub const SCORE_MELEE_REGULAR_HIT: u32 = 3;
pub const SCORE_MELEE_CRITICAL_HIT: u32 = 10;

#[derive(Component, Default)]
pub struct EnemyAttack;

#[derive(Component, Debug, Reflect)]
pub enum EnemyHoveringAttackType {
    BloodShot,
    BoulderThrow,
}

#[derive(Component, Debug, Reflect)]
pub struct EnemyAttackOriginPosition(pub Vec2);

// TODO this should impact damage
// (but it should also be affected by the stage's environment)
#[derive(Component, Debug, Reflect)]
pub struct EnemyAttackOriginDepth(pub Depth);

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
