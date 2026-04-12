pub mod bundles;

use super::data::{
    HoveringAttackAnimations, blood_shot::BLOOD_ATTACK_ANIMATIONS,
    boulder_throw::BOULDER_ATTACK_ANIMATIONS,
};
use crate::stage::components::placement::Depth;
use bevy::prelude::*;
use carapace::prelude::AtlasRegionId;

pub const SCORE_RANGED_REGULAR_HIT: u32 = 1;
pub const SCORE_RANGED_CRITICAL_HIT: u32 = 4;
pub const SCORE_MELEE_REGULAR_HIT: u32 = 3;
pub const SCORE_MELEE_CRITICAL_HIT: u32 = 10;

/// Asset path for the shared enemy attack sprite atlas.
pub const ENEMY_ATTACK_ATLAS_PATH: &str = "sprites/attacks/enemy_attacks/atlas.px_atlas.ron";

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

/// Keeps an attack entity's position locked to a resolved composed part each
/// frame until removed (e.g. when a startup hold ends and travel begins).
/// Generic — works for any composed enemy + any part.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct AttachedToComposedPart {
    /// The composed enemy entity that owns the part.
    pub source_entity: Entity,
    /// Which semantic part to track (e.g. `"head"`).
    pub part_id: String,
    /// Authored local offset within the part's sprite (e.g. mouth position).
    pub local_offset: IVec2,
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

    /// Atlas region ID for the hovering animation.
    /// Region ordering matches `atlas.px_atlas.ron`: hovering_4=0, attack_hit_1=1, hovering_2=2, attack_hit_2=3.
    #[must_use]
    pub fn hovering_region_id(&self) -> AtlasRegionId {
        match self {
            Self::BloodShot => AtlasRegionId(0),
            Self::BoulderThrow => AtlasRegionId(2),
        }
    }

    /// Atlas region ID for the hit animation.
    #[must_use]
    pub fn hit_region_id(&self) -> AtlasRegionId {
        match self {
            Self::BloodShot => AtlasRegionId(1),
            Self::BoulderThrow => AtlasRegionId(3),
        }
    }

    /// Base collision radius at the authored depth (depth 1).
    ///
    /// Projectiles travel from spawn depth toward the player at depth 0/1.
    /// Damage collision only triggers when the projectile reaches the player
    /// depth (via `LinearValueReached<TargetingValueZ>`), so the authored
    /// depth-1 radius is correct at the gameplay-critical collision point.
    /// Player-to-projectile shooting uses this same radius at all depths,
    /// making distant projectiles slightly easier to shoot than their visual
    /// size suggests — acceptable for gameplay readability.
    #[must_use]
    pub fn base_collider_radius(&self) -> f32 {
        match self {
            Self::BloodShot => 18.,
            Self::BoulderThrow => 23.,
        }
    }
}
