use bevy::utils::HashMap;

use crate::data::AnimationData;

pub mod blood_shot;
pub mod boulder_throw;

pub const BLOOD_ATTACK_DEPTH_SPEED: f32 = 4.;
pub const BLOOD_ATTACK_LINE_SPEED: f32 = 25.;
pub const BLOOD_ATTACK_DAMAGE: u32 = 20;

pub struct HoveringAttackAnimations {
    pub hovering: HashMap<usize, AnimationData>,
    pub hit: HashMap<usize, AnimationData>,
}
