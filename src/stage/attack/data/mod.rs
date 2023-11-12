pub mod blood_shot;
pub mod boulder_throw;

use bevy::utils::HashMap;
use crate::{data::AnimationData, stage::components::placement::Depth};

pub struct HoveringAttackAnimations {
    pub hovering: HashMap<Depth, AnimationData>,
    pub hit: HashMap<Depth, AnimationData>,
}
