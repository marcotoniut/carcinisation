use crate::stage::{
    components::{
        StageEntity,
        interactive::{Flickerer, Health, Hittable},
        placement::{Depth, Speed},
    },
    enemy::{
        components::{Enemy, behavior::EnemyBehaviors, composed_state},
        composed::{ComposedAnimationState, ComposedEnemyVisual},
        entity::EnemyType,
        mosquito::entity::{ENEMY_MOSQUITO_BASE_HEALTH, EnemyMosquito, EnemyMosquitoAttacking},
    },
};
use bevy::prelude::*;
use carapace::position::WorldPos;

// Re-export generic composed enemy state components for backwards compatibility
pub use composed_state::{BrokenParts, Dying};

#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct EnemyMosquiton;

/// Maximum depth at which a mosquiton can perform ranged (blood shot) attacks.
/// Beyond this depth, mosquitons are too far to aim effectively.
pub const MOSQUITON_MAX_RANGED_DEPTH: Depth = Depth::Seven;

impl EnemyMosquiton {
    /// Whether a mosquiton at the given depth can use ranged (blood shot) attacks.
    ///
    /// Mosquitons beyond [`MOSQUITON_MAX_RANGED_DEPTH`] are restricted to
    /// non-ranged behaviour only.  This keeps deep-background mosquitons as
    /// ambient threats without projectile spam from offscreen-feeling distances.
    #[must_use]
    pub fn can_ranged_attack(depth: &Depth) -> bool {
        depth.to_i8() <= MOSQUITON_MAX_RANGED_DEPTH.to_i8()
    }

    #[must_use]
    pub fn kill_score(&self) -> u32 {
        10
    }
}

#[derive(Clone, Component, Debug, PartialEq, Eq, Reflect)]
pub enum EnemyMosquitonAnimation {
    IdleFly,
    ShootFly,
    MeleeFly,
    Falling,
}

// BrokenParts is now imported from components::composed_state

/// Marker component indicating the mosquiton's wing subsystem is fully destroyed.
///
/// When this component is present:
/// - The mosquiton will enter a falling state if airborne
/// - Flying behaviours and wing animations are disabled
/// - The entity transitions to ground-based movement
///
/// Inserted when **all** parts tagged `"wings"` are broken (joined-wing rule).
/// Individual wing parts (`wing_l`, `wing_r`) share a "wings" health pool and are
/// independently targetable, but the fall trigger requires the full subsystem
/// to be destroyed.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct WingsBroken;

/// Tracks the falling state of a mosquiton with destroyed wings.
///
/// The mosquiton will fall with gravity until it hits a floor (if one exists
/// for the current screen+depth). Upon landing, it takes fall damage based on
/// the drop height, and if it survives, continues with ground-based movement.
#[derive(Component, Clone, Debug, Reflect)]
pub struct FallingState {
    /// Y-position where the fall started (for calculating fall damage)
    pub fall_start_y: f32,
    /// Current vertical velocity (pixels per second)
    pub vertical_velocity: f32,
    /// Whether the entity has landed on the ground
    pub grounded: bool,
}

// Dying is now imported from components::composed_state

#[derive(Bundle, Debug)]
pub struct MosquitonDefaultBundle {
    pub enemy: Enemy,
    pub enemy_mosquito: EnemyMosquito,
    pub enemy_mosquito_attacking: EnemyMosquitoAttacking,
    pub enemy_mosquiton: EnemyMosquiton,
    pub flickerer: Flickerer,
    pub name: Name,
    pub health: Health,
    pub hittable: Hittable,
    pub stage_entity: StageEntity,
}

impl Default for MosquitonDefaultBundle {
    fn default() -> Self {
        Self {
            enemy: Enemy,
            enemy_mosquito: EnemyMosquito,
            enemy_mosquito_attacking: EnemyMosquitoAttacking::default(),
            enemy_mosquiton: EnemyMosquiton,
            flickerer: Flickerer,
            name: EnemyType::Mosquiton.get_name(),
            health: Health(ENEMY_MOSQUITO_BASE_HEALTH),
            hittable: Hittable,
            stage_entity: StageEntity,
        }
    }
}

#[derive(Bundle, Debug)]
pub struct MosquitonBundle {
    pub behaviors: EnemyBehaviors,
    pub composed_animation: ComposedAnimationState,
    pub composed_visual: ComposedEnemyVisual,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub depth: Depth,
    pub position: WorldPos,
    pub speed: Speed,
    pub default: MosquitonDefaultBundle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_7_allows_ranged_attack() {
        assert!(EnemyMosquiton::can_ranged_attack(&Depth::Seven));
    }

    #[test]
    fn depth_8_blocks_ranged_attack() {
        assert!(!EnemyMosquiton::can_ranged_attack(&Depth::Eight));
    }

    #[test]
    fn depth_9_blocks_ranged_attack() {
        assert!(!EnemyMosquiton::can_ranged_attack(&Depth::Nine));
    }

    #[test]
    fn all_shallow_depths_allow_ranged_attack() {
        for d in 0..=7_i8 {
            let depth = Depth::try_from(d).unwrap();
            assert!(
                EnemyMosquiton::can_ranged_attack(&depth),
                "depth {d} should allow ranged attack"
            );
        }
    }

    #[test]
    fn all_deep_depths_block_ranged_attack() {
        for d in 8..=9_i8 {
            let depth = Depth::try_from(d).unwrap();
            assert!(
                !EnemyMosquiton::can_ranged_attack(&depth),
                "depth {d} should block ranged attack"
            );
        }
    }
}
