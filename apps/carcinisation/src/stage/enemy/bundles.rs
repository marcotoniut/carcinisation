use crate::{
    data::AnimationData,
    layer::Layer,
    pixel::{CxAnimationBundle, CxAssets, CxSpriteBundle},
    stage::components::placement::Depth,
};
use bevy::prelude::*;
use carapace::prelude::{CxAnchor, CxSprite};

/**
 * TODO
 * - depth will have an impact on the `sprite_path`
 * - anchor data should be included in the `AnimationData`
 * - this function could be agnostic
 */
pub fn make_enemy_animation_bundle(
    assets_sprite: &mut CxAssets<CxSprite>,
    data: &AnimationData,
    depth: &Depth,
) -> (CxSpriteBundle<Layer>, CxAnimationBundle) {
    let texture = assets_sprite.load_animated(data.sprite_path.clone(), data.frames);

    (
        CxSpriteBundle::<Layer> {
            sprite: texture.into(),
            layer: depth.to_layer(),
            anchor: CxAnchor::Center,
            ..default()
        },
        data.make_animation_bundle(),
    )
}
