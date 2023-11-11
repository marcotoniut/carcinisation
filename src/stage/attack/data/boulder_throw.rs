use crate::{
    data::AnimationData,
    globals::PATH_SPRITES_ATTACKS,
    stage::{
        attack::data::HoveringAttackAnimations,
        components::interactive::{Collision, CollisionData},
        player::components::PLAYER_DEPTH,
    },
};
use bevy::utils::HashMap;
use seldom_pixel::prelude::PxAnimationFinishBehavior;

pub const BOULDER_THROW_ATTACK_DEPTH_SPEED: f32 = 3.;
pub const BOULDER_THROW_ATTACK_LINE_Y_ACCELERATION: f32 = -55.;
pub const BOULDER_THROW_ATTACK_DAMAGE: u32 = 45;
pub const BOULDER_THROW_ATTACK_RANDOMNESS: f32 = 35.;

const FRAGMENT_HOVERING: &str = "hovering";
const FRAGMENT_HIT: &str = "hit";
const FRAGMENT_ATTACK: &str = "boulder_attack";

fn concat_strings_and_number(s1: &str, s2: &str, s3: &str, index: u8) -> String {
    format!("{}{}_{}_{}.png", s1, s2, s3, index)
}

const MIN_DEPTH: u8 = 1;
const MAX_DEPTH: u8 = 8;
const HIT_DEPTH: u8 = PLAYER_DEPTH as u8 + 1;

lazy_static! {
    pub static ref BOULDER_ATTACK_ANIMATIONS: HoveringAttackAnimations = {
        let hovering_frames = 2;
        let hovering_speed = 300;

        let mut hovering = HashMap::new();
        for i in MIN_DEPTH..=MAX_DEPTH {
            hovering.insert(
                i,
                AnimationData {
                    collision: CollisionData::from_one(Collision::new_circle(match i {
                        1 => 1.,
                        2 => 2.5,
                        3 => 4.5,
                        4 => 7.,
                        5 => 10.,
                        6 => 14.,
                        7 => 18.,
                        8 => 23.,
                        _ => 0.,
                    })),
                    finish_behavior: PxAnimationFinishBehavior::Loop,
                    frames: hovering_frames,
                    speed: hovering_speed,
                    sprite_path: concat_strings_and_number(
                        PATH_SPRITES_ATTACKS,
                        FRAGMENT_ATTACK,
                        FRAGMENT_HOVERING,
                        i,
                    ),
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
                finish_behavior: PxAnimationFinishBehavior::Mark,
                frames: hit_frames,
                speed: hit_speed,
                sprite_path: concat_strings_and_number(
                    PATH_SPRITES_ATTACKS,
                    FRAGMENT_ATTACK,
                    FRAGMENT_HIT,
                    HIT_DEPTH,
                ),
                ..Default::default()
            },
        );

        HoveringAttackAnimations { hovering, hit }
    };
}
