pub mod mosquito;

use seldom_pixel::prelude::{PxAnimationDirection, PxAnimationFinishBehavior};

pub struct AnimationData {
    pub sprite_path: String,
    pub frames: usize,
    pub speed: u64,
    pub finish_behavior: PxAnimationFinishBehavior,
    pub direction: PxAnimationDirection,
    // pub collision: Collision,
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

pub const PATH_SPRITES_ENEMIES: &str = "sprites/enemies/";
