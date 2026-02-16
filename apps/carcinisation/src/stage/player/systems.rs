pub mod camera;
pub mod messages;

use super::attacks::{
    AttackButton, AttackDefinitions, AttackId, AttackInputPolicy, AttackInputState, AttackLifetime,
    AttackLoadout,
};
use super::components::*;
use crate::input::GBInput;
use crate::pixel::PxAssets;
use crate::stage::player::attacks::AttackEffectState;
use crate::stage::player::messages::CameraShakeEvent;
use crate::stage::resources::StageTimeDomain;
use crate::{
    components::{DespawnMark, VolumeSettings},
    globals::{HUD_HEIGHT, SCREEN_RESOLUTION},
    stage::components::placement::Depth,
};
use bevy::prelude::*;
use cween::linear::components::{
    TargetingValueX, TargetingValueY, TargetingValueZ, TweenChildAcceleratedBundle,
    TweenChildBundle,
};
use leafwing_input_manager::prelude::ActionState;
use seldom_pixel::prelude::{PxCamera, PxSprite, PxSubPosition};

const BOMB_THROW_INITIAL_SPEED_Y: f32 = 80.0;
const BOMB_THROW_MIN_DURATION_SECS: f32 = 0.05;
const BOMB_THROW_START_DEPTH: Depth = Depth::One;
const BOMB_THROW_END_DEPTH: Depth = Depth::Six;

/// Marker for tween children that move bomb throws along their arc.
#[derive(Component, Clone, Debug)]
pub struct BombThrowTween;

/// @system Clamps the player position to the visible screen bounds.
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

/// @system Moves the player according to directional input.
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

/// @system Reads attack inputs (press/hold/release) and spawns player attacks.
#[allow(clippy::too_many_arguments)]
pub fn detect_player_attack(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    asset_server: Res<AssetServer>,
    gb_input: Res<ActionState<GBInput>>,
    player_attack_query: Query<Entity, With<PlayerAttack>>,
    player_query: Query<&PxSubPosition, With<Player>>,
    camera: Res<PxCamera>,
    volume_settings: Res<VolumeSettings>,
    time: Res<Time<StageTimeDomain>>,
    attack_definitions: Res<AttackDefinitions>,
    mut loadout: ResMut<AttackLoadout>,
    mut input_state: ResMut<AttackInputState>,
) {
    let now = time.elapsed_secs();
    let attack_active = player_attack_query.iter().next().is_some();

    let player_position = player_query.single().ok();
    let camera_offset = camera.0.as_vec2();

    if input_state.active_button.is_none() {
        if gb_input.just_pressed(&GBInput::A) {
            let world_position = player_position.map(|position| position.0 + camera_offset);
            input_state.arm(AttackButton::A, now, world_position);
        } else if gb_input.just_pressed(&GBInput::B) {
            let world_position = player_position.map(|position| position.0 + camera_offset);
            input_state.arm(AttackButton::B, now, world_position);
        }
    }

    if gb_input.just_pressed(&GBInput::Select) {
        if let Some(button) = input_state.active_button {
            loadout.cycle(button);
            input_state.mark_cycled();
        }
    }

    let Some(button) = input_state.active_button else {
        return;
    };

    let button_input = button.gb_input();
    let still_pressed = gb_input.pressed(&button_input);
    let attack_id = loadout.current(button);
    let definition = attack_definitions.get(attack_id);

    let mut spawn_attack = |spawn_position: Vec2, origin_position: Vec2| {
        let mut spawn_position = spawn_position;
        if definition.aim_spread > 0.0 {
            let spread = definition.aim_spread;
            let offset = Vec2::new(
                (rand::random::<f32>() * 2.0 - 1.0) * spread,
                (rand::random::<f32>() * 2.0 - 1.0) * spread,
            );
            spawn_position += offset;
        }
        let player_attack = PlayerAttack {
            position: if attack_id == AttackId::Bomb {
                origin_position
            } else {
                spawn_position
            },
            attack_id,
        };
        let (attack_bundle, sound_bundle) = player_attack.make_bundles(
            definition,
            &mut assets_sprite,
            asset_server.as_ref(),
            volume_settings.as_ref(),
        );
        let attack_entity = commands.spawn(attack_bundle).id();
        if let Some(sound_bundle) = sound_bundle {
            commands.spawn(sound_bundle);
        }

        if attack_id == AttackId::Bomb {
            let t = definition.duration_secs.max(BOMB_THROW_MIN_DURATION_SECS);
            let delta = spawn_position - origin_position;
            let speed_x = delta.x / t;
            let speed_y = BOMB_THROW_INITIAL_SPEED_Y;
            let adjusted_delta_y = delta.y - speed_y * t;
            let acceleration_y = 2.0 * adjusted_delta_y / (t * t);
            let start_depth = BOMB_THROW_START_DEPTH.to_f32();
            let end_depth = BOMB_THROW_END_DEPTH.to_f32();
            let speed_z = (end_depth - start_depth) / t;

            commands.entity(attack_entity).insert((
                BOMB_THROW_START_DEPTH,
                TargetingValueX::new(origin_position.x),
                TargetingValueY::new(origin_position.y),
                TargetingValueZ::new(start_depth),
            ));

            commands.spawn((
                TweenChildBundle::<StageTimeDomain, TargetingValueX>::new(
                    attack_entity,
                    origin_position.x,
                    spawn_position.x,
                    speed_x,
                ),
                BombThrowTween,
                Name::new("Bomb Throw Tween X"),
            ));

            commands.spawn((
                TweenChildAcceleratedBundle::<StageTimeDomain, TargetingValueY>::new(
                    attack_entity,
                    origin_position.y,
                    spawn_position.y,
                    speed_y,
                    acceleration_y,
                ),
                BombThrowTween,
                Name::new("Bomb Throw Tween Y"),
            ));

            commands.spawn((
                TweenChildBundle::<StageTimeDomain, TargetingValueZ>::new(
                    attack_entity,
                    start_depth,
                    end_depth,
                    speed_z,
                ),
                BombThrowTween,
                Name::new("Bomb Throw Tween Z"),
            ));
        }
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
                if let Some(position) = player_position {
                    spawn_attack(position.0, position.0);
                    input_state.mark_hold_fired(now);
                }
            }
        }
    }

    if gb_input.just_released(&button_input) {
        let should_fire = !input_state.cycled
            && match definition.input_policy {
                AttackInputPolicy::Release => true,
                AttackInputPolicy::Hold { .. } => !input_state.hold_fired,
            };

        if should_fire && !attack_active {
            if let Some(position) = player_position {
                let current_world = position.0 + camera_offset;
                let (spawn_position, origin_position) = if attack_id == AttackId::Bomb {
                    let origin = input_state.pressed_world_position.unwrap_or(current_world);
                    (current_world, origin)
                } else {
                    (position.0, position.0)
                };
                spawn_attack(spawn_position, origin_position);
            }
        }

        input_state.clear();
    } else if !still_pressed {
        input_state.clear();
    }
}

/// @system Advances attack lifetime timers each frame.
pub fn tick_attack_lifetimes(
    mut query: Query<&mut AttackLifetime, With<PlayerAttack>>,
    time: Res<Time<StageTimeDomain>>,
) {
    for mut lifetime in &mut query {
        lifetime.timer.tick(time.delta());
    }
}

/// @system Despawns attacks whose lifetime has expired, triggering follow-up effects.
pub fn despawn_expired_attacks(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
    attack_definitions: Res<AttackDefinitions>,
    mut query: Query<
        (
            Entity,
            &PlayerAttack,
            &AttackLifetime,
            &mut AttackEffectState,
            &PxSubPosition,
        ),
        With<PlayerAttack>,
    >,
) {
    for (entity, attack, lifetime, mut effect_state, position) in &mut query {
        if !lifetime.timer.is_finished() {
            continue;
        }

        let definition = attack_definitions.get(attack.attack_id);
        if definition.effects.screen_shake && !effect_state.screen_shake_triggered {
            commands.trigger(CameraShakeEvent);
            effect_state.screen_shake_triggered = true;
        }
        if !effect_state.follow_up_spawned {
            if let Some(next_id) = definition.spawn_on_expire {
                let next_definition = attack_definitions.get(next_id);
                let next_attack = PlayerAttack {
                    position: position.0,
                    attack_id: next_id,
                };
                let (attack_bundle, sound_bundle) = next_attack.make_bundles(
                    next_definition,
                    &mut assets_sprite,
                    asset_server.as_ref(),
                    volume_settings.as_ref(),
                );
                commands.spawn(attack_bundle);
                if let Some(sound_bundle) = sound_bundle {
                    commands.spawn(sound_bundle);
                }
                effect_state.follow_up_spawned = true;
            }
        }
        commands.entity(entity).insert(DespawnMark);
    }
}
