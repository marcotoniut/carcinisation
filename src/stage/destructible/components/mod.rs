use std::collections::HashMap;

use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAnimationBundle, PxAssets},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    stage::components::interactive::{Collision, CollisionData},
    Layer,
};

use super::data::destructibles::DestructibleAnimationData;

#[derive(Component)]
pub struct Destructible;

#[derive(Component, Clone, Debug)]
pub enum DestructibleType {
    Lamp,
    Trashcan,
    Crystal,
    Mushroom,
    // Window,
    // Plant,
}

pub enum DestructibleState {
    Base,
    Broken,
}

pub fn make_animation_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
    animation_map: &HashMap<usize, DestructibleAnimationData>,
    destructible_state: &DestructibleState,
    depth: usize,
) -> Option<(PxSpriteBundle<Layer>, PxAnimationBundle, CollisionData)> {
    animation_map
        .get(&depth)
        .map(|data| data.by_state(destructible_state))
        .map(|animation_data| {
            let sprite = assets_sprite
                .load_animated(animation_data.sprite_path.as_str(), animation_data.frames);
            (
                PxSpriteBundle::<Layer> {
                    sprite,
                    layer: Layer::Middle(depth),
                    anchor: animation_data.anchor,
                    ..default()
                },
                animation_data.make_animation_bundle(),
                CollisionData {
                    collision: animation_data.collision.clone(),
                    offset: animation_data.collision_offset.clone(),
                },
            )
        })
}
