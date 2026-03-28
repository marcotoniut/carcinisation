pub mod bundles;

use super::data::{
    HoveringAttackAnimations, blood_shot::BLOOD_ATTACK_ANIMATIONS,
    boulder_throw::BOULDER_ATTACK_ANIMATIONS,
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
#[reflect(Component)]
pub enum EnemyHoveringAttackType {
    BloodShot,
    BoulderThrow,
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct EnemyAttackOriginPosition(pub Vec2);

// TODO this should impact damage
// (but it should also be affected by the stage's environment)
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct EnemyAttackOriginDepth(pub Depth);

/// BRP/debug-friendly snapshot of a live attack's world-space position data.
///
/// `PxSubPosition` is not reflectable, so debug builds mirror the current
/// center position here to make exact projectile-vs-cue comparisons possible.
#[cfg(debug_assertions)]
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct EnemyAttackDebugPosition {
    pub current: Vec2,
    pub origin: Vec2,
}

impl EnemyHoveringAttackType {
    #[must_use]
    pub fn get_name(&self) -> String {
        match self {
            EnemyHoveringAttackType::BloodShot => "Blood Shot".to_string(),
            EnemyHoveringAttackType::BoulderThrow => "Boulder Throw".to_string(),
        }
    }

    #[must_use]
    pub fn get_animations(&self) -> &'static HoveringAttackAnimations {
        match self {
            EnemyHoveringAttackType::BloodShot => &BLOOD_ATTACK_ANIMATIONS,
            EnemyHoveringAttackType::BoulderThrow => &BOULDER_ATTACK_ANIMATIONS,
        }
    }
}
