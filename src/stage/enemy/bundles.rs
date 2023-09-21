use bevy::prelude::*;
use seldom_pixel::{asset::*, prelude::*};

use crate::{
    globals::*,
    stage::components::{Collision, Health},
    Layer,
};

use super::components::*;

pub fn make_enemy_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
) -> (
    Name,
    PlaceholderEnemy,
    PxSpriteBundle<Layer>,
    PxSubPosition,
    Collision,
    Health,
) {
    let texture = assets_sprite.load("sprites/ball_red_large.png");
    (
        Name::new("Enemy"),
        PlaceholderEnemy {
            direction: Vec2::new(rand::random::<f32>(), rand::random::<f32>()).normalize(),
        },
        PxSpriteBundle::<Layer> {
            sprite: texture.clone(),
            layer: Layer::Middle(2),
            anchor: PxAnchor::Center,
            ..default()
        },
        // PxAnimationBundle {
        //     duration: PxAnimationDuration::millis_per_animation(700),
        //     on_finish: PxAnimationFinishBehavior::Despawn,
        //     ..default()
        // },
        PxSubPosition::from(Vec2::new(
            rand::random::<f32>() * SCREEN_RESOLUTION.x as f32,
            HUD_HEIGHT as f32 + rand::random::<f32>() * (SCREEN_RESOLUTION.y - HUD_HEIGHT) as f32,
        )),
        Collision::Circle(PLACEHOLDER_ENEMY_SIZE),
        Health(40),
    )
}
