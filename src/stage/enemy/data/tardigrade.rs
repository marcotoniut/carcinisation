use seldom_pixel::prelude::{PxAnimationDirection, PxAnimationFinishBehavior};
use std::collections::HashMap;

use crate::{data::AnimationData, globals::PATH_SPRITES_ENEMIES};

pub struct TardigradeAnimations {
    pub attack: HashMap<usize, AnimationData>,
    pub death: HashMap<usize, AnimationData>,
    pub idle: HashMap<usize, AnimationData>,
    pub sucking: HashMap<usize, AnimationData>,
}

// Animation fragments
const FRAGMENT_ATTACK: &str = "attack";
const FRAGMENT_DEATH: &str = "death";
const FRAGMENT_IDLE: &str = "idle";
const FRAGMENT_SUCKING: &str = "sucking";

// Enemy
const FRAGMENT_ENEMY: &str = "tardigrade";

fn concat_strings_and_number(s1: &str, s2: &str, s3: &str, index: usize) -> String {
    format!("{}{}_{}_{}.png", s1, s2, s3, index)
}

lazy_static! {
    pub static ref TARDIGRADE_ANIMATIONS: TardigradeAnimations = {
        let idle_frames = 2;
        let idle_speed = 500;

        let sucking_frames = 4;
        let sucking_speed = 300;

        let death_frames = 5;
        let death_speed = 1000;

        let attack_frames = 5;
        let attack_speed = 330;

        let mut death = HashMap::new();
        for i in 1..=3 {
            death.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ENEMIES,
                        FRAGMENT_ENEMY,
                        FRAGMENT_DEATH,
                        i,
                    ),
                    frames: death_frames,
                    speed: death_speed,
                    finish_behavior: PxAnimationFinishBehavior::Despawn,
                    direction: PxAnimationDirection::Backward,
                    ..Default::default()
                },
            );
        }

        let mut sucking = HashMap::new();
        for i in 1..=3 {
            sucking.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ENEMIES,
                        FRAGMENT_ENEMY,
                        FRAGMENT_SUCKING,
                        i,
                    ),
                    frames: sucking_frames,
                    speed: sucking_speed,
                    finish_behavior: PxAnimationFinishBehavior::Loop,
                    ..Default::default()
                },
            );
        }

        let mut idle = HashMap::new();
        for i in 1..=3 {
            idle.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ENEMIES,
                        FRAGMENT_ENEMY,
                        FRAGMENT_IDLE,
                        i,
                    ),
                    frames: idle_frames,
                    speed: idle_speed,
                    finish_behavior: PxAnimationFinishBehavior::Loop,
                    ..Default::default()
                },
            );
        }

        let mut attack = HashMap::new();
        for i in 1..=3 {
            attack.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ENEMIES,
                        FRAGMENT_ENEMY,
                        FRAGMENT_ATTACK,
                        i,
                    ),
                    frames: attack_frames,
                    speed: attack_speed,
                    finish_behavior: PxAnimationFinishBehavior::Mark,
                    ..Default::default()
                },
            );
        }

        TardigradeAnimations {
            death,
            sucking,
            idle,
            attack,
        }
    };
}
