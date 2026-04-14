use crate::{data::AnimationData, stage::attack::data::HoveringAttackAnimations};
use bevy::prelude::*;
use carapace::prelude::PxAnimationFinishBehavior;

pub const BOULDER_THROW_ATTACK_DEPTH_SPEED: f32 = -1.6;
pub const BOULDER_THROW_ATTACK_LINE_Y_ACCELERATION: f32 = -55.;
pub const BOULDER_THROW_ATTACK_DAMAGE: u32 = 45;
pub const BOULDER_THROW_ATTACK_RANDOMNESS: f32 = 35.;

pub static BOULDER_ATTACK_ANIMATIONS: std::sync::LazyLock<HoveringAttackAnimations> =
    std::sync::LazyLock::new(|| {
        let hovering_canonical = AnimationData {
            finish_behavior: PxAnimationFinishBehavior::Loop,
            frames: 2,
            speed: 300,
            ..default()
        };

        let hit_canonical = AnimationData {
            finish_behavior: PxAnimationFinishBehavior::Mark,
            frames: 2,
            speed: 200,
            ..default()
        };

        let destroy_canonical = AnimationData {
            finish_behavior: PxAnimationFinishBehavior::Mark,
            frames: 2,
            speed: 100,
            ..default()
        };

        HoveringAttackAnimations {
            hovering_canonical,
            hit_canonical,
            destroy_canonical: Some(destroy_canonical),
        }
    });
