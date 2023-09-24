use seldom_pixel::prelude::{PxAnimationDirection, PxAnimationFinishBehavior};
use std::collections::HashMap;

use crate::stage::enemy::data::{AnimationData, PATH_SPRITES_ENEMIES};

pub struct MosquitoAnimations {
    pub death: HashMap<usize, AnimationData>,
    pub fly: HashMap<usize, AnimationData>,
    pub idle: HashMap<usize, AnimationData>,
    pub melee_attack: HashMap<usize, AnimationData>,
}

// Animation fragments
const FRAGMENT_IDLE: &str = "idle";
const FRAGMENT_FLY: &str = "fly";
const FRAGMENT_DEATH: &str = "death";
const FRAGMENT_MELEE_ATTACK: &str = "melee_attack";

// Enemy
const FRAGMENT_MOSQUITO: &str = "mosquito";

fn concat_strings_and_number(s1: &str, s2: &str, s3: &str, index: usize) -> String {
    format!("{}{}_{}_{}.png", s1, s2, s3, index)
}

lazy_static! {
    pub static ref MOSQUITO_ANIMATIONS: MosquitoAnimations = {
        let idle_frames = 3;
        let idle_speed = 500;

        // TODO
        let fly_frames = 3;
        let fly_speed = 90;

        let death_frames = 20;
        let death_speed = 780;

        let attack_frames = 8;
        let melee_attack_speed = 130;

        let mut death = HashMap::new();
        for i in 0..=3 {
            death.insert(i, AnimationData {
                sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_MOSQUITO, FRAGMENT_DEATH, i),
                frames: death_frames,
                speed: death_speed,
                finish_behavior: PxAnimationFinishBehavior::Despawn,
                direction: PxAnimationDirection::Backward,
                ..Default::default()
            });
        }

        let mut fly = HashMap::new();
        for i in 0..=2 {
            fly.insert(i, AnimationData {
                sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_MOSQUITO, FRAGMENT_FLY, i),
                frames: fly_frames,
                speed: fly_speed,
                finish_behavior: PxAnimationFinishBehavior::Loop,
                ..Default::default()
            });
        }

        let mut idle = HashMap::new();
        for i in 1..=5 {
            idle.insert(i, AnimationData {
                sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_MOSQUITO, FRAGMENT_IDLE, i),
                frames: idle_frames,
                speed: idle_speed,
                finish_behavior: PxAnimationFinishBehavior::Loop,
                ..Default::default()
            });
        }

        let mut melee_attack = HashMap::new();
        melee_attack.insert(0, AnimationData {
            sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_MOSQUITO, FRAGMENT_MELEE_ATTACK, 0),
            frames: attack_frames,
            speed: melee_attack_speed,
            finish_behavior: PxAnimationFinishBehavior::Mark,
            ..Default::default()
        });

        MosquitoAnimations { death, fly, idle, melee_attack }
    };
}
