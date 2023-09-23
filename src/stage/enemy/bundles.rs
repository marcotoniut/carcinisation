use bevy::prelude::*;
use seldom_pixel::{asset::*, prelude::*};

use crate::{
    globals::*,
    stage::components::{Collision, Health, Hittable},
    Layer,
};

use super::{components::*, data::AnimationData};

pub fn make_enemy_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
) -> (
    Name,
    Enemy,
    Hittable,
    PlaceholderEnemy,
    PxSpriteBundle<Layer>,
    PxSubPosition,
    Collision,
    Health,
) {
    let texture = assets_sprite.load("sprites/ball_red_large.png");
    (
        Name::new("Enemy"),
        Enemy {},
        Hittable {},
        PlaceholderEnemy {
            direction: Vec2::new(rand::random::<f32>(), rand::random::<f32>()).normalize(),
        },
        PxSpriteBundle::<Layer> {
            sprite: texture.clone(),
            layer: Layer::Middle(2),
            anchor: PxAnchor::Center,
            ..default()
        },
        PxSubPosition::from(Vec2::new(
            rand::random::<f32>() * SCREEN_RESOLUTION.x as f32,
            HUD_HEIGHT as f32 + rand::random::<f32>() * (SCREEN_RESOLUTION.y - HUD_HEIGHT) as f32,
        )),
        Collision::Circle(PLACEHOLDER_ENEMY_SIZE),
        Health(40),
    )
}

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
        PxAnimationBundle {
            duration: PxAnimationDuration::millis_per_animation(animation.speed),
            // on_finish: animation.finish_behavior,
            on_finish: PxAnimationFinishBehavior::Loop,
            ..default()
        },
    )
}
