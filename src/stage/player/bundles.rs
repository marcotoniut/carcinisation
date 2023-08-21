use bevy::prelude::*;
use seldom_pixel::prelude::*;

use crate::{globals::SCREEN_RESOLUTION, Layer};

use super::components::*;

fn make_player_bundle(
    assets_sprite: &mut PxAssets<PxSprite>,
) -> (PxSpriteBundle<Layer>, PxSubPosition, Player) {
    let sprite = assets_sprite.load("sprites/ball_blue_large.png");
    (
        PxSpriteBundle::<Layer> {
            sprite,
            // visibility: Visibility::Hidden,
            anchor: PxAnchor::Center,
            ..default()
        },
        PxSubPosition::from(Vec2::new(
            (SCREEN_RESOLUTION.x / 2) as f32,
            (SCREEN_RESOLUTION.y / 2) as f32,
        )),
        Player {},
    )
}

pub fn spawn_player_bundle(mut commands: Commands, assets_sprite: &mut PxAssets<PxSprite>) {
    commands.spawn(make_player_bundle(assets_sprite));
}
