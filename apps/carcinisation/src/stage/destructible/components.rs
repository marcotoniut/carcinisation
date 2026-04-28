use super::data::destructibles::DestructibleAnimationData;
use crate::{
    assets::CxAssets,
    layer::Layer,
    stage::components::{interactive::ColliderData, placement::Depth},
};
use bevy::prelude::*;
use carapace::prelude::{CxAnimationBundle, CxSprite, CxSpriteBundle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Component)]
pub struct Destructible;

#[derive(Clone, Component, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Reflect, Serialize)]
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

pub fn make_animation_bundle<S: std::hash::BuildHasher>(
    assets_sprite: &mut CxAssets<CxSprite>,
    animation_map: &HashMap<Depth, DestructibleAnimationData, S>,
    destructible_state: &DestructibleState,
    depth: &Depth,
) -> Option<(CxSpriteBundle<Layer>, CxAnimationBundle, ColliderData)> {
    animation_map
        .get(depth)
        .map(|data| data.by_state(destructible_state))
        .map(|animation_data| {
            let sprite = assets_sprite
                .load_animated(animation_data.sprite_path.clone(), animation_data.frames);
            (
                CxSpriteBundle::<Layer> {
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
