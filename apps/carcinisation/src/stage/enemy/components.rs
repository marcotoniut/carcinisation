pub mod behavior;
pub mod composed_state;

use crate::stage::components::placement::Depth;
use bevy::prelude::*;
use cween::structs::TweenDirection;

#[derive(Component, Debug, Default)]
pub struct Enemy;

/// Canonical continuous depth value for enemy entities.
///
/// This is the semantic source of enemy depth:
/// - movement writes this continuously
/// - gameplay bucket [`Depth`] is derived from it
/// - it remains meaningful even when no depth tween is active
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct EnemyContinuousDepth(pub f32);

impl Default for EnemyContinuousDepth {
    fn default() -> Self {
        Self(Depth::default().to_f32())
    }
}

impl EnemyContinuousDepth {
    #[must_use]
    pub fn from_depth(depth: Depth) -> Self {
        Self(depth.to_f32())
    }

    #[must_use]
    pub fn clamped_value(self) -> f32 {
        Depth::clamp_continuous(self.0)
    }

    #[must_use]
    pub fn snapped_depth(self) -> Depth {
        Depth::from_continuous(self.0)
    }
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct CircleAround {
    pub radius: f32,
    pub center: Vec2,
    pub time_offset: f32,
    pub direction: TweenDirection,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct LinearTween {
    pub direction: Vec2,
    pub trayectory: f32,
    // TODO replace with LinearTween2DReached
    pub reached_x: bool,
    pub reached_y: bool,
}

// Bosses

#[derive(Component)]
pub struct EnemyMarauder;

#[derive(Component)]
pub struct EnemySpidomonsta {}

#[derive(Component)]
pub struct EnemyKyle {}
