use crate::{
    data::AnimationData, globals::PATH_SPRITES_ENEMIES, stage::components::placement::Depth,
};
use bevy::prelude::*;
use seldom_pixel::prelude::{PxAnimationDirection, PxAnimationFinishBehavior};
use std::{collections::HashMap, ops::RangeInclusive};

pub struct TardigradeAnimations {
    pub attack: HashMap<Depth, AnimationData>,
    pub death: HashMap<Depth, AnimationData>,
    pub idle: HashMap<Depth, AnimationData>,
    pub sucking: HashMap<Depth, AnimationData>,
}

// Animation fragments
const FRAGMENT_ATTACK: &str = "attack";
const FRAGMENT_DEATH: &str = "death";
const FRAGMENT_IDLE: &str = "idle";
const FRAGMENT_SUCKING: &str = "sucking";

// Enemy
const FRAGMENT_ENEMY: &str = "tardigrade";

fn concat_strings_and_number(s1: &str, s2: &str, s3: &str, depth: Depth) -> String {
    format!("{}{}_{}_{}.png", s1, s2, s3, depth.to_i8())
}

pub const TARDIGRADE_DEPTH_RANGE: RangeInclusive<Depth> = Depth::Three..=Depth::Eight;

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
        for i in TARDIGRADE_DEPTH_RANGE {
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

        let mut sucking = HashMap::new();
        for i in TARDIGRADE_DEPTH_RANGE {
            sucking.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ENEMIES,
                        FRAGMENT_ENEMY,
                        FRAGMENT_SUCKING,
                        i,
                    ),
                    finish_behavior: PxAnimationFinishBehavior::Loop,
                    frames: sucking_frames,
                    speed: sucking_speed,
                    ..default()
                },
            );
        }

        let mut idle = HashMap::new();
        for i in TARDIGRADE_DEPTH_RANGE {
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

        let mut attack = HashMap::new();
        for i in TARDIGRADE_DEPTH_RANGE {
            attack.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ENEMIES,
                        FRAGMENT_ENEMY,
                        FRAGMENT_ATTACK,
                        i,
                    ),
                    finish_behavior: PxAnimationFinishBehavior::Mark,
                    frames: attack_frames,
                    speed: attack_speed,
                    ..default()
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
