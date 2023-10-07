use bevy::utils::HashMap;
use seldom_pixel::prelude::PxAnimationFinishBehavior;

use crate::{
    data::AnimationData,
    globals::PATH_SPRITES_ATTACKS,
    stage::{attack::data::HoveringAttackAnimations, player::components::PLAYER_DEPTH},
};

pub const BLOOD_SHOT_ATTACK_DEPTH_SPEED: f32 = 4.;
pub const BLOOD_SHOT_ATTACK_LINE_SPEED: f32 = 25.;
pub const BLOOD_SHOT_ATTACK_DAMAGE: u32 = 20;

const FRAGMENT_HOVERING: &str = "hovering";
const FRAGMENT_HIT: &str = "hit";
const FRAGMENT_ATTACK: &str = "blood_attack";

fn concat_strings_and_number(s1: &str, s2: &str, s3: &str, index: usize) -> String {
    format!("{}{}_{}_{}.png", s1, s2, s3, index)
}

const MIN_DEPTH: usize = 1;
const MAX_DEPTH: usize = 8;

const HIT_DEPTH: usize = PLAYER_DEPTH as usize + 1;

lazy_static! {
    pub static ref BLOOD_ATTACK_ANIMATIONS: HoveringAttackAnimations = {
        let hovering_frames = 4;
        let hovering_speed = 700;

        let hit_frames = 1;
        let hit_speed = 600;

        let mut hovering = HashMap::new();

        // fn get_size(i: usize) {
        //     match i {
        //         1 => 1,
        //         2 => 2,
        //         3 => 3,
        //     }
        // }

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
                    // size: get_size(i),
                    ..Default::default()
                },
            );
        }

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
                finish_behavior: PxAnimationFinishBehavior::Despawn,
                ..Default::default()
            },
        );

        HoveringAttackAnimations { hovering, hit }
    };
}
