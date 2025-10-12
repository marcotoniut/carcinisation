use crate::{
    data::AnimationData, globals::PATH_SPRITES_ENEMIES, stage::components::placement::Depth,
};
use bevy::prelude::*;
use bevy::utils::HashMap;
use seldom_pixel::prelude::{PxAnimationDirection, PxAnimationFinishBehavior};
use std::ops::RangeInclusive;

pub struct MosquitoAnimations {
    pub death: HashMap<Depth, AnimationData>,
    pub fly: HashMap<Depth, AnimationData>,
    pub idle: HashMap<Depth, AnimationData>,
    pub melee_attack: HashMap<Depth, AnimationData>,
}

// Animation fragments
const FRAGMENT_IDLE: &str = "idle";
const FRAGMENT_FLY: &str = "fly";
const FRAGMENT_DEATH: &str = "death";
const FRAGMENT_MELEE_ATTACK: &str = "melee_attack";

// Enemy
const FRAGMENT_ENEMY: &str = "mosquito";

fn concat_strings_and_number(s1: &str, s2: &str, s3: &str, depth: Depth) -> String {
    format!("{}{}_{}_{}.png", s1, s2, s3, depth.to_i8())
}

const MOSQUITO_DEPTHS: &[Depth] = &[
    Depth::Three,
    Depth::Four,
    Depth::Five,
    Depth::Six,
    Depth::Seven,
    Depth::Eight,
];

pub const MOSQUITO_DEPTH_RANGE: RangeInclusive<Depth> = Depth::Three..=Depth::Eight;

lazy_static! {
    pub static ref MOSQUITO_ANIMATIONS: MosquitoAnimations = {
        let idle_frames = 3;
        let idle_speed = 500;

        let fly_frames = 3;
        let fly_speed = 90;

        let death_frames = 20;
        let death_speed = 780;

        let melee_attack_frames = 8;
        let melee_attack_speed = 130;

        let mut death = HashMap::new();
        for &i in MOSQUITO_DEPTHS {
            death.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ENEMIES,
                        FRAGMENT_ENEMY,
                        FRAGMENT_DEATH,
                        i,
                    ),
                    direction: PxAnimationDirection::Backward,
                    finish_behavior: PxAnimationFinishBehavior::Despawn,
                    frames: death_frames,
                    speed: death_speed,
                    ..default()
                },
            );
        }

        let mut fly = HashMap::new();
        for &i in MOSQUITO_DEPTHS {
            fly.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ENEMIES,
                        FRAGMENT_ENEMY,
                        FRAGMENT_FLY,
                        i,
                    ),
                    finish_behavior: PxAnimationFinishBehavior::Loop,
                    frames: fly_frames,
                    speed: fly_speed,
                    ..default()
                },
            );
        }

        let mut idle = HashMap::new();
        for &i in MOSQUITO_DEPTHS {
            idle.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ENEMIES,
                        FRAGMENT_ENEMY,
                        FRAGMENT_IDLE,
                        i,
                    ),
                    finish_behavior: PxAnimationFinishBehavior::Loop,
                    frames: idle_frames,
                    speed: idle_speed,
                    ..default()
                },
            );
        }

        let mut melee_attack = HashMap::new();
        for &i in MOSQUITO_DEPTHS {
            melee_attack.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ENEMIES,
                        FRAGMENT_ENEMY,
                        FRAGMENT_MELEE_ATTACK,
                        i,
                    ),
                    finish_behavior: PxAnimationFinishBehavior::Mark,
                    frames: melee_attack_frames,
                    speed: melee_attack_speed,
                    ..default()
                },
            );
        }

        MosquitoAnimations {
            death,
            fly,
            idle,
            melee_attack,
        }
    };
}
