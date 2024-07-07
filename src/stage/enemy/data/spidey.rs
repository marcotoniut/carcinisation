use crate::{
    data::AnimationData, globals::PATH_SPRITES_ENEMIES, stage::components::placement::Depth,
};
use bevy::prelude::*;
use seldom_pixel::prelude::{PxAnimationDirection, PxAnimationFinishBehavior};
use std::{collections::HashMap, ops::RangeInclusive};

pub struct SpideyAnimations {
    pub death: HashMap<Depth, AnimationData>,
    pub idle: HashMap<Depth, AnimationData>,
}

// Animation fragments
const FRAGMENT_DEATH: &str = "death";
const FRAGMENT_IDLE: &str = "idle";

// Enemy
const FRAGMENT_ENEMY: &str = "spider";

fn concat_strings_and_number(s1: &str, s2: &str, s3: &str, depth: Depth) -> String {
    format!("{}{}_{}_{}.png", s1, s2, s3, depth.to_i8())
}

pub const SPIDEY_DEPTH_RANGE: RangeInclusive<Depth> = Depth::Two..=Depth::Seven;

lazy_static! {
    pub static ref MOSQUITO_ANIMATIONS: SpideyAnimations = {
        let idle_frames = 1;
        let idle_speed = 500;

        let death_frames = 10;
        let death_speed = 780;

        let mut death = HashMap::new();
        for i in SPIDEY_DEPTH_RANGE {
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

        let mut idle = HashMap::new();
        for i in SPIDEY_DEPTH_RANGE {
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

        SpideyAnimations { death, idle }
    };
}
