pub mod camera;
pub mod messages;

use super::attacks::{
    AttackButton, AttackDefinitions, AttackInputPolicy, AttackInputState, AttackLifetime,
    AttackLoadout,
};
use super::components::*;
use crate::input::GBInput;
use crate::pixel::PxAssets;
use crate::stage::resources::StageTimeDomain;
use crate::{
    components::{DespawnMark, VolumeSettings},
    globals::{HUD_HEIGHT, SCREEN_RESOLUTION},
};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use seldom_pixel::prelude::{PxSprite, PxSubPosition};

pub fn confine_player_movement(mut player_query: Query<&mut PxSubPosition, With<Player>>) {
    if let Ok(mut position) = player_query.single_mut() {
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
    gb_input: Res<ActionState<GBInput>>,
    // TODO should this system refer to a Cursor component instead?
    mut query: Query<&mut PxSubPosition, With<Player>>,
    time: Res<Time<StageTimeDomain>>,
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
            position.0 += direction * PLAYER_SPEED * time.delta().as_secs_f32();
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn detect_player_attack(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    asset_server: Res<AssetServer>,
    gb_input: Res<ActionState<GBInput>>,
    player_attack_query: Query<Entity, With<PlayerAttack>>,
    player_query: Query<&PxSubPosition, With<Player>>,
    volume_settings: Res<VolumeSettings>,
    time: Res<Time<StageTimeDomain>>,
    attack_definitions: Res<AttackDefinitions>,
    mut loadout: ResMut<AttackLoadout>,
    mut input_state: ResMut<AttackInputState>,
) {
    let now = time.elapsed_secs();
    let attack_active = player_attack_query.iter().next().is_some();

    if input_state.active_button.is_none() {
        if gb_input.just_pressed(&GBInput::A) {
            input_state.arm(AttackButton::A, now);
        } else if gb_input.just_pressed(&GBInput::B) {
            input_state.arm(AttackButton::B, now);
        }
    }

    if gb_input.just_pressed(&GBInput::Select) {
        if let Some(button) = input_state.active_button {
            if gb_input.pressed(&button.gb_input()) {
                loadout.cycle(button);
            }
        }
    }

    let Some(button) = input_state.active_button else {
        return;
    };

    let button_input = button.gb_input();
    let still_pressed = gb_input.pressed(&button_input);
    let attack_id = loadout.current(button);
    let definition = attack_definitions.get(attack_id);

    let mut spawn_attack = |position: Vec2| {
        let player_attack = PlayerAttack {
            position,
            attack_id,
        };
        let (attack_bundle, sound_bundle) = player_attack.make_bundles(
            definition,
            &mut assets_sprite,
            asset_server.as_ref(),
            volume_settings.as_ref(),
        );
        commands.spawn(attack_bundle);
        commands.spawn(sound_bundle);
    };

    if still_pressed && !attack_active {
        if let AttackInputPolicy::Hold {
            warmup_secs,
            interval_secs,
        } = definition.input_policy
        {
            let held_for = now - input_state.pressed_at;
            let can_fire = input_state
                .last_hold_fire_at
                .is_none_or(|last| now - last >= interval_secs);
            if held_for >= warmup_secs && can_fire {
                if let Ok(position) = player_query.single() {
                    spawn_attack(position.0);
                    input_state.mark_hold_fired(now);
                }
            }
        }
    }

    if gb_input.just_released(&button_input) {
        let should_fire = match definition.input_policy {
            AttackInputPolicy::Release => true,
            AttackInputPolicy::Hold { .. } => !input_state.hold_fired,
        };

        if should_fire && !attack_active {
            if let Ok(position) = player_query.single() {
                spawn_attack(position.0);
            }
        }

        input_state.clear();
    } else if !still_pressed {
        input_state.clear();
    }
}

pub fn tick_attack_lifetimes(
    mut query: Query<&mut AttackLifetime, With<PlayerAttack>>,
    time: Res<Time<StageTimeDomain>>,
) {
    for mut lifetime in &mut query {
        lifetime.timer.tick(time.delta());
    }
}

pub fn despawn_expired_attacks(
    mut commands: Commands,
    query: Query<(Entity, &AttackLifetime), With<PlayerAttack>>,
) {
    for (entity, lifetime) in &query {
        if lifetime.timer.is_finished() {
            commands.entity(entity).insert(DespawnMark);
        }
    }
}
