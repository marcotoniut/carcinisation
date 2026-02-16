use bevy::asset::AssetEvent;
use bevy::prelude::*;
use carcinisation_collision::pixel_mask::{
    PixelCollisionCache, mask_contains_point, pixel_overlap, sprite_data, sprite_rect,
};
use seldom_pixel::prelude::*;

use crate::{
    components::{DespawnMark, VolumeSettings},
    game::score::components::Score,
    pixel::PxAssets,
    stage::{
        attack::components::*,
        components::interactive::{ColliderData, Hittable},
        components::placement::Depth,
        enemy::components::Enemy,
        messages::DamageMessage,
        player::components::PlayerAttack,
        player::{
            attacks::{
                AttackCategory, AttackCollisionMode, AttackDefinitions, AttackEffectState,
                AttackHitPolicy, AttackHitTracker, AttackId,
            },
            messages::CameraShakeEvent,
        },
        resources::StageTimeDomain,
    },
};
use colored::*;

const CRITICAL_THRESHOLD: f32 = 0.5;
const MELEE_DEPTH_MIN: crate::stage::components::placement::Depth =
    crate::stage::components::placement::Depth::One;
const MELEE_DEPTH_MAX: crate::stage::components::placement::Depth =
    crate::stage::components::placement::Depth::Three;

/// @system Checks player attacks against hittable entities using pixel-mask and collider tests.
// TODO could split between box and circle collider
#[allow(clippy::too_many_arguments)]
pub fn check_got_hit(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    asset_server: Res<AssetServer>,
    camera: Res<PxCamera>,
    sprite_assets: Res<Assets<PxSpriteAsset>>,
    mut asset_events: MessageReader<AssetEvent<PxSpriteAsset>>,
    mut event_writer: MessageWriter<DamageMessage>,
    time: Res<Time<StageTimeDomain>>,
    attack_definitions: Res<AttackDefinitions>,
    volume_settings: Res<VolumeSettings>,
    mut attack_query: Query<(
        Entity,
        &PlayerAttack,
        &PxPosition,
        &PxAnchor,
        &PxCanvas,
        Option<&PxFrameView>,
        &PxSprite,
        &mut AttackHitTracker,
        &mut AttackEffectState,
        Option<&Depth>,
    )>,
    // mut attack_query: Query<(&PlayerAttack, &mut AttackHitTracker, Option<&Reach>)>,
    mut hittable_query: Query<
        (
            Entity,
            &PxPosition,
            &PxSubPosition,
            &PxAnchor,
            &PxCanvas,
            Option<&PxFrameView>,
            Option<&PxSprite>,
            Option<&ColliderData>,
            Option<&Depth>,
            Option<&Enemy>,
            Option<&crate::stage::destructible::components::Destructible>,
        ),
        With<Hittable>,
    >,
    mut score: ResMut<Score>,
    mut cache: Local<PixelCollisionCache>,
) {
    if asset_events.read().next().is_some() {
        cache.clear();
    }

    let delta_secs = time.delta().as_secs_f32();

    for (
        attack_entity,
        attack,
        attack_position,
        attack_anchor,
        attack_canvas,
        attack_frame,
        attack_sprite,
        mut hit_tracker,
        mut effect_state,
        bomb_depth,
    ) in attack_query.iter_mut()
    {
        hit_tracker.tick(delta_secs);
        let attack_definition = attack_definitions.get(attack.attack_id);
        if matches!(attack_definition.collision, AttackCollisionMode::None) {
            continue;
        }
        let attack_data = if matches!(attack_definition.collision, AttackCollisionMode::SpriteMask)
        {
            sprite_data(&mut cache, &sprite_assets, attack_sprite)
        } else {
            None
        };
        let attack_rect = attack_data.as_deref().map(|data| {
            sprite_rect(
                data.frame_size(),
                *attack_position,
                *attack_anchor,
                *attack_canvas,
                camera.0,
            )
        });

        let attack_screen = match *attack_canvas {
            PxCanvas::World => attack_position.0 - camera.0,
            PxCanvas::Camera => attack_position.0,
        };
        let attack_world = match *attack_canvas {
            PxCanvas::World => attack_position.0,
            PxCanvas::Camera => attack_position.0 + camera.0,
        };
        let attack_world = attack_world.as_vec2();

        for (
            entity,
            entity_position,
            entity_sub_position,
            entity_anchor,
            entity_canvas,
            entity_frame,
            entity_sprite,
            collider_data,
            entity_depth,
            enemy,
            destructible,
        ) in hittable_query.iter_mut()
        {
            if attack.attack_id == AttackId::Bomb {
                let depth_match = match (bomb_depth, entity_depth) {
                    (Some(attack_depth), Some(entity_depth)) => attack_depth == entity_depth,
                    _ => false,
                };
                let depth_reached = matches!(bomb_depth, Some(depth) if *depth == Depth::Six);
                if !depth_match && !depth_reached {
                    continue;
                }
            }

            let mut hit = None;
            let mut evaluated = false;
            let wants_pixel = destructible.is_none() && entity_sprite.is_some();
            let entity_data = if wants_pixel {
                // TODO: allow opting into a dedicated collision sprite/mask component.
                entity_sprite.and_then(|sprite| sprite_data(&mut cache, &sprite_assets, sprite))
            } else {
                None
            };
            if let Some(entity_data) = entity_data {
                let entity_rect = sprite_rect(
                    entity_data.frame_size(),
                    *entity_position,
                    *entity_anchor,
                    *entity_canvas,
                    camera.0,
                );

                if matches!(attack_definition.collision, AttackCollisionMode::Point) {
                    evaluated = true;
                    if mask_contains_point(
                        entity_data.as_ref(),
                        entity_frame.copied(),
                        entity_rect,
                        attack_screen,
                    ) {
                        hit = Some(match *entity_canvas {
                            PxCanvas::World => (attack_screen + camera.0).as_vec2(),
                            PxCanvas::Camera => attack_screen.as_vec2(),
                        });
                    }
                } else if let (Some(attack_data), Some(attack_rect)) =
                    (attack_data.as_deref(), attack_rect)
                {
                    evaluated = true;
                    hit = pixel_overlap(
                        attack_data,
                        attack_frame.copied(),
                        attack_rect,
                        entity_data.as_ref(),
                        entity_frame.copied(),
                        entity_rect,
                    )
                    .map(|screen_pos| match *entity_canvas {
                        PxCanvas::World => (screen_pos + camera.0).as_vec2(),
                        PxCanvas::Camera => screen_pos.as_vec2(),
                    });
                }
            }

            if hit.is_none() && !evaluated {
                if let Some(collider_data) = collider_data {
                    if collider_data
                        .point_collides(entity_sub_position.0, attack_world)
                        .is_some()
                    {
                        hit = Some(attack_world);
                    }
                    evaluated = true;
                }
            }

            if !evaluated && enemy.is_some() {
                // If we couldn't evaluate pixel data yet, fall back to collider checks for enemies.
                if let Some(collider_data) = collider_data {
                    if collider_data
                        .point_collides(entity_sub_position.0, attack_world)
                        .is_some()
                    {
                        hit = Some(attack_world);
                    }
                    evaluated = true;
                }
            }

            if !evaluated {
                continue;
            }

            let Some(hit_position) = hit else {
                continue;
            };

            if attack_definition.category == AttackCategory::Melee {
                if let Some(depth) = entity_depth {
                    if *depth < MELEE_DEPTH_MIN || *depth > MELEE_DEPTH_MAX {
                        continue;
                    }
                }
            }

            if attack_definition.detonates_on_hit {
                if !effect_state.follow_up_spawned {
                    if let Some(next_id) = attack_definition.spawn_on_expire {
                        let next_definition = attack_definitions.get(next_id);
                        let next_attack = PlayerAttack {
                            position: attack_world,
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
                    }
                    effect_state.follow_up_spawned = true;
                }

                if attack_definition.effects.screen_shake && !effect_state.screen_shake_triggered {
                    commands.trigger(CameraShakeEvent);
                    effect_state.screen_shake_triggered = true;
                }

                commands.entity(attack_entity).insert(DespawnMark);
                break;
            }

            let defense = collider_data
                .and_then(|data| data.point_collides(entity_sub_position.0, hit_position))
                .map(|value| value.defense)
                .unwrap_or(1.0);

            if !hit_tracker.can_hit(entity, attack_definition.hit_policy) {
                continue;
            }

            let damage = match attack_definition.hit_policy {
                AttackHitPolicy::Single => attack_definition.damage,
                AttackHitPolicy::Repeat { repeat_damage, .. } => {
                    if hit_tracker.has_hit(entity) {
                        repeat_damage
                    } else {
                        attack_definition.damage
                    }
                }
            };

            hit_tracker.register_hit(entity, attack_definition.hit_policy);
            event_writer.write(DamageMessage::new(entity, (damage as f32 / defense) as u32));

            if attack_definition.effects.screen_shake && !effect_state.screen_shake_triggered {
                commands.trigger(CameraShakeEvent);
                effect_state.screen_shake_triggered = true;
            }

            match attack_definition.category {
                AttackCategory::Melee => {
                    if defense <= CRITICAL_THRESHOLD {
                        score.add_u(SCORE_MELEE_CRITICAL_HIT);

                        #[cfg(debug_assertions)]
                        println!("{} Melee ***CRITICAL***", "HIT".yellow());
                    } else {
                        score.add_u(SCORE_MELEE_REGULAR_HIT);

                        #[cfg(debug_assertions)]
                        println!("{} Melee", "HIT".yellow());
                    }
                }
                AttackCategory::Ranged => {
                    if defense <= CRITICAL_THRESHOLD {
                        score.add_u(SCORE_RANGED_CRITICAL_HIT);

                        #[cfg(debug_assertions)]
                        println!("{} Ranged ***CRITICAL***", "HIT".yellow());
                    } else {
                        score.add_u(SCORE_RANGED_REGULAR_HIT);

                        #[cfg(debug_assertions)]
                        println!("{} Ranged", "HIT".yellow());
                    }
                }
            }
        }
    }
}
