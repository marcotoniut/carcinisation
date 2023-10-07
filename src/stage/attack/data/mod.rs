pub mod blood_shot;
pub mod boulder_throw;

use bevy::utils::HashMap;

use crate::data::AnimationData;

pub struct HoveringAttackAnimations {
    pub hovering: HashMap<usize, AnimationData>,
    pub hit: HashMap<usize, AnimationData>,
}
