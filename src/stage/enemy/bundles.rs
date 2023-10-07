use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAnimationBundle, PxAssets},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{data::AnimationData, Layer};

/**
 * TODO
 * - depth will have an impact on the sprite_path
 * - anchor data should be included in the AnimationData
 * - this function could be agnostic
 */
pub fn make_enemy_animation_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
    data: &AnimationData,
    depth: usize,
) -> (PxSpriteBundle<Layer>, PxAnimationBundle) {
    let texture = assets_sprite.load_animated(data.sprite_path.as_str(), data.frames);

    (
        PxSpriteBundle::<Layer> {
            sprite: texture,
            layer: Layer::Middle(depth),
            anchor: PxAnchor::Center,
            ..default()
        },
        data.make_animation_bundle(),
    )
}
