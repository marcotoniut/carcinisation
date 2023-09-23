use seldom_pixel::prelude::PxAnimationFinishBehavior;

use crate::stage::enemy::data::{AnimationData, PATH_SPRITES_ATTACKS};

pub struct BloodAttackAnimations {
    pub hovering: Vec<AnimationData>,
}

// Animation fragments
const FRAGMENT_HOVERING: &str = "hovering";

// Enemy
const FRAGMENT_BLOOD_ATTACK: &str = "blood_attack";

fn concat_strings_and_number(s1: &str, s2: &str, s3: &str, index: u32) -> String {
    format!("{}{}_{}_{}.png", s1, s2, s3, index)
}

lazy_static! {
    pub static ref BLOOD_ATTACK_ANIMATIONS: BloodAttackAnimations = {
        let hovering_frames = 1;
        let hovering_speed = 400;

        BloodAttackAnimations {
            hovering: vec![
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ATTACKS, FRAGMENT_BLOOD_ATTACK, FRAGMENT_HOVERING, 1),
                    frames: hovering_frames,
                    speed: hovering_speed,
                    finish_behavior: PxAnimationFinishBehavior::Loop,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ATTACKS, FRAGMENT_BLOOD_ATTACK, FRAGMENT_HOVERING, 2),
                    frames: hovering_frames,
                    speed: hovering_speed,
                    finish_behavior: PxAnimationFinishBehavior::Loop,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ATTACKS, FRAGMENT_BLOOD_ATTACK, FRAGMENT_HOVERING, 3),
                    frames: hovering_frames,
                    speed: hovering_speed,
                    finish_behavior: PxAnimationFinishBehavior::Loop,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ATTACKS, FRAGMENT_BLOOD_ATTACK, FRAGMENT_HOVERING, 4),
                    frames: hovering_frames,
                    speed: hovering_speed,
                    finish_behavior: PxAnimationFinishBehavior::Loop,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ATTACKS, FRAGMENT_BLOOD_ATTACK, FRAGMENT_HOVERING, 5),
                    frames: hovering_frames,
                    speed: hovering_speed,
                    finish_behavior: PxAnimationFinishBehavior::Loop,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ATTACKS, FRAGMENT_BLOOD_ATTACK, FRAGMENT_HOVERING, 6),
                    frames: hovering_frames,
                    speed: hovering_speed,
                    finish_behavior: PxAnimationFinishBehavior::Loop,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
            ],
        }
    };
}
