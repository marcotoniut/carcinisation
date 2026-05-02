//! Shared animation data helpers for stage/enemy definitions.

use crate::stage::components::interactive::ColliderData;
use bevy::prelude::*;
use carapace::prelude::{
    CxAnimationBundle, CxAnimationDirection, CxAnimationDuration, CxAnimationFinishBehavior,
    CxFrameTransition,
};

/// Serialized animation metadata used by stage/enemy bundles.
pub struct AnimationData {
    pub collider_data: ColliderData,
    pub direction: CxAnimationDirection,
    pub finish_behavior: CxAnimationFinishBehavior,
    pub frame_transition: CxFrameTransition,
    pub frames: usize,
    pub speed: u64,
    pub sprite_path: String,
}

impl AnimationData {
    /// Converts this metadata into a ready-to-use animation bundle.
    #[must_use]
    pub fn make_animation_bundle(&self) -> CxAnimationBundle {
        CxAnimationBundle::from_parts(
            self.direction,
            CxAnimationDuration::millis_per_animation(self.speed),
            self.finish_behavior,
            self.frame_transition,
        )
    }
}

impl Default for AnimationData {
    fn default() -> Self {
        AnimationData {
            collider_data: ColliderData::new(),
            direction: CxAnimationDirection::Forward,
            finish_behavior: CxAnimationFinishBehavior::Mark,
            frame_transition: CxFrameTransition::None,
            frames: 0,
            speed: 0,
            sprite_path: String::new(),
        }
    }
}
