use crate::stage::{
    components::{
        StageEntity,
        interactive::{ColliderData, Flickerer, Health, Hittable},
        placement::{Depth, Speed},
    },
    enemy::{
        components::{Enemy, behavior::EnemyBehaviors},
        composed::{ComposedAnimationState, ComposedEnemyVisual},
        entity::EnemyType,
        mosquito::entity::{ENEMY_MOSQUITO_BASE_HEALTH, EnemyMosquito, EnemyMosquitoAttacking},
    },
};
use bevy::prelude::*;
use seldom_pixel::position::PxSubPosition;

#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct EnemyMosquiton;

impl EnemyMosquiton {
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

/// Tracks which parts of this composed enemy have been broken.
///
/// This is a generic component that tracks all broken parts. Specific behavioral
/// markers like `WingsBroken` are added based on which parts break.
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct BrokenParts {
    /// Set of part IDs that have been broken
    pub parts: std::collections::HashSet<String>,
}

impl BrokenParts {
    /// Check if a specific part is broken
    pub fn is_broken(&self, part_id: &str) -> bool {
        self.parts.contains(part_id)
    }

    /// Mark a part as broken
    pub fn mark_broken(&mut self, part_id: String) {
        self.parts.insert(part_id);
    }

    /// Get all broken parts
    pub fn broken_parts(&self) -> &std::collections::HashSet<String> {
        &self.parts
    }
}

/// Marker component indicating the mosquiton's wings have been destroyed.
///
/// When this component is present:
/// - The mosquiton will enter a falling state if airborne
/// - Flying behaviors and wing animations are disabled
/// - The entity transitions to ground-based movement
///
/// This is automatically added when the "wings_visual" part breaks.
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
    pub collider_data: ColliderData,
    pub composed_animation: ComposedAnimationState,
    pub composed_visual: ComposedEnemyVisual,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub depth: Depth,
    pub position: PxSubPosition,
    pub speed: Speed,
    pub default: MosquitonDefaultBundle,
}
