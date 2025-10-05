pub mod blood_shot;
pub mod boulder_throw;

use crate::{data::AnimationData, stage::components::placement::Depth};
use bevy::utils::HashMap;

pub struct HoveringAttackAnimations {
    pub hovering: HashMap<Depth, AnimationData>,
    pub hit: HashMap<Depth, AnimationData>,
}
