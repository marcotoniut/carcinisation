use super::data::destructibles::DestructibleAnimationData;
use crate::{
    layer::Layer,
    stage::components::{interactive::ColliderData, placement::Depth},
};
use bevy::prelude::*;
use seldom_pixel::prelude::{PxAnimation, PxSprite};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Component)]
pub struct Destructible;

#[derive(Clone, Component, Debug, Deserialize, Reflect, Serialize)]
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
    asset_server: &Res<AssetServer>,
    animation_map: &HashMap<Depth, DestructibleAnimationData>,
    destructible_state: &DestructibleState,
    depth: &Depth,
) -> Option<(PxSprite, Layer, PxAnchor, PxAnimation, ColliderData)> {
    animation_map
        .get(depth)
        .map(|data| data.by_state(destructible_state))
        .map(|animation_data| {
            let sprite = PxSprite(asset_server.load(animation_data.sprite_path.clone()));
            // TODO animate animation_data.frames
            (
                sprite,
                depth.to_layer(),
                animation_data.anchor,
                animation_data.make_animation_bundle(),
                animation_data.collider_data.clone(),
            )
        })
}
