use crate::stubs::PATH_SPRITES_ENEMIES;
use crate::{data::AnimationData, stage::components::placement::Depth};
use bevy::prelude::*;
use carapace::prelude::{CxAnimationDirection, CxAnimationFinishBehavior};
use std::collections::HashMap;

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
    format!("{}{}_{}_{}.px_sprite.png", s1, s2, s3, depth.to_i8())
}

const TARDIGRADE_DEPTHS: &[Depth] = &[Depth::Six, Depth::Seven, Depth::Eight];

pub static TARDIGRADE_ANIMATIONS: std::sync::LazyLock<TardigradeAnimations> =
    std::sync::LazyLock::new(|| {
        let idle_frames = 2;
        let idle_speed = 500;

        let sucking_frames = 4;
        let sucking_speed = 300;

        let death_frames = 5;
        let death_speed = 1000;

        let attack_frames = 5;
        let attack_speed = 330;

        let mut death = HashMap::new();
        for &i in TARDIGRADE_DEPTHS {
            death.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ENEMIES,
                        FRAGMENT_ENEMY,
                        FRAGMENT_DEATH,
                        i,
                    ),
                    direction: CxAnimationDirection::Backward,
                    finish_behavior: CxAnimationFinishBehavior::Despawn,
                    frames: death_frames,
                    speed: death_speed,
                    ..default()
                },
            );
        }

        let mut sucking = HashMap::new();
        for &i in TARDIGRADE_DEPTHS {
            sucking.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ENEMIES,
                        FRAGMENT_ENEMY,
                        FRAGMENT_SUCKING,
                        i,
                    ),
                    finish_behavior: CxAnimationFinishBehavior::Loop,
                    frames: sucking_frames,
                    speed: sucking_speed,
                    ..default()
                },
            );
        }

        let mut idle = HashMap::new();
        for &i in TARDIGRADE_DEPTHS {
            idle.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ENEMIES,
                        FRAGMENT_ENEMY,
                        FRAGMENT_IDLE,
                        i,
                    ),
                    finish_behavior: CxAnimationFinishBehavior::Loop,
                    frames: idle_frames,
                    speed: idle_speed,
                    ..default()
                },
            );
        }

        let mut attack = HashMap::new();
        for &i in TARDIGRADE_DEPTHS {
            attack.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ENEMIES,
                        FRAGMENT_ENEMY,
                        FRAGMENT_ATTACK,
                        i,
                    ),
                    finish_behavior: CxAnimationFinishBehavior::Mark,
                    frames: attack_frames,
                    speed: attack_speed,
                    ..default()
                },
            );
        }

        TardigradeAnimations {
            attack,
            death,
            idle,
            sucking,
        }
    });
