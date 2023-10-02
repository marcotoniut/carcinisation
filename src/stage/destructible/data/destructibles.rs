use bevy::prelude::*;
use seldom_pixel::prelude::PxAnimationFinishBehavior;
use std::collections::HashMap;

use crate::{
    globals::PATH_SPRITES_OBJECTS,
    stage::{
        components::interactive::Collision,
        destructible::components::{DestructibleState, DestructibleType},
    },
};

use super::AnimationData;

pub struct DestructibleAnimationData {
    pub base: AnimationData,
    pub broken: AnimationData,
}

impl DestructibleAnimationData {
    pub fn by_state(&self, state: &DestructibleState) -> &AnimationData {
        match state {
            DestructibleState::Base => &self.base,
            DestructibleState::Broken => &self.broken,
        }
    }
}

pub struct DestructibleAnimations {
    pub crystal: HashMap<usize, DestructibleAnimationData>,
    pub lamp: HashMap<usize, DestructibleAnimationData>,
    pub mushroom: HashMap<usize, DestructibleAnimationData>,
    pub trashcan: HashMap<usize, DestructibleAnimationData>,
}

impl DestructibleAnimations {
    pub fn new() -> Self {
        Self {
            crystal: HashMap::new(),
            lamp: HashMap::new(),
            mushroom: HashMap::new(),
            trashcan: HashMap::new(),
        }
    }

    pub fn get_animation_data(
        &self,
        destructible_type: &DestructibleType,
    ) -> &HashMap<usize, DestructibleAnimationData> {
        match destructible_type {
            DestructibleType::Crystal => &self.crystal,
            DestructibleType::Lamp => &self.lamp,
            DestructibleType::Mushroom => &self.mushroom,
            DestructibleType::Trashcan => &self.trashcan,
        }
    }
}

fn concat_strings_and_number(s1: &str, s2: &str, s3: &str, index: usize) -> String {
    format!("{}{}_{}_{}.png", s1, s2, s3, index)
}

const FRAGMENT_BASE: &str = "base";
const FRAGMENT_BROKEN: &str = "broken";

const FRAGMENT_LAMP: &str = "lamp";
const FRAGMENT_TRASHCAN: &str = "trashcan";

lazy_static! {
    pub static ref DESTRUCTIBLE_ANIMATIONS: DestructibleAnimations = {
        let mut animations = DestructibleAnimations::new();

        let lamp_base_frames = 1;
        let lamp_base_speed = 300;
        let lamp_broken_frames = 1;
        let lamp_broken_speed = 300;
        let lamp_depths = [5];

        for i in lamp_depths {
            let collision = match i {
                5 => Vec2::new(30.0, 50.0),
                _ => Vec2::new(0.0, 0.0),
            };
            let collision_offset = match i {
                5 => Vec2::new(0.0, 100.0),
                _ => Vec2::new(0.0, 0.0),
            };
            animations.lamp.insert(
                i,
                DestructibleAnimationData {
                    base: AnimationData {
                        sprite_path: concat_strings_and_number(
                            PATH_SPRITES_OBJECTS,
                            FRAGMENT_LAMP,
                            FRAGMENT_BASE,
                            i,
                        ),
                        frames: lamp_base_frames,
                        speed: lamp_base_speed,
                        finish_behavior: PxAnimationFinishBehavior::Loop,
                        collision_offset,
                        collision: Collision::Box(collision),
                        ..Default::default()
                    },
                    broken: AnimationData {
                        sprite_path: concat_strings_and_number(
                            PATH_SPRITES_OBJECTS,
                            FRAGMENT_LAMP,
                            FRAGMENT_BROKEN,
                            i,
                        ),
                        frames: lamp_broken_frames,
                        speed: lamp_broken_speed,
                        finish_behavior: PxAnimationFinishBehavior::Despawn,
                        collision_offset,
                        collision: Collision::Box(collision),
                        ..Default::default()
                    },
                },
            );
        }

        let trashcan_frames = 1;
        let trashcan_speed = 500;
        let trashcan_broken_frames = 1;
        let trashcan_broken_speed = 500;
        let trashcan_depths = [1, 4];

        for i in trashcan_depths {
            let collision = match i {
                1 => Vec2::new(14.0, 16.0),
                4 => Vec2::new(33., 38.),
                _ => Vec2::new(0.0, 0.0),
            };
            animations.trashcan.insert(
                i,
                DestructibleAnimationData {
                    base: AnimationData {
                        sprite_path: concat_strings_and_number(
                            PATH_SPRITES_OBJECTS,
                            FRAGMENT_TRASHCAN,
                            FRAGMENT_BASE,
                            i,
                        ),
                        frames: trashcan_frames,
                        speed: trashcan_speed,
                        finish_behavior: PxAnimationFinishBehavior::Loop,
                        collision: Collision::Box(collision),
                        ..Default::default()
                    },
                    broken: AnimationData {
                        sprite_path: concat_strings_and_number(
                            PATH_SPRITES_OBJECTS,
                            FRAGMENT_TRASHCAN,
                            FRAGMENT_BROKEN,
                            i,
                        ),
                        frames: trashcan_broken_frames,
                        speed: trashcan_broken_speed,
                        finish_behavior: PxAnimationFinishBehavior::Despawn,
                        collision: Collision::Box(collision),
                        ..Default::default()
                    },
                },
            );
        }

        animations
    };
}
