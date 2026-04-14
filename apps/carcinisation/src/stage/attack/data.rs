pub mod blood_shot;
pub mod boulder_throw;

use crate::data::AnimationData;

pub struct HoveringAttackAnimations {
    /// Canonical animation data for the hovering phase (authored at depth 1).
    pub hovering_canonical: AnimationData,
    /// Canonical animation data for the hit phase (authored at depth 1).
    pub hit_canonical: AnimationData,
    /// Animation played when the projectile is destroyed by the player before
    /// reaching its target. `None` = despawn immediately (no visual).
    pub destroy_canonical: Option<AnimationData>,
}

impl HoveringAttackAnimations {
    /// Returns the canonical hovering animation data (depth-1 authored).
    #[must_use]
    pub fn hovering_animation_data(&self) -> &AnimationData {
        &self.hovering_canonical
    }

    /// Returns the canonical hit animation data (depth-1 authored).
    #[must_use]
    pub fn hit_animation_data(&self) -> &AnimationData {
        &self.hit_canonical
    }

    /// Returns the canonical destroy animation data, if authored.
    #[must_use]
    pub fn destroy_animation_data(&self) -> Option<&AnimationData> {
        self.destroy_canonical.as_ref()
    }
}
