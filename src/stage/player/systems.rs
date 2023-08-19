use bevy::{prelude::*, window::*};
use seldom_pixel::{prelude::*, sprite::*};

use crate::{globals::resolution, Layer};

use super::components::*;

pub fn spawn_player(mut commands: Commands, mut sprites: PxAssets<PxSprite>) {
    let sprite = sprites.load("sprites/ball_blue_large.png");

    // let _x = asset_server.get_handle_path(texture.to_owned()).unwrap();

    commands.spawn((
        PxSpriteBundle::<Layer> {
            sprite,
            // visibility: Visibility::Hidden,
            anchor: PxAnchor::Center,
            // position: IVec2::new((resolution.x / 2) as i32, (resolution.y / 2) as i32).into(),
            ..default()
        },
        PxSubPosition::from(Vec2::new(
            (resolution.x / 2) as f32,
            (resolution.y / 2) as f32,
        )),
        Player {},
    ));
}

pub fn despawn_player(mut commands: Commands, query: Query<Entity, With<Player>>) {
    for entity in &mut query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn confine_player_movement(mut player_query: Query<&mut PxSubPosition, With<Player>>) {
    if let Ok(mut position) = player_query.get_single_mut() {
        let half_player_size = PLAYER_SIZE / 2.0;
        let x_min = 0.0 + half_player_size;
        let x_max = resolution.x as f32 - half_player_size;
        let y_min = 0.0 + half_player_size;
        let y_max = resolution.y as f32 - half_player_size;

        let mut translation = position.0;

        if translation.x < x_min {
            translation.x = x_min;
        } else if translation.x > x_max {
            translation.x = x_max;
        }

        if translation.y < y_min {
            translation.y = y_min;
        } else if translation.y > y_max {
            translation.y = y_max;
        }

        position.0 = translation;
    }
}

pub fn player_movement(
    input: Res<Input<KeyCode>>,
    mut query: Query<(&mut PxSubPosition, &Player)>,
    time: Res<Time>,
) {
    // if let Ok((mut transform, _)) = query.get_single_mut() {
    for (mut position, _) in &mut query {
        let mut direction = Vec2::new(
            (input.pressed(KeyCode::Right) as i32 - input.pressed(KeyCode::Left) as i32) as f32,
            (input.pressed(KeyCode::Up) as i32 - input.pressed(KeyCode::Down) as i32) as f32,
        );

        if direction.length() > 0.0 {
            direction = direction.normalize();
            position.0 += direction * PLAYER_SPEED * time.delta_seconds();
        }
    }
}
