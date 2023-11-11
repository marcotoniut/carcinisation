use super::data::destructibles::DestructibleAnimationData;
use crate::{stage::components::interactive::CollisionData, Layer};
use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnimationBundle, PxAssets},
    sprite::{PxSprite, PxSpriteBundle},
};
use std::collections::HashMap;

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
    animation_map: &HashMap<u8, DestructibleAnimationData>,
    destructible_state: &DestructibleState,
    depth: u8,
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
                    ..Default::default()
                },
                animation_data.make_animation_bundle(),
                animation_data.collision_data.clone(),
            )
        })
}
