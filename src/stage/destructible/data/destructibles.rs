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
    pub crystal: HashMap<u8, DestructibleAnimationData>,
    pub lamp: HashMap<u8, DestructibleAnimationData>,
    pub mushroom: HashMap<u8, DestructibleAnimationData>,
    pub trashcan: HashMap<u8, DestructibleAnimationData>,
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
    ) -> &HashMap<u8, DestructibleAnimationData> {
        match destructible_type {
            DestructibleType::Crystal => &self.crystal,
            DestructibleType::Lamp => &self.lamp,
            DestructibleType::Mushroom => &self.mushroom,
            DestructibleType::Trashcan => &self.trashcan,
        }
    }
}

fn concat_strings_and_number(s1: &str, s2: &str, s3: &str, index: u8) -> String {
    format!("{}{}_{}_{}.png", s1, s2, s3, index)
}

const FRAGMENT_BASE: &str = "base";
const FRAGMENT_BROKEN: &str = "broken";

lazy_static! {
    pub static ref DESTRUCTIBLE_ANIMATIONS: DestructibleAnimations = {
        let mut animations = DestructibleAnimations::new();

        let lamp_base_frames = 1;
        let lamp_base_speed = 300;
        let lamp_broken_frames = 1;
        let lamp_broken_speed = 300;
        let lamp_depths = [5];
        let lamp_fragment = "lamp";

        // TODO review values
        for i in lamp_depths {
            let collision = match i {
                5 => Vec2::new(30.0, 50.0),
                _ => Vec2::ZERO,
            };
            let collision_offset = match i {
                5 => Vec2::new(0.0, 100.0),
                _ => Vec2::ZERO,
            };
            animations.lamp.insert(
                i,
                DestructibleAnimationData {
                    base: AnimationData {
                        sprite_path: concat_strings_and_number(
                            PATH_SPRITES_OBJECTS,
                            lamp_fragment,
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
                            lamp_fragment,
                            FRAGMENT_BROKEN,
                            i,
                        ),
                        frames: lamp_broken_frames,
                        speed: lamp_broken_speed,
                        finish_behavior: PxAnimationFinishBehavior::Mark,
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
        let trashcan_fragment = "trashcan";

        for i in trashcan_depths {
            let collision = match i {
                1 => Vec2::new(14.0, 16.0),
                4 => Vec2::new(33., 38.),
                _ => Vec2::ZERO,
            };
            animations.trashcan.insert(
                i,
                DestructibleAnimationData {
                    base: AnimationData {
                        sprite_path: concat_strings_and_number(
                            PATH_SPRITES_OBJECTS,
                            trashcan_fragment,
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
                            trashcan_fragment,
                            FRAGMENT_BROKEN,
                            i,
                        ),
                        frames: trashcan_broken_frames,
                        speed: trashcan_broken_speed,
                        finish_behavior: PxAnimationFinishBehavior::Mark,
                        collision: Collision::Box(collision),
                        ..Default::default()
                    },
                },
            );

            let mushroom_frames = 1;
            let mushroom_speed = 500;
            let mushroom_broken_frames = 1;
            let mushroom_broken_speed = 500;
            let mushroom_depths = [5];
            let mushroom_fragment = "mushroom";

            for i in mushroom_depths {
                let collision = match i {
                    5 => Vec2::new(33., 38.),
                    _ => Vec2::ZERO,
                };
                let collision_offset = match i {
                    5 => Vec2::new(-3.0, 15.0),
                    _ => Vec2::ZERO,
                };
                animations.mushroom.insert(
                    i,
                    DestructibleAnimationData {
                        base: AnimationData {
                            sprite_path: concat_strings_and_number(
                                PATH_SPRITES_OBJECTS,
                                mushroom_fragment,
                                FRAGMENT_BASE,
                                i,
                            ),
                            frames: mushroom_frames,
                            speed: mushroom_speed,
                            finish_behavior: PxAnimationFinishBehavior::Loop,
                            collision: Collision::Box(collision),
                            ..Default::default()
                        },
                        broken: AnimationData {
                            sprite_path: concat_strings_and_number(
                                PATH_SPRITES_OBJECTS,
                                mushroom_fragment,
                                FRAGMENT_BROKEN,
                                i,
                            ),
                            frames: mushroom_broken_frames,
                            speed: mushroom_broken_speed,
                            finish_behavior: PxAnimationFinishBehavior::Mark,
                            collision_offset,
                            collision: Collision::Box(collision),
                            ..Default::default()
                        },
                    },
                );
            }

            let crystal_frames = 1;
            let crystal_speed = 500;
            let crystal_broken_frames = 1;
            let crystal_broken_speed = 500;
            let crystal_depths = [4];
            let crystal_fragment = "crystal";

            for i in crystal_depths {
                let collision = match i {
                    4 => Vec2::new(33., 48.),
                    _ => Vec2::ZERO,
                };
                let collision_offset = match i {
                    4 => Vec2::new(-3.0, 15.0),
                    _ => Vec2::ZERO,
                };
                animations.crystal.insert(
                    i,
                    DestructibleAnimationData {
                        base: AnimationData {
                            sprite_path: concat_strings_and_number(
                                PATH_SPRITES_OBJECTS,
                                crystal_fragment,
                                FRAGMENT_BASE,
                                i,
                            ),
                            frames: crystal_frames,
                            speed: crystal_speed,
                            finish_behavior: PxAnimationFinishBehavior::Loop,
                            collision: Collision::Box(collision),
                            ..Default::default()
                        },
                        broken: AnimationData {
                            sprite_path: concat_strings_and_number(
                                PATH_SPRITES_OBJECTS,
                                crystal_fragment,
                                FRAGMENT_BROKEN,
                                i,
                            ),
                            frames: crystal_broken_frames,
                            speed: crystal_broken_speed,
                            finish_behavior: PxAnimationFinishBehavior::Mark,
                            collision_offset,
                            collision: Collision::Box(collision),
                            ..Default::default()
                        },
                    },
                );
            }
        }
        animations
    };
}
