pub mod mosquito;

use seldom_pixel::prelude::{PxAnimationDirection, PxAnimationFinishBehavior};

#[derive(Default)]
pub struct AnimationData {
    pub sprite_path: String,
    pub frames: u32,
    pub speed: u32,
    pub finish_behavior: PxAnimationFinishBehavior,
    pub direction: PxAnimationDirection,
    // pub collision: Collision,
}
