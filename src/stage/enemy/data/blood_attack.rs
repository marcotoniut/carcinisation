use seldom_pixel::prelude::PxAnimationFinishBehavior;
use std::collections::HashMap;

use crate::stage::enemy::data::{AnimationData, PATH_SPRITES_ATTACKS};

pub struct BloodAttackAnimations {
    pub hovering: HashMap<usize, AnimationData>,
    pub splat: HashMap<usize, AnimationData>,
}

// Animation fragments
const FRAGMENT_HOVERING: &str = "hovering";
const FRAGMENT_SPLAT: &str = "splat";

// Enemy
const FRAGMENT_BLOOD_ATTACK: &str = "blood_attack";

fn concat_strings_and_number(s1: &str, s2: &str, s3: &str, index: usize) -> String {
    format!("{}{}_{}_{}.png", s1, s2, s3, index)
}

lazy_static! {
    pub static ref BLOOD_ATTACK_ANIMATIONS: BloodAttackAnimations = {
        let hovering_frames = 4;
        let hovering_speed = 700;

        let splat_frames = 1;
        let splat_speed = 600;

        let mut hovering = HashMap::new();
        for i in 1..=6 {
            hovering.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ATTACKS,
                        FRAGMENT_BLOOD_ATTACK,
                        FRAGMENT_HOVERING,
                        i,
                    ),
                    frames: hovering_frames,
                    speed: hovering_speed,
                    finish_behavior: PxAnimationFinishBehavior::Loop,
                    ..Default::default()
                },
            );
        }

        let mut splat = HashMap::new();
        splat.insert(
            7,
            AnimationData {
                sprite_path: concat_strings_and_number(
                    PATH_SPRITES_ATTACKS,
                    FRAGMENT_BLOOD_ATTACK,
                    FRAGMENT_SPLAT,
                    7,
                ),
                frames: splat_frames,
                speed: splat_speed,
                finish_behavior: PxAnimationFinishBehavior::Despawn,
                ..Default::default()
            },
        );

        BloodAttackAnimations { hovering, splat }
    };
}
