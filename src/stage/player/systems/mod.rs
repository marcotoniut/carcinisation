pub mod camera;

use std::time::Duration;

use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use seldom_pixel::prelude::*;

use crate::{
    components::DespawnMark,
    game::events::GameOver,
    globals::{mark_for_despawn_by_component_query, HUD_HEIGHT, SCREEN_RESOLUTION},
    stage::{components::Dead, score::components::Score},
    systems::audio::{AudioSystemBundle, AudioSystemType, VolumeSettings},
    GBInput,
};

use super::{bundles::*, components::*};
use super::{crosshair::CrosshairSettings, resources::AttackTimer};

pub fn spawn_player(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    crosshair_settings: Res<CrosshairSettings>,
) {
    commands.spawn(make_player_bundle(&mut assets_sprite, crosshair_settings));
}

/**
 * deprecate
 */
pub fn despawn_player(mut commands: Commands, query: Query<Entity, With<Player>>) {
    mark_for_despawn_by_component_query(&mut commands, &query)
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
    volume_settings: Res<VolumeSettings>,
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

            let (player_attack_bundle, player_attack_sound_bundle) =
                player_attack.make_bundles(&mut assets_sprite, asset_server, volume_settings);

            commands.spawn(player_attack_bundle);
            commands.spawn(player_attack_sound_bundle);

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
            commands.entity(entity).insert(DespawnMark);
        }
    }
}

pub const DEATH_SCORE_PENALTY: i32 = 150;

pub fn check_player_died(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut query: Query<(Entity, &mut Player), Added<Dead>>,
    mut event_writer: EventWriter<GameOver>,
) {
    if let Ok((entity, mut player)) = query.get_single_mut() {
        score.add(-DEATH_SCORE_PENALTY);
        player.lives = player.lives.saturating_sub(1);
        if player.lives == 0 {
            // event_writer.send(StageGameOverTrigger {});
            event_writer.send(GameOver { score: score.value });
        }
    }
}
