use bevy::prelude::*;
use seldom_pixel::{asset::*, prelude::*};

use crate::{globals::*, stage::components::Collision, Layer};

use super::components::*;

pub fn make_enemy_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
) -> (PxSpriteBundle<Layer>, PxSubPosition, Enemy, Collision, Name) {
    let texture = assets_sprite.load("sprites/ball_red_large.png");
    (
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
        Enemy {
            direction: Vec2::new(rand::random::<f32>(), rand::random::<f32>()).normalize(),
        },
        Collision::Circle(ENEMY_SIZE),
        Name::new("Enemy"),
    )
}
