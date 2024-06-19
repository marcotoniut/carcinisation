use super::EnemyHoveringAttackType;
use crate::{
    layer::Layer,
    stage::components::{interactive::ColliderData, placement::Depth},
};
use seldom_pixel::{
    prelude::{PxAnchor, PxAnimationBundle, PxAnimationDuration, PxAssets},
    sprite::{PxSprite, PxSpriteBundle},
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
            sprite: texture,
            layer: (depth - 1).to_layer(),
            anchor: PxAnchor::Center,
            ..Default::default()
        },
        PxAnimationBundle {
            duration: PxAnimationDuration::millis_per_animation(animation.speed),
            on_finish: animation.finish_behavior,
            direction: animation.direction,
            ..Default::default()
        },
        animation.collider_data.clone(),
    )
}
