use super::data::destructibles::DestructibleAnimationData;
use crate::pixel::{PxAnimationBundle, PxAssets, PxSpriteBundle};
use crate::{
    layer::Layer,
    stage::components::{interactive::ColliderData, placement::Depth},
};
use bevy::prelude::*;
use seldom_pixel::prelude::PxSprite;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "derive-ts")]
use ts_rs::TS;

#[derive(Component)]
pub struct Destructible;

#[cfg_attr(feature = "derive-ts", derive(TS))]
#[cfg_attr(feature = "derive-ts", ts(export))]
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
                    sprite: sprite.into(),
                    layer: depth.to_layer(),
                    anchor: animation_data.anchor,
                    ..default()
                },
                animation_data.make_animation_bundle(),
                animation_data.collider_data.clone(),
            )
        })
}
