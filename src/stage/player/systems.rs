use std::time::Duration;

use bevy::{
    audio::{PlaybackMode, Volume},
    prelude::*,
};
use leafwing_input_manager::prelude::ActionState;
use seldom_pixel::prelude::*;

use crate::{
    globals::{HUD_HEIGHT, SCREEN_RESOLUTION},
    stage::score::components::Score,
    systems::audio::{AudioSystemBundle, AudioSystemType, VolumeSettings},
    GBInput,
};

use super::{bundles::*, components::*, resources::*};
use super::{crosshair::CrosshairSettings, resources::AttackTimer};

pub fn spawn_player(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    crosshair_settings: Res<CrosshairSettings>,
) {
    // if let Ok((entity, _)) = stage_query.get_single() {
    //     commands.entity(entity).despawn_recursive();
    // }
    commands.spawn(make_player_bundle(&mut assets_sprite, crosshair_settings));
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
        let x_max = (SCREEN_RESOLUTION.x - 1) as f32 - half_player_size;
        let y_min = HUD_HEIGHT as f32 + half_player_size;
        let y_max = (SCREEN_RESOLUTION.y - 1) as f32 - half_player_size;

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
    gb_input_query: Query<&ActionState<GBInput>>,
    mut query: Query<(&mut PxSubPosition, &Player)>,
    time: Res<Time>,
) {
    let gb_input = gb_input_query.single();
    for (mut position, _) in &mut query {
        let mut direction = Vec2::new(
            (gb_input.pressed(GBInput::Right) as i32 - gb_input.pressed(GBInput::Left) as i32)
                as f32,
            (gb_input.pressed(GBInput::Up) as i32 - gb_input.pressed(GBInput::Down) as i32) as f32,
        );

        if direction.length() > 0.0 {
            direction = direction.normalize();
            position.0 += direction * PLAYER_SPEED * time.delta_seconds();
        }
    }
}
pub fn detect_player_attack(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut timer: ResMut<AttackTimer>,
    asset_server: Res<AssetServer>,
    gb_input_query: Query<&ActionState<GBInput>>,
    player_attack_query: Query<&PlayerAttack>,
    player_query: Query<&PxSubPosition, With<Player>>,
) {
    if player_attack_query.iter().next().is_none() {
        let position = player_query.get_single().unwrap();
        let gb_input = gb_input_query.get_single().unwrap();

        let attack = if gb_input.just_pressed(GBInput::A) {
            Some((Weapon::Pincer, 0.6))
        } else if gb_input.just_pressed(GBInput::B) {
            Some((Weapon::Gun, 0.08))
        } else {
            None
        };

        if let Some((weapon, duration)) = attack {
            timer.timer.set_duration(Duration::from_secs_f64(duration));
            let player_attack = PlayerAttack {
                position: position.0.clone(),
                weapon,
            };

            let (player_attack_bundle, audio_bundle) =
                player_attack.make_bundles(&mut assets_sprite, asset_server);

            commands.spawn(player_attack_bundle);
            commands.spawn((
                audio_bundle,
                AudioSystemBundle {
                    system_type: AudioSystemType::SFX,
                },
            ));

            timer.timer.reset();
            timer.timer.unpause();
        }
    }
}

pub fn tick_attack_timer(mut timer: ResMut<AttackTimer>, time: Res<Time>) {
    timer.timer.tick(time.delta());
}

pub fn check_attack_timer(
    mut commands: Commands,
    timer: ResMut<AttackTimer>,
    player_attack_query: Query<(Entity, &PlayerAttack)>, // event to attack?
                                                         // mut event_writer: EventWriter<StageActionTrigger>,
) {
    if timer.timer.finished() {
        for (entity, _) in &mut player_attack_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}
