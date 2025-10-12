use super::EnemyHoveringAttackType;
use crate::pixel::{PxAnimationBundle, PxAssets, PxSpriteBundle};
use crate::{
    layer::Layer,
    stage::components::{interactive::ColliderData, placement::Depth},
};
use bevy::prelude::*;
use seldom_pixel::prelude::{
    PxAnchor, PxAnimationDirection, PxAnimationDuration, PxAnimationFinishBehavior,
    PxAnimationFrameTransition, PxSprite,
};

pub fn make_hovering_attack_animation_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
    attack_type: &EnemyHoveringAttackType,
    depth: Depth,
) -> (PxSpriteBundle<Layer>, PxAnimationBundle, ColliderData) {
    let animation_o = attack_type.get_animations().hovering.get(&depth);

    let animation = animation_o.unwrap();
    let texture = assets_sprite.load_animated(animation.sprite_path.as_str(), animation.frames);
    (
        PxSpriteBundle::<Layer> {
            sprite: texture.into(),
            layer: (depth - 1).to_layer(),
            anchor: PxAnchor::Center,
            ..default()
        },
        PxAnimationBundle::from_parts(
            animation.direction,
            PxAnimationDuration::millis_per_animation(animation.speed),
            animation.finish_behavior,
            animation.frame_transition,
        ),
        animation.collider_data.clone(),
    )
}
