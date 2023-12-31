use super::data::destructibles::DestructibleAnimationData;
use crate::{
    stage::components::{interactive::ColliderData, placement::Depth},
    Layer,
};
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
    animation_map: &HashMap<Depth, DestructibleAnimationData>,
    destructible_state: &DestructibleState,
    depth: &Depth,
) -> Option<(PxSpriteBundle<Layer>, PxAnimationBundle, ColliderData)> {
    animation_map
        .get(depth)
        .map(|data| data.by_state(destructible_state))
        .map(|animation_data| {
            let sprite = assets_sprite
                .load_animated(animation_data.sprite_path.clone(), animation_data.frames);
            (
                PxSpriteBundle::<Layer> {
                    sprite,
                    layer: depth.to_layer(),
                    anchor: animation_data.anchor,
                    ..Default::default()
                },
                animation_data.make_animation_bundle(),
                animation_data.collider_data.clone(),
            )
        })
}
