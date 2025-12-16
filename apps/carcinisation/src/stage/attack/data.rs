pub mod blood_shoot_embed;
pub mod blood_shot;
pub mod boulder_throw;

use crate::{data::AnimationData, stage::components::placement::Depth};
use std::collections::HashMap;

pub struct HoveringAttackAnimations {
    pub hovering: HashMap<Depth, AnimationData>,
    pub hit: HashMap<Depth, AnimationData>,
}
