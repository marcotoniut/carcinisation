use bevy::{
    audio::{PlaybackMode, Volume},
    prelude::*,
};
use seldom_pixel::prelude::*;

use crate::{
    globals::SCREEN_RESOLUTION,
    stage::enemy::components::{Enemy, ENEMY_SIZE},
    Layer,
};

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

pub fn spawn_player(mut commands: Commands, mut assets_sprite: PxAssets<PxSprite>) {
    commands.spawn(make_player_bundle(&mut assets_sprite));
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
        let x_max = SCREEN_RESOLUTION.x as f32 - half_player_size;
        let y_min = 0.0 + half_player_size;
        let y_max = SCREEN_RESOLUTION.y as f32 - half_player_size;

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

pub fn enemy_hit_player(
    mut commands: Commands,
    // mut game_over_event_writer: EventWriter<GameOver>,
    mut player_query: Query<(Entity, &PxSubPosition), With<Player>>,
    enemy_query: Query<&PxSubPosition, With<Enemy>>,
    asset_server: Res<AssetServer>,
    // score: Res<Score>,
) {
    if let Ok((player_entity, player_position)) = player_query.get_single_mut() {
        for enemy_position in enemy_query.iter() {
            let distance = player_position.0.distance(enemy_position.0);

            if distance < (PLAYER_SIZE / 2.0 + ENEMY_SIZE / 2.0) {
                commands.entity(player_entity).despawn();

                let sound_effect = asset_server.load("audio/explosionCrunch_000.ogg");
                commands.spawn(AudioBundle {
                    source: sound_effect,
                    settings: PlaybackSettings {
                        mode: PlaybackMode::Despawn,
                        volume: Volume::new_relative(0.02),
                        ..default()
                    },
                    ..default()
                });

                println!("Enemy hit player! Game over!");
                // game_over_event_writer.send(GameOver { score: score.value });
            }
        }
    }
}
