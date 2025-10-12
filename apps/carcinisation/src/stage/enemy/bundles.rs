use crate::{
    data::AnimationData,
    layer::Layer,
    pixel::{PxAnimationBundle, PxAssets, PxSpriteBundle},
    stage::components::placement::Depth,
};
use bevy::prelude::*;
use seldom_pixel::prelude::{PxAnchor, PxSprite};

/**
 * TODO
 * - depth will have an impact on the sprite_path
 * - anchor data should be included in the AnimationData
 * - this function could be agnostic
 */
pub fn make_enemy_animation_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
    data: &AnimationData,
    depth: &Depth,
) -> (PxSpriteBundle<Layer>, PxAnimationBundle) {
    let texture = assets_sprite.load_animated(data.sprite_path.clone(), data.frames);

    (
        PxSpriteBundle::<Layer> {
            sprite: texture.into(),
            layer: depth.to_layer(),
            anchor: PxAnchor::Center,
            ..default()
        },
        data.make_animation_bundle(),
    )
}
