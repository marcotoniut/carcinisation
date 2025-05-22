use crate::{data::AnimationData, layer::Layer, stage::components::placement::Depth};
use bevy::prelude::*;
use seldom_pixel::prelude::{PxAnchor, PxAnimation, PxSprite};

#[derive(Bundle)]
pub struct EnemyAnimationBundle {
    pub anchor: PxAnchor,
    pub animation: PxAnimation,
    pub depth: Depth,
    pub sprite: PxSprite,
}

/**
 * TODO
 * - depth will have an impact on the sprite_path
 * - anchor data should be included in the AnimationData
 * - this function could be agnostic
 */
impl EnemyAnimationBundle {
    pub fn new(asset_server: &Res<AssetServer>, data: &AnimationData, depth: &Depth) -> Self {
        let sprite = PxSprite(asset_server.load(data.sprite_path.clone()));
        // TODO animate data.frames

        Self {
            anchor: PxAnchor::Center,
            animation: data.make_animation_bundle(),
            depth: depth.clone(),
            sprite,
        }
    }
}
