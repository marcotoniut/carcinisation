use bevy::prelude::*;

use super::data::AnimationData;
use seldom_pixel::{
    prelude::{PxAnchor, PxAnimationBundle, PxAnimationDuration, PxAssets},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    stage::{
        components::{Collision, Depth},
        enemy::data::blood_attack::BLOOD_ATTACK_ANIMATIONS,
    },
    Layer,
};

// Bundle
pub fn make_blood_attack_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
    depth: Depth,
) -> (PxSpriteBundle<Layer>, PxAnimationBundle, Collision) {
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
        Collision::Circle(depth.0 as f32 * 4.),
    )
}

// pub fn make_enemy_bundle(
//     assets_sprite: &mut PxAssets<PxSprite>,
// ) -> (
//     Name,
//     Enemy,
//     Hittable,
//     PlaceholderEnemy,
//     PxSpriteBundle<Layer>,
//     PxSubPosition,
//     Collision,
//     Health,
// ) {
//     let texture = assets_sprite.load("sprites/ball_red_large.png");
//     (
//         Name::new("Enemy"),
//         Enemy {},
//         Hittable {},
//         PlaceholderEnemy {
//             direction: Vec2::new(rand::random::<f32>(), rand::random::<f32>()).normalize(),
//         },
//         PxSpriteBundle::<Layer> {
//             sprite: texture.clone(),
//             layer: Layer::Middle(2),
//             anchor: PxAnchor::Center,
//             ..default()
//         },
//         PxSubPosition::from(Vec2::new(
//             rand::random::<f32>() * SCREEN_RESOLUTION.x as f32,
//             HUD_HEIGHT as f32 + rand::random::<f32>() * (SCREEN_RESOLUTION.y - HUD_HEIGHT) as f32,
//         )),
//         Collision::Circle(PLACEHOLDER_ENEMY_SIZE),
//         Health(40),
//     )
// }

pub fn make_animation_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
    animation: &AnimationData,
    depth: usize,
) -> (PxSpriteBundle<Layer>, PxAnimationBundle) {
    let texture = assets_sprite.load_animated(animation.sprite_path.as_str(), animation.frames);

    (
        PxSpriteBundle::<Layer> {
            sprite: texture,
            layer: Layer::Middle(depth),
            anchor: PxAnchor::Center,
            ..default()
        },
        animation.get_animation_bundle(),
    )
}
