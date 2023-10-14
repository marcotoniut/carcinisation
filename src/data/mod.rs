use crate::stage::components::interactive::CollisionData;
use bevy::prelude::*;
use seldom_pixel::prelude::*;

pub struct AnimationData {
    pub collision: Option<CollisionData>,
    pub direction: PxAnimationDirection,
    pub finish_behavior: PxAnimationFinishBehavior,
    pub frame_transition: PxAnimationFrameTransition,
    pub frames: usize,
    pub speed: u64,
    pub sprite_path: String,
}

impl AnimationData {
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
            collision: None,
            direction: PxAnimationDirection::Foreward,
            finish_behavior: PxAnimationFinishBehavior::Mark,
            frame_transition: PxAnimationFrameTransition::None,
            frames: 0,
            speed: 0,
            sprite_path: String::from(""),
        }
    }
}
