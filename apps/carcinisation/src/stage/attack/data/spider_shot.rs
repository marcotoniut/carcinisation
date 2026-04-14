use crate::{data::AnimationData, stage::attack::data::HoveringAttackAnimations};
use bevy::prelude::*;
use carapace::prelude::PxAnimationFinishBehavior;

pub const SPIDER_SHOT_ATTACK_DEPTH_SPEED: f32 = -1.8;
pub const SPIDER_SHOT_ATTACK_LINE_SPEED: f32 = 20.;
pub const SPIDER_SHOT_ATTACK_DAMAGE: u32 = 15;
pub const SPIDER_SHOT_ATTACK_RANDOMNESS: f32 = 25.;

pub static SPIDER_ATTACK_ANIMATIONS: std::sync::LazyLock<HoveringAttackAnimations> =
    std::sync::LazyLock::new(|| {
        // TODO: These are fallback values. Animation params are now data-driven
        // from the atlas RON. These only apply if the atlas is not loaded yet.
        let hovering_canonical = AnimationData {
            finish_behavior: PxAnimationFinishBehavior::Loop,
            frames: 1,
            speed: 300,
            ..default()
        };

        let hit_canonical = AnimationData {
            finish_behavior: PxAnimationFinishBehavior::Mark,
            frames: 1,
            speed: 100,
            ..default()
        };

        let destroy_canonical = AnimationData {
            finish_behavior: PxAnimationFinishBehavior::Mark,
            frames: 1,
            speed: 100,
            ..default()
        };

        HoveringAttackAnimations {
            hovering_canonical,
            hit_canonical,
            destroy_canonical: Some(destroy_canonical),
        }
    });
