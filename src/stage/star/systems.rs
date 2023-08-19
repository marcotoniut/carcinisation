use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{globals::SCREEN_RESOLUTION, Layer};

use super::{components::*, resources::*};

fn make_star_bundle(
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

pub fn spawn_stars(mut commands: Commands, mut assets_sprite: PxAssets<PxSprite>) {
    for _ in 0..NUMBER_OF_STARS {
        commands.spawn(make_star_bundle(&mut assets_sprite));
    }
}

pub fn despawn_stars(mut commands: Commands, query: Query<Entity, With<Star>>) {
    for entity in &mut query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn tick_star_spawn_timer(mut star_spawn_timer: ResMut<StarSpawnTimer>, time: Res<Time>) {
    star_spawn_timer.timer.tick(time.delta());
}

pub fn spawn_stars_over_time(
    mut commands: Commands,
    star_spawn_timer: Res<StarSpawnTimer>,
    mut assets_sprite: PxAssets<PxSprite>,
) {
    if star_spawn_timer.timer.finished() {
        commands.spawn(make_star_bundle(&mut assets_sprite));
    }
}
