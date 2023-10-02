pub mod blood_attack;
pub mod mosquito;
pub mod tardigrade;
use bevy::prelude::*;

use seldom_pixel::prelude::{
    PxAnimationBundle, PxAnimationDirection, PxAnimationDuration, PxAnimationFinishBehavior,
};

pub struct AnimationData {
    pub sprite_path: String,
    pub frames: usize,
    pub speed: u64,
    pub finish_behavior: PxAnimationFinishBehavior,
    pub direction: PxAnimationDirection,
    // pub collision: Collision,
}

impl AnimationData {
    pub fn make_animation_bundle(&self) -> PxAnimationBundle {
        PxAnimationBundle {
            duration: PxAnimationDuration::millis_per_animation(self.speed),
            on_finish: self.finish_behavior,
            direction: self.direction,
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
            // collision: Collision::Circle(0.),
        }
    }
}
