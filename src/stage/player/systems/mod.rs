pub mod camera;
pub mod events;

use super::components::*;
use super::resources::AttackTimer;
use crate::core::time::DeltaTime;
use crate::{
    components::DespawnMark,
    globals::{mark_for_despawn_by_component_query, HUD_HEIGHT, SCREEN_RESOLUTION},
    systems::audio::VolumeSettings,
    GBInput,
};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use seldom_pixel::prelude::*;
use std::time::Duration;

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

pub fn player_movement<T: DeltaTime + Resource>(
    gb_input: Res<ActionState<GBInput>>,
    // TODO should this system refer to a Cursor component instead?
    mut query: Query<&mut PxSubPosition, With<Player>>,
    time: Res<T>,
) {
    for mut position in &mut query {
        // TODO review what's more expensive, querying or input subroutine, although, most of the times
        // a player will exist, so it's probably more that the former is redundant
        let mut direction = Vec2::new(
            (gb_input.pressed(&GBInput::Right) as i32 - gb_input.pressed(&GBInput::Left) as i32)
                as f32,
            (gb_input.pressed(&GBInput::Up) as i32 - gb_input.pressed(&GBInput::Down) as i32)
                as f32,
        );

        if direction.length() > 0.0 {
            direction = direction.normalize_or_zero();
            position.0 += direction * PLAYER_SPEED * time.delta_seconds();
        }
    }
}

pub fn detect_player_attack(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut timer: ResMut<AttackTimer>,
    asset_server: Res<AssetServer>,
    gb_input: Res<ActionState<GBInput>>,
    player_attack_query: Query<&PlayerAttack>,
    player_query: Query<&PxSubPosition, With<Player>>,
    volume_settings: Res<VolumeSettings>,
) {
    if player_attack_query.iter().next().is_none() {
        if let Ok(position) = player_query.get_single() {
            let attack = if gb_input.just_pressed(&GBInput::A) {
                Some((Weapon::Pincer, 0.6))
            } else if gb_input.just_pressed(&GBInput::B) {
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
}

pub fn tick_attack_timer<T: DeltaTime + Resource>(mut timer: ResMut<AttackTimer>, time: Res<T>) {
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
