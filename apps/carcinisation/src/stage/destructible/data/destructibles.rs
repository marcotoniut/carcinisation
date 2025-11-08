use super::AnimationData;
use crate::{
    globals::PATH_SPRITES_OBJECTS,
    stage::{
        components::{
            interactive::{Collider, ColliderData},
            placement::Depth,
        },
        destructible::{
            components::{DestructibleState, DestructibleType},
            data::{CrystalDepth, LampDepth, MushroomDepth, TrashcanDepth},
        },
    },
};
use bevy::prelude::*;
use derive_new::new;
use seldom_pixel::prelude::PxAnimationFinishBehavior;
use std::collections::HashMap;

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

#[derive(new)]
pub struct DestructibleAnimations {
    #[new(default)]
    pub crystal: HashMap<Depth, DestructibleAnimationData>,
    #[new(default)]
    pub lamp: HashMap<Depth, DestructibleAnimationData>,
    #[new(default)]
    pub mushroom: HashMap<Depth, DestructibleAnimationData>,
    #[new(default)]
    pub trashcan: HashMap<Depth, DestructibleAnimationData>,
}

impl DestructibleAnimations {
    pub fn get_animation_data(
        &self,
        destructible_type: &DestructibleType,
    ) -> &HashMap<Depth, DestructibleAnimationData> {
        match destructible_type {
            DestructibleType::Crystal => &self.crystal,
            DestructibleType::Lamp => &self.lamp,
            DestructibleType::Mushroom => &self.mushroom,
            DestructibleType::Trashcan => &self.trashcan,
        }
    }
}

// TODO turn this into a general access macro
fn concat_strings_and_number(s1: &str, s2: &str, s3: &str, depth: Depth) -> String {
    format!("{}{}_{}_{}.png", s1, s2, s3, depth.to_i8())
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
        let lamp_depths = [LampDepth::Three];
        let lamp_fragment = "lamp";

        for i in lamp_depths {
            let depth = i.to_depth();
            animations.lamp.insert(
                depth,
                DestructibleAnimationData {
                    base: AnimationData {
                        sprite_path: concat_strings_and_number(
                            PATH_SPRITES_OBJECTS,
                            lamp_fragment,
                            FRAGMENT_BASE,
                            depth,
                        ),
                        frames: lamp_base_frames,
                        speed: lamp_base_speed,
                        finish_behavior: PxAnimationFinishBehavior::Loop,
                        collider_data: match i {
                            LampDepth::Three => ColliderData::from_one(
                                Collider::new_box(Vec2::new(17.0, 19.0))
                                    .with_offset(Vec2::new(-1.0, 122.0)),
                            ),
                        },
                        ..default()
                    },
                    broken: AnimationData {
                        sprite_path: concat_strings_and_number(
                            PATH_SPRITES_OBJECTS,
                            lamp_fragment,
                            FRAGMENT_BROKEN,
                            depth,
                        ),
                        frames: lamp_broken_frames,
                        speed: lamp_broken_speed,
                        finish_behavior: PxAnimationFinishBehavior::Mark,
                        ..default()
                    },
                },
            );
        }

        let trashcan_frames = 1;
        let trashcan_speed = 500;
        let trashcan_broken_frames = 1;
        let trashcan_broken_speed = 500;
        let trashcan_depths = [TrashcanDepth::Six, TrashcanDepth::Four];
        let trashcan_fragment = "trashcan";

        for i in trashcan_depths {
            let depth = i.to_depth();
            animations.trashcan.insert(
                depth,
                DestructibleAnimationData {
                    base: AnimationData {
                        sprite_path: concat_strings_and_number(
                            PATH_SPRITES_OBJECTS,
                            trashcan_fragment,
                            FRAGMENT_BASE,
                            depth,
                        ),
                        frames: trashcan_frames,
                        speed: trashcan_speed,
                        finish_behavior: PxAnimationFinishBehavior::Loop,
                        collider_data: match i {
                            TrashcanDepth::Six => ColliderData::from_one(
                                Collider::new_box(Vec2::new(8.0, 11.0))
                                    .with_offset(Vec2::new(-1.0, 6.0)),
                            ),
                            TrashcanDepth::Four => ColliderData::from_one(
                                Collider::new_box(Vec2::new(18., 24.))
                                    .with_offset(Vec2::new(-2.0, 16.0)),
                            ),
                        },
                        ..default()
                    },
                    broken: AnimationData {
                        sprite_path: concat_strings_and_number(
                            PATH_SPRITES_OBJECTS,
                            trashcan_fragment,
                            FRAGMENT_BROKEN,
                            depth,
                        ),
                        frames: trashcan_broken_frames,
                        speed: trashcan_broken_speed,
                        finish_behavior: PxAnimationFinishBehavior::Mark,
                        ..default()
                    },
                },
            );

            let mushroom_frames = 1;
            let mushroom_speed = 500;
            let mushroom_broken_frames = 1;
            let mushroom_broken_speed = 500;
            let mushroom_depths = [MushroomDepth::Four];
            let mushroom_fragment = "mushroom";

            for i in mushroom_depths {
                let depth = i.to_depth();
                animations.mushroom.insert(
                    depth,
                    DestructibleAnimationData {
                        base: AnimationData {
                            sprite_path: concat_strings_and_number(
                                PATH_SPRITES_OBJECTS,
                                mushroom_fragment,
                                FRAGMENT_BASE,
                                depth,
                            ),
                            frames: mushroom_frames,
                            speed: mushroom_speed,
                            finish_behavior: PxAnimationFinishBehavior::Loop,
                            collider_data: match i {
                                MushroomDepth::Four => ColliderData::from_many(vec![
                                    Collider::new_box(Vec2::new(15., 70.))
                                        .with_offset(Vec2::new(1., 49.)),
                                    Collider::new_circle(24.).with_offset(Vec2::new(-1.0, 57.0)),
                                ]),
                            },
                            ..default()
                        },
                        broken: AnimationData {
                            sprite_path: concat_strings_and_number(
                                PATH_SPRITES_OBJECTS,
                                mushroom_fragment,
                                FRAGMENT_BROKEN,
                                depth,
                            ),
                            frames: mushroom_broken_frames,
                            speed: mushroom_broken_speed,
                            finish_behavior: PxAnimationFinishBehavior::Mark,
                            ..default()
                        },
                    },
                );
            }

            let crystal_frames = 1;
            let crystal_speed = 500;
            let crystal_broken_frames = 1;
            let crystal_broken_speed = 500;
            let crystal_depths = [CrystalDepth::Five];
            let crystal_fragment = "crystal";

            for i in crystal_depths {
                let depth = i.to_depth();
                animations.crystal.insert(
                    depth,
                    DestructibleAnimationData {
                        base: AnimationData {
                            sprite_path: concat_strings_and_number(
                                PATH_SPRITES_OBJECTS,
                                crystal_fragment,
                                FRAGMENT_BASE,
                                depth,
                            ),
                            frames: crystal_frames,
                            speed: crystal_speed,
                            finish_behavior: PxAnimationFinishBehavior::Loop,
                            collider_data: match i {
                                CrystalDepth::Five => ColliderData::from_one(
                                    Collider::new_box(Vec2::new(40., 60.))
                                        .with_offset(Vec2::new(-4., 40.)),
                                ),
                            },
                            ..default()
                        },
                        broken: AnimationData {
                            sprite_path: concat_strings_and_number(
                                PATH_SPRITES_OBJECTS,
                                crystal_fragment,
                                FRAGMENT_BROKEN,
                                depth,
                            ),
                            frames: crystal_broken_frames,
                            speed: crystal_broken_speed,
                            finish_behavior: PxAnimationFinishBehavior::Mark,
                            ..default()
                        },
                    },
                );
            }
        }
        animations
    };
}
