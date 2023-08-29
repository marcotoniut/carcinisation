use bevy::prelude::*;
use seldom_pixel::{asset::*, prelude::*};

use crate::{globals::*, Layer};

use super::components::*;

fn make_enemy_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
) -> (PxSpriteBundle<Layer>, PxSubPosition, Enemy, Name) {
    let texture = assets_sprite.load("sprites/ball_red_large.png");
    (
        PxSpriteBundle::<Layer> {
            sprite: texture.clone(),
            anchor: PxAnchor::Center,
            ..default()
        },
        PxSubPosition::from(Vec2::new(
            rand::random::<f32>() * SCREEN_RESOLUTION.x as f32,
            HUD_HEIGHT as f32 + rand::random::<f32>() * (SCREEN_RESOLUTION.y - HUD_HEIGHT) as f32,
        )),
        Enemy {
            direction: Vec2::new(rand::random::<f32>(), rand::random::<f32>()).normalize(),
        },
        Name::new("Enemy"),
    )
}

pub fn spawn_enemy_bundle(commands: &mut Commands, assets_sprite: &mut PxAssets<PxSprite>) {
    commands.spawn(make_enemy_bundle(assets_sprite));
}
