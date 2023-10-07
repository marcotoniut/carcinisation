use bevy::prelude::*;

use seldom_pixel::prelude::*;

pub struct AnimationData {
    pub sprite_path: String,
    pub frames: usize,
    pub speed: u64,
    pub finish_behavior: PxAnimationFinishBehavior,
    pub direction: PxAnimationDirection,
    pub frame_transition: PxAnimationFrameTransition,
    // pub size: Vec2,
    // pub collision: Option<Collision>,
}

impl AnimationData {
    pub fn make_animation_bundle(&self) -> PxAnimationBundle {
        PxAnimationBundle {
            duration: PxAnimationDuration::millis_per_animation(self.speed),
            on_finish: self.finish_behavior,
            direction: self.direction,
            frame_transition: self.frame_transition,
            ..default()
        }
    }
}

impl Default for AnimationData {
    fn default() -> Self {
        AnimationData {
            sprite_path: String::from(""),
            frames: 0,
            speed: 0,
            finish_behavior: PxAnimationFinishBehavior::Mark,
            direction: PxAnimationDirection::Foreward,
            frame_transition: PxAnimationFrameTransition::None,
            // collision: Collision::Circle(0.),
        }
    }
}
