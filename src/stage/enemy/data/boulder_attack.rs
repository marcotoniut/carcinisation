use bevy::utils::HashMap;
use seldom_pixel::prelude::PxAnimationFinishBehavior;

use crate::{
    globals::PATH_SPRITES_ATTACKS,
    stage::{
        enemy::data::{AnimationData, HoveringAttackAnimations},
        player::components::PLAYER_DEPTH,
    },
};

// Animation fragments
const FRAGMENT_HOVERING: &str = "hovering";
const FRAGMENT_HIT: &str = "hit";

// Enemy
const FRAGMENT_ATTACK: &str = "boulder_attack";

fn concat_strings_and_number(s1: &str, s2: &str, s3: &str, index: usize) -> String {
    format!("{}{}_{}_{}.png", s1, s2, s3, index)
}

const MIN_DEPTH: usize = 1;
const MAX_DEPTH: usize = 8;
const HIT_DEPTH: usize = PLAYER_DEPTH as usize + 1;

lazy_static! {
    pub static ref BOULDER_ATTACK_ANIMATIONS: HoveringAttackAnimations = {
        let hovering_frames = 2;
        let hovering_speed = 700;

        let mut hovering = HashMap::new();
        for i in MIN_DEPTH..=MAX_DEPTH {
            hovering.insert(
                i,
                AnimationData {
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ATTACKS,
                        FRAGMENT_ATTACK,
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

        let hit_frames = 2;
        let hit_speed = 200;

        let mut hit = HashMap::new();

        hit.insert(
            HIT_DEPTH,
            AnimationData {
                sprite_path: concat_strings_and_number(
                    PATH_SPRITES_ATTACKS,
                    FRAGMENT_ATTACK,
                    FRAGMENT_HIT,
                    HIT_DEPTH,
                ),
                frames: hit_frames,
                speed: hit_speed,
                finish_behavior: PxAnimationFinishBehavior::Mark,
                ..Default::default()
            },
        );

        HoveringAttackAnimations { hovering, hit }
    };
}
