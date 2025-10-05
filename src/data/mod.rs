//! Shared animation data helpers for stage/enemy definitions.

use crate::stage::components::interactive::ColliderData;
use bevy::prelude::*;
use seldom_pixel::prelude::*;

/// Serialized animation metadata used by stage/enemy bundles.
pub struct AnimationData {
    pub collider_data: ColliderData,
    pub direction: PxAnimationDirection,
    pub finish_behavior: PxAnimationFinishBehavior,
    pub frame_transition: PxAnimationFrameTransition,
    pub frames: usize,
    pub speed: u64,
    pub sprite_path: String,
}

impl AnimationData {
    /// Converts this metadata into a ready-to-use animation bundle.
    pub fn make_animation_bundle(&self) -> PxAnimationBundle {
        PxAnimationBundle {
            direction: self.direction,
            duration: PxAnimationDuration::millis_per_animation(self.speed),
            frame_transition: self.frame_transition,
            on_finish: self.finish_behavior,
            ..default()
        }
    }
}

impl Default for AnimationData {
    fn default() -> Self {
        AnimationData {
            collider_data: ColliderData::new(),
            direction: PxAnimationDirection::Foreward,
            finish_behavior: PxAnimationFinishBehavior::Mark,
            frame_transition: PxAnimationFrameTransition::None,
            frames: 0,
            speed: 0,
            sprite_path: "".into(),
        }
    }
}
