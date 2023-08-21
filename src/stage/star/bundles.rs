use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{globals::SCREEN_RESOLUTION, Layer};

use super::components::*;

pub fn make_star_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
) -> (PxSpriteBundle<Layer>, PxSubPosition, Star) {
    let sprite = assets_sprite.load("sprites/star.png");
    (
        PxSpriteBundle::<Layer> {
            sprite,
            anchor: PxAnchor::Center,
            ..default()
        },
        PxSubPosition::from(Vec2::new(
            rand::random::<f32>() * SCREEN_RESOLUTION.x as f32,
            rand::random::<f32>() * SCREEN_RESOLUTION.y as f32,
        )),
        Star {},
    )
}

pub fn spawn_star_bundle(commands: &mut Commands, assets_sprite: &mut PxAssets<PxSprite>) {
    commands.spawn(make_star_bundle(assets_sprite));
}
