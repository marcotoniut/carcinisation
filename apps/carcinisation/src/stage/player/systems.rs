pub mod camera;
pub mod messages;

use super::attacks::{
    AttackDefinitions, AttackId, AttackInputPolicy, AttackInputState, AttackLifetime, AttackLoadout,
};
use super::components::{PLAYER_SIZE, Player, PlayerAttack, Webbed};
use super::config::PlayerConfig;
use super::intent::PlayerIntent;
use crate::pixel::CxAssets;
use crate::stage::player::attacks::AttackEffectState;
use crate::stage::player::messages::CameraShakeEvent;
use crate::stage::resources::StageTimeDomain;
use crate::{
    components::{DespawnMark, VolumeSettings},
    globals::{HUD_HEIGHT, SCREEN_RESOLUTION},
    stage::components::placement::Depth,
};
use bevy::prelude::*;
use carapace::prelude::{CxCamera, CxSprite, WorldPos};
use cween::linear::components::{
    TargetingValueX, TargetingValueY, TargetingValueZ, TweenChildAcceleratedBundle,
    TweenChildBundle,
};

const BOMB_THROW_INITIAL_SPEED_Y: f32 = 80.0;
const BOMB_THROW_MIN_DURATION_SECS: f32 = 0.05;
const BOMB_THROW_START_DEPTH: Depth = Depth::One;
const BOMB_THROW_END_DEPTH: Depth = Depth::Six;

/// Marker for tween children that move bomb throws along their arc.
#[derive(Component, Clone, Debug)]
pub struct BombThrowTween;

/// @system Clamps the player position to the visible screen bounds.
pub fn confine_player_movement(mut player_query: Query<&mut WorldPos, With<Player>>) {
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

/// @system Moves the player according to resolved intent (direction + slow modifier).
///
/// When the player has a [`Webbed`] debuff, the webbed speed multiplier
/// overrides the normal SHIFT slow modifier — the slower of the two is NOT
/// used; the webbed multiplier always wins.
pub fn player_movement(
    intent: Res<PlayerIntent>,
    config: Res<PlayerConfig>,
    mut query: Query<(&mut WorldPos, Option<&Webbed>), With<Player>>,
    time: Res<Time<StageTimeDomain>>,
) {
    if intent.move_direction == Vec2::ZERO {
        return;
    }
    for (mut position, webbed) in &mut query {
        let speed = if let Some(webbed) = webbed {
            config.base_speed * webbed.speed_multiplier
        } else if intent.slow_modifier {
            config.base_speed * config.slow_modifier
        } else {
            config.base_speed
        };
        position.0 += intent.move_direction * speed * time.delta().as_secs_f32();
    }
}

/// @system Removes expired [`Webbed`] components from the player.
pub fn tick_webbed_status(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    query: Query<(Entity, &Webbed), With<Player>>,
) {
    for (entity, webbed) in &query {
        if stage_time.elapsed() >= webbed.expires_at {
            commands.entity(entity).remove::<Webbed>();
        }
    }
}

/// @system Reads resolved [`PlayerIntent`] and spawns player attacks.
///
/// Melee (Pincer) is triggered directly by the Select+A chord.
/// Ranged attacks follow the arm → hold/release cycle driven by the A button.
/// Item select cycles the ranged loadout.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn detect_player_attack(
    mut commands: Commands,
    mut assets_sprite: CxAssets<CxSprite>,
    asset_server: Res<AssetServer>,
    intent: Res<PlayerIntent>,
    player_attack_query: Query<Entity, With<PlayerAttack>>,
    player_query: Query<&WorldPos, With<Player>>,
    camera: Res<CxCamera>,
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

    // --- Item select (Select resolved without A) ---
    if intent.item_select_triggered {
        loadout.cycle();
    }

    // --- Melee (Select+A chord) ---
    if intent.melee_triggered {
        if !attack_active && let Some(position) = player_position {
            let definition = attack_definitions.get(AttackId::Pincer);
            let player_attack = PlayerAttack {
                position: position.0,
                attack_id: AttackId::Pincer,
            };
            let (attack_bundle, sound_bundle) = player_attack.make_bundles(
                definition,
                &mut assets_sprite,
                asset_server.as_ref(),
                volume_settings.as_ref(),
            );
            commands.spawn(attack_bundle);
            if let Some(sound_bundle) = sound_bundle {
                commands.spawn(sound_bundle);
            }
        }
        // The A press was consumed by melee — clear any armed ranged state.
        input_state.clear();
        return;
    }

    // --- Ranged shoot (A button via intent) ---
    if !input_state.armed && intent.shoot_just_pressed {
        let world_position = player_position.map(|p| p.0 + camera_offset);
        input_state.arm(now, world_position);
    }

    if !input_state.armed {
        return;
    }

    let attack_id = loadout.current();
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

    // Hold-fire (e.g. MachineGun).
    if intent.shoot_held
        && !attack_active
        && let AttackInputPolicy::Hold {
            warmup_secs,
            interval_secs,
        } = definition.input_policy
    {
        let held_for = now - input_state.pressed_at;
        let can_fire = input_state
            .last_hold_fire_at
            .is_none_or(|last| now - last >= interval_secs);
        if held_for >= warmup_secs
            && can_fire
            && let Some(position) = player_position
        {
            spawn_attack(position.0, position.0);
            input_state.mark_hold_fired(now);
        }
    }

    // Release-fire (Pistol, Bomb).
    if intent.shoot_just_released {
        let should_fire = match definition.input_policy {
            AttackInputPolicy::Release => true,
            AttackInputPolicy::Hold { .. } => !input_state.hold_fired,
        };

        if should_fire
            && !attack_active
            && let Some(position) = player_position
        {
            let current_world = position.0 + camera_offset;
            let (spawn_position, origin_position) = if attack_id == AttackId::Bomb {
                let origin = input_state.pressed_world_position.unwrap_or(current_world);
                (current_world, origin)
            } else {
                (position.0, position.0)
            };
            spawn_attack(spawn_position, origin_position);
        }

        input_state.clear();
    } else if !intent.shoot_held && input_state.armed {
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
    mut assets_sprite: CxAssets<CxSprite>,
    asset_server: Res<AssetServer>,
    volume_settings: Res<VolumeSettings>,
    attack_definitions: Res<AttackDefinitions>,
    mut query: Query<
        (
            Entity,
            &PlayerAttack,
            &AttackLifetime,
            &mut AttackEffectState,
            &WorldPos,
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
        if !effect_state.follow_up_spawned
            && let Some(next_id) = definition.spawn_on_expire
        {
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
        commands.entity(entity).insert(DespawnMark);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::player::components::{WEBBED_DURATION, WEBBED_SPEED_MULTIPLIER};
    use std::time::Duration;

    fn setup_movement_app() -> App {
        let mut app = App::new();
        app.insert_resource(Time::<StageTimeDomain>::default());
        app.insert_resource(PlayerConfig {
            base_speed: 100.0,
            slow_modifier: 0.5,
        });
        app.insert_resource(PlayerIntent {
            move_direction: Vec2::X,
            slow_modifier: false,
            ..default()
        });
        app.add_systems(Update, (tick_webbed_status, player_movement).chain());
        app
    }

    #[test]
    fn webbed_applies_speed_multiplier() {
        let mut app = setup_movement_app();
        let entity = app
            .world_mut()
            .spawn((Player, WorldPos(Vec2::ZERO), Webbed::new(Duration::ZERO)))
            .id();

        app.world_mut()
            .resource_mut::<Time<StageTimeDomain>>()
            .advance_by(Duration::from_secs(1));
        app.update();

        let pos = app.world().entity(entity).get::<WorldPos>().unwrap();
        let expected_speed = 100.0 * WEBBED_SPEED_MULTIPLIER;
        assert!(
            (pos.0.x - expected_speed).abs() < 1.0,
            "expected ~{expected_speed}, got {}",
            pos.0.x,
        );
    }

    #[test]
    fn webbed_overrides_shift_slow() {
        let mut app = setup_movement_app();
        app.world_mut().resource_mut::<PlayerIntent>().slow_modifier = true;
        let entity = app
            .world_mut()
            .spawn((Player, WorldPos(Vec2::ZERO), Webbed::new(Duration::ZERO)))
            .id();

        app.world_mut()
            .resource_mut::<Time<StageTimeDomain>>()
            .advance_by(Duration::from_secs(1));
        app.update();

        let pos = app.world().entity(entity).get::<WorldPos>().unwrap();
        // Webbed multiplier (0.4) should be used, NOT slow_modifier (0.5).
        let webbed_speed = 100.0 * WEBBED_SPEED_MULTIPLIER;
        assert!(
            (pos.0.x - webbed_speed).abs() < 1.0,
            "webbed should override SHIFT slow; expected ~{webbed_speed}, got {}",
            pos.0.x,
        );
    }

    #[test]
    fn webbed_expires_after_duration() {
        let mut app = setup_movement_app();
        let entity = app
            .world_mut()
            .spawn((Player, WorldPos(Vec2::ZERO), Webbed::new(Duration::ZERO)))
            .id();

        // Advance past the web duration.
        app.world_mut()
            .resource_mut::<Time<StageTimeDomain>>()
            .advance_by(WEBBED_DURATION + Duration::from_millis(1));
        app.update();

        assert!(
            !app.world().entity(entity).contains::<Webbed>(),
            "Webbed should be removed after duration expires"
        );
    }

    #[test]
    fn webbed_refresh_extends_duration() {
        let start = Duration::from_secs(1);
        let mut webbed = Webbed::new(start);
        let original_expires = webbed.expires_at;

        let refresh_time = start + Duration::from_secs(1);
        webbed.refresh(refresh_time);

        assert!(
            webbed.expires_at > original_expires,
            "refresh should extend expiry"
        );
        assert_eq!(webbed.expires_at, refresh_time + WEBBED_DURATION);
        // Multiplier should not change on refresh.
        assert!((webbed.speed_multiplier - WEBBED_SPEED_MULTIPLIER).abs() < f32::EPSILON);
    }

    #[test]
    fn normal_movement_without_webbed() {
        let mut app = setup_movement_app();
        let entity = app.world_mut().spawn((Player, WorldPos(Vec2::ZERO))).id();

        app.world_mut()
            .resource_mut::<Time<StageTimeDomain>>()
            .advance_by(Duration::from_secs(1));
        app.update();

        let pos = app.world().entity(entity).get::<WorldPos>().unwrap();
        assert!(
            (pos.0.x - 100.0).abs() < 1.0,
            "expected ~100, got {}",
            pos.0.x,
        );
    }
}
