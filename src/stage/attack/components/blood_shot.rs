use bevy::prelude::*;

use seldom_pixel::{
    prelude::{PxAnchor, PxAnimationBundle, PxAnimationDuration, PxAssets},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    stage::{
        components::{
            interactive::{Collision, CollisionData},
            placement::Depth,
        },
        enemy::data::blood_attack::BLOOD_ATTACK_ANIMATIONS,
    },
    Layer,
};

// Bundle
pub fn make_blood_shot_attack_animation_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
    depth: Depth,
) -> (PxSpriteBundle<Layer>, PxAnimationBundle, CollisionData) {
    let animation_o = BLOOD_ATTACK_ANIMATIONS.hovering.get(&depth.0);

    let animation = animation_o.unwrap();
    let texture = assets_sprite.load_animated(animation.sprite_path.as_str(), animation.frames);

    (
        PxSpriteBundle::<Layer> {
            sprite: texture,
            // DEBUG
            layer: Layer::Middle(depth.0 + 2),
            anchor: PxAnchor::Center,
            ..default()
        },
        PxAnimationBundle {
            duration: PxAnimationDuration::millis_per_animation(animation.speed),
            on_finish: animation.finish_behavior,
            direction: animation.direction,
            ..default()
        },
        // TODO hardcoded
        CollisionData::new(Collision::Circle(depth.0 as f32 * 4.)),
    )
}
