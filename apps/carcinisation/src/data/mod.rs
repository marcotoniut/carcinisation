//! Shared animation data helpers for stage/enemy definitions.

use crate::{pixel::PxAnimationBundle, stage::components::interactive::ColliderData};
use bevy::prelude::*;
use seldom_pixel::prelude::{
    PxAnimationDirection, PxAnimationDuration, PxAnimationFinishBehavior, PxFrameTransition,
};

/// Serialized animation metadata used by stage/enemy bundles.
pub struct AnimationData {
    pub collider_data: ColliderData,
    pub direction: PxAnimationDirection,
    pub finish_behavior: PxAnimationFinishBehavior,
    pub frame_transition: PxFrameTransition,
    pub frames: usize,
    pub speed: u64,
    pub sprite_path: String,
}

impl AnimationData {
    /// Converts this metadata into a ready-to-use animation bundle.
    pub fn make_animation_bundle(&self) -> PxAnimationBundle {
        PxAnimationBundle::from_parts(
            self.direction,
            PxAnimationDuration::millis_per_animation(self.speed),
            self.finish_behavior,
            self.frame_transition,
        )
    }
}

impl Default for AnimationData {
    fn default() -> Self {
        AnimationData {
            collider_data: ColliderData::new(),
            direction: PxAnimationDirection::Foreward,
            finish_behavior: PxAnimationFinishBehavior::Mark,
            frame_transition: PxFrameTransition::None,
            frames: 0,
            speed: 0,
            sprite_path: "".into(),
        }
    }
}
