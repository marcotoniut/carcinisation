pub mod blood_shot;
pub mod boulder_throw;

use bevy::utils::HashMap;

use crate::data::AnimationData;

pub struct HoveringAttackAnimations {
    pub hovering: HashMap<u8, AnimationData>,
    pub hit: HashMap<u8, AnimationData>,
}
