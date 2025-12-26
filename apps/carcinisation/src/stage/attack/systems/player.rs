use bevy::asset::AssetEvent;
use bevy::prelude::*;
use carcinisation_collision::pixel_mask::{
    mask_contains_point, pixel_overlap, sprite_data, sprite_rect, PixelCollisionCache,
};
use seldom_pixel::prelude::*;

use crate::{
    game::score::components::Score,
    stage::{
        attack::components::*,
        components::interactive::{ColliderData, Hittable},
        enemy::components::Enemy,
        messages::DamageMessage,
        player::components::PlayerAttack,
        player::{
            attacks::{
                AttackCategory, AttackCollisionMode, AttackDefinitions, AttackEffectState,
                AttackHitPolicy, AttackHitTracker,
            },
            messages::CameraShakeEvent,
        },
        resources::StageTimeDomain,
    },
};
use colored::*;

const CRITICAL_THRESHOLD: f32 = 0.5;

/**
 * Could split between box and circle collider
 */
pub fn check_got_hit(
    mut commands: Commands,
    camera: Res<PxCamera>,
    sprite_assets: Res<Assets<PxSpriteAsset>>,
    mut asset_events: MessageReader<AssetEvent<PxSpriteAsset>>,
    mut event_writer: MessageWriter<DamageMessage>,
    time: Res<Time<StageTimeDomain>>,
    attack_definitions: Res<AttackDefinitions>,
    mut attack_query: Query<(
        &PlayerAttack,
        &PxPosition,
        &PxAnchor,
        &PxCanvas,
        Option<&PxFrameView>,
        &PxSprite,
        &mut AttackHitTracker,
        &mut AttackEffectState,
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
        attack,
        attack_position,
        attack_anchor,
        attack_canvas,
        attack_frame,
        attack_sprite,
        mut hit_tracker,
        mut effect_state,
    ) in attack_query.iter_mut()
    {
        hit_tracker.tick(delta_secs);
        let attack_definition = attack_definitions.get(attack.attack_id);
        let attack_data = sprite_data(&mut cache, &sprite_assets, attack_sprite);
        let attack_rect = attack_data.as_deref().map(|data| {
            sprite_rect(
                data.frame_size(),
                *attack_position,
                *attack_anchor,
                *attack_canvas,
                **camera,
            )
        });

        let attack_screen = match *attack_canvas {
            PxCanvas::World => **attack_position - **camera,
            PxCanvas::Camera => **attack_position,
        };
        let attack_world = match *attack_canvas {
            PxCanvas::World => **attack_position,
            PxCanvas::Camera => **attack_position + **camera,
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
            enemy,
            destructible,
        ) in hittable_query.iter_mut()
        {
            let mut hit = None;
            let mut evaluated = false;
            let wants_pixel = destructible.is_none() && entity_sprite.is_some();
            if wants_pixel {
                // TODO: allow opting into a dedicated collision sprite/mask component.
                if let Some(entity_data) =
                    entity_sprite.and_then(|sprite| sprite_data(&mut cache, &sprite_assets, sprite))
                {
                    let entity_rect = sprite_rect(
                        entity_data.frame_size(),
                        *entity_position,
                        *entity_anchor,
                        *entity_canvas,
                        **camera,
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
                                PxCanvas::World => (attack_screen + **camera).as_vec2(),
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
                            PxCanvas::World => (screen_pos + **camera).as_vec2(),
                            PxCanvas::Camera => screen_pos.as_vec2(),
                        });
                    }
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
