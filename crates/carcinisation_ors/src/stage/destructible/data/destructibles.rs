use super::AnimationData;
use crate::stage::{
    components::{
        interactive::{Collider, ColliderData},
        placement::Depth,
    },
    destructible::components::{DestructibleState, DestructibleType},
};
use crate::stubs::PATH_SPRITES_OBJECTS;
use bevy::prelude::*;
use carapace::prelude::CxAnimationFinishBehavior;
use derive_new::new;
use std::collections::HashMap;

pub struct DestructibleAnimationData {
    pub base: AnimationData,
    pub broken: AnimationData,
}

impl DestructibleAnimationData {
    #[must_use]
    pub const fn by_state(&self, state: &DestructibleState) -> &AnimationData {
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
    #[must_use]
    pub const fn get_animation_data(
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
    format!("{}{}_{}_{}.px_sprite.png", s1, s2, s3, depth.to_i8())
}

const FRAGMENT_BASE: &str = "base";
const FRAGMENT_BROKEN: &str = "broken";

pub static DESTRUCTIBLE_ANIMATIONS: std::sync::LazyLock<DestructibleAnimations> =
    std::sync::LazyLock::new(|| {
        let mut animations = DestructibleAnimations::new();

        let lamp_base_frames = 1;
        let lamp_base_speed = 300;
        let lamp_broken_frames = 1;
        let lamp_broken_speed = 300;
        let lamp_depths = [Depth::Three];
        let lamp_fragment = "lamp";

        for depth in lamp_depths {
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
                        finish_behavior: CxAnimationFinishBehavior::Loop,
                        collider_data: match depth {
                            Depth::Three => ColliderData::from_one(
                                Collider::new_box(Vec2::new(17.0, 19.0))
                                    .with_offset(Vec2::new(-1.0, 122.0)),
                            ),
                            _ => unreachable!(),
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
                        finish_behavior: CxAnimationFinishBehavior::Mark,
                        ..default()
                    },
                },
            );
        }

        let trashcan_frames = 1;
        let trashcan_speed = 500;
        let trashcan_broken_frames = 1;
        let trashcan_broken_speed = 500;
        let trashcan_depths = [Depth::Six, Depth::Four];
        let trashcan_fragment = "trashcan";

        for depth in trashcan_depths {
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
                        finish_behavior: CxAnimationFinishBehavior::Loop,
                        collider_data: match depth {
                            Depth::Six => ColliderData::from_one(
                                Collider::new_box(Vec2::new(8.0, 11.0))
                                    .with_offset(Vec2::new(-1.0, 6.0)),
                            ),
                            Depth::Four => ColliderData::from_one(
                                Collider::new_box(Vec2::new(18., 24.))
                                    .with_offset(Vec2::new(-2.0, 16.0)),
                            ),
                            _ => unreachable!(),
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
                        finish_behavior: CxAnimationFinishBehavior::Mark,
                        ..default()
                    },
                },
            );
        }

        let mushroom_frames = 1;
        let mushroom_speed = 500;
        let mushroom_broken_frames = 1;
        let mushroom_broken_speed = 500;
        let mushroom_depths = [Depth::Four];
        let mushroom_fragment = "mushroom";

        for depth in mushroom_depths {
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
                        finish_behavior: CxAnimationFinishBehavior::Loop,
                        collider_data: match depth {
                            Depth::Four => ColliderData::from_many(vec![
                                Collider::new_box(Vec2::new(15., 70.))
                                    .with_offset(Vec2::new(1., 49.)),
                                Collider::new_circle(24.).with_offset(Vec2::new(-1.0, 57.0)),
                            ]),
                            _ => unreachable!(),
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
                        finish_behavior: CxAnimationFinishBehavior::Mark,
                        ..default()
                    },
                },
            );
        }

        let crystal_frames = 1;
        let crystal_speed = 500;
        let crystal_broken_frames = 1;
        let crystal_broken_speed = 500;
        let crystal_depths = [Depth::Five];
        let crystal_fragment = "crystal";

        for depth in crystal_depths {
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
                        finish_behavior: CxAnimationFinishBehavior::Loop,
                        collider_data: match depth {
                            Depth::Five => ColliderData::from_one(
                                Collider::new_box(Vec2::new(40., 60.))
                                    .with_offset(Vec2::new(-4., 40.)),
                            ),
                            _ => unreachable!(),
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
                        finish_behavior: CxAnimationFinishBehavior::Mark,
                        ..default()
                    },
                },
            );
        }
        animations
    });
