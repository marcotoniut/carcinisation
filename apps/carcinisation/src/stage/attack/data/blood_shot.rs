use crate::{data::AnimationData, stage::attack::data::HoveringAttackAnimations};
use bevy::prelude::*;
use carapace::prelude::PxAnimationFinishBehavior;
use std::time::Duration;

pub const BLOOD_SHOT_ATTACK_DEPTH_SPEED: f32 = -2.;
pub const BLOOD_SHOT_ATTACK_LINE_SPEED: f32 = 25.;
pub const BLOOD_SHOT_ATTACK_DAMAGE: u32 = 20;
pub const BLOOD_SHOT_ATTACK_RANDOMNESS: f32 = 20.;
pub const BLOOD_SHOT_ATTACK_STARTUP_HOLD: Duration = Duration::from_millis(60);

pub static BLOOD_ATTACK_ANIMATIONS: std::sync::LazyLock<HoveringAttackAnimations> =
    std::sync::LazyLock::new(|| {
        let hovering_canonical = AnimationData {
            finish_behavior: PxAnimationFinishBehavior::Loop,
            frames: 4,
            speed: 700,
            ..default()
        };

        let hit_canonical = AnimationData {
            finish_behavior: PxAnimationFinishBehavior::Mark,
            frames: 1,
            speed: 300,
            ..default()
        };

        let destroy_canonical = AnimationData {
            finish_behavior: PxAnimationFinishBehavior::Mark,
            frames: 3,
            speed: 100,
            ..default()
        };

        HoveringAttackAnimations {
            hovering_canonical,
            hit_canonical,
            destroy_canonical: Some(destroy_canonical),
        }
    });
