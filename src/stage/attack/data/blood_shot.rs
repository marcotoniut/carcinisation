use crate::{
    data::AnimationData,
    globals::PATH_SPRITES_ATTACKS,
    stage::{
        attack::data::HoveringAttackAnimations,
        components::{
            interactive::{Collision, CollisionData},
            placement::Depth,
        },
    },
};
use bevy::utils::HashMap;
use seldom_pixel::prelude::PxAnimationFinishBehavior;

pub const BLOOD_SHOT_ATTACK_DEPTH_SPEED: f32 = -2.;
pub const BLOOD_SHOT_ATTACK_LINE_SPEED: f32 = 25.;
pub const BLOOD_SHOT_ATTACK_DAMAGE: u32 = 20;
pub const BLOOD_SHOT_ATTACK_RANDOMNESS: f32 = 20.;

const FRAGMENT_HOVERING: &str = "hovering";
const FRAGMENT_HIT: &str = "hit";
const FRAGMENT_ATTACK: &str = "blood_attack";

fn concat_strings_and_number(s1: &str, s2: &str, s3: &str, depth: Depth) -> String {
    format!("{}{}_{}_{}.png", s1, s2, s3, depth.to_i8())
}

const MIN_DEPTH: Depth = Depth::One;
const MAX_DEPTH: Depth = Depth::Eight;

// TODO PLAYER_DEPTH + 1;
const HIT_DEPTH: Depth = Depth::MIN;

lazy_static! {
    pub static ref BLOOD_ATTACK_ANIMATIONS: HoveringAttackAnimations = {
        let hovering_frames = 4;
        let hovering_speed = 700;

        let hit_frames = 1;
        let hit_speed = 300;

        let mut hovering = HashMap::new();

        for i in MIN_DEPTH..=MAX_DEPTH {
            hovering.insert(
                i,
                AnimationData {
                    collision: CollisionData::from_one(Collision::new_circle(match i {
                        Depth::Eight => 1.,
                        Depth::Seven => 2.,
                        Depth::Six => 3.,
                        Depth::Five => 5.,
                        Depth::Four => 7.5,
                        Depth::Three => 10.5,
                        Depth::Two => 14.,
                        Depth::One => 18.,
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

        let mut hit = HashMap::new();
        hit.insert(
            HIT_DEPTH,
            AnimationData {
                finish_behavior: PxAnimationFinishBehavior::Despawn,
                frames: hit_frames,
                sprite_path: concat_strings_and_number(
                    PATH_SPRITES_ATTACKS,
                    FRAGMENT_ATTACK,
                    FRAGMENT_HIT,
                    HIT_DEPTH,
                ),
                speed: hit_speed,
                ..Default::default()
            },
        );

        HoveringAttackAnimations { hovering, hit }
    };
}
