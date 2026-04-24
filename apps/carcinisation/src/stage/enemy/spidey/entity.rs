use crate::stage::{
    components::{
        StageEntity,
        interactive::{Flickerer, Health, Hittable},
        placement::{Depth, Speed},
    },
    enemy::{
        components::{Enemy, behavior::EnemyBehaviors},
        composed::{ComposedAnimationState, ComposedEnemyVisual},
        data::steps::EnemyStep,
        entity::EnemyType,
    },
};
use bevy::prelude::*;
use carapace::position::WorldPos;
use std::collections::VecDeque;

pub const ENEMY_SPIDEY_RADIUS: f32 = 8.0;
pub const ENEMY_SPIDEY_BASE_HEALTH: u32 = 20;

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct EnemySpidey;

impl EnemySpidey {
    #[must_use]
    pub fn kill_score(&self) -> u32 {
        8
    }
}

#[derive(Clone, Component, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub enum EnemySpideyAnimation {
    Idle,
    Lounge,
    Shoot,
    Jump,
    Landing,
}

/// Marker component indicating the spidey is in an attacking state.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct EnemySpideyAttacking;

/// Authored step cycle to requeue once a Spidey exhausts its current behavior list.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct EnemySpideyBehaviorLoop(pub VecDeque<EnemyStep>);

#[derive(Bundle, Debug)]
pub struct SpideyDefaultBundle {
    pub enemy: Enemy,
    pub enemy_spidey: EnemySpidey,
    pub flickerer: Flickerer,
    pub name: Name,
    pub health: Health,
    pub hittable: Hittable,
    pub stage_entity: StageEntity,
}

impl Default for SpideyDefaultBundle {
    fn default() -> Self {
        Self {
            enemy: Enemy,
            enemy_spidey: EnemySpidey,
            flickerer: Flickerer,
            name: EnemyType::Spidey.get_name(),
            health: Health(ENEMY_SPIDEY_BASE_HEALTH),
            hittable: Hittable,
            stage_entity: StageEntity,
        }
    }
}

#[derive(Bundle, Debug)]
pub struct SpideyBundle {
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
    pub default: SpideyDefaultBundle,
}
