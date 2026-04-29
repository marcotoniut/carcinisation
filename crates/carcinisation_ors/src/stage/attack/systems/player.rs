use bevy::prelude::*;
use carapace::prelude::*;

use crate::stage::collision::AttackMask;
use crate::stubs::Score;
use crate::{
    assets::CxAssets,
    stage::{
        attack::components::{
            SCORE_MELEE_CRITICAL_HIT, SCORE_MELEE_REGULAR_HIT, SCORE_RANGED_CRITICAL_HIT,
            SCORE_RANGED_REGULAR_HIT,
        },
        collision::{
            CollisionTarget, MaskCollisionAssets, TargetCollisionHit, TargetCollisionResult,
            build_attack_mask, resolve_target_mask_hit, resolve_target_point_hit,
        },
        components::{
            interactive::{ColliderData, Hittable},
            placement::Depth,
        },
        enemy::{
            components::Enemy,
            composed::{ComposedAtlasBindings, ComposedCollisionState, ComposedResolvedParts},
        },
        messages::{DamageMessage, PartDamageMessage},
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
use carcinisation_collision::{
    AtlasMaskFrames, PixelMaskSource, WorldMaskInstance, world_mask_rect_from_spatial,
};
use carcinisation_core::components::DespawnMark;
use carcinisation_core::components::VolumeSettings;
use colored::Colorize;

const CRITICAL_THRESHOLD: f32 = 0.5;
const MELEE_DEPTH_MIN: crate::stage::components::placement::Depth =
    crate::stage::components::placement::Depth::One;
const MELEE_DEPTH_MAX: crate::stage::components::placement::Depth =
    crate::stage::components::placement::Depth::Three;

/// @system Checks player attacks against hittable entities.
///
/// Collision resolution is delegated to the shared stage collision pipeline:
/// player attacks provide world-space probe shapes and targets resolve
/// sprite/atlas/composed/fallback collision through a single API.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn check_got_hit(
    mut commands: Commands,
    mut assets_sprite: CxAssets<CxSprite>,
    asset_server: Res<AssetServer>,
    camera: Res<CxCamera>,
    mut collision_assets: MaskCollisionAssets<'_, '_>,
    mut event_writer: MessageWriter<DamageMessage>,
    mut part_event_writer: MessageWriter<PartDamageMessage>,
    time: Res<Time<StageTimeDomain>>,
    attack_definitions: Res<AttackDefinitions>,
    volume_settings: Res<VolumeSettings>,
    mut attack_query: Query<(
        Entity,
        &PlayerAttack,
        &CxPosition,
        &CxAnchor,
        &CxRenderSpace,
        Option<&CxFrameView>,
        Option<&CxSprite>,
        Option<&CxAtlasSprite>,
        &mut AttackHitTracker,
        &mut AttackEffectState,
        Option<&Depth>,
    )>,
    hittable_query: Query<
        (
            Entity,
            &CxPosition,
            &WorldPos,
            (&CxAnchor, &CxRenderSpace),
            (
                Option<&CxFrameView>,
                Option<&CxSprite>,
                Option<&CxAtlasSprite>,
                Option<&CxPresentationTransform>,
            ),
            Option<&ColliderData>,
            Option<&ComposedCollisionState>,
            Option<&ComposedResolvedParts>,
            Option<&ComposedAtlasBindings>,
            Option<&Depth>,
            Option<&Enemy>,
            Option<&crate::stage::destructible::components::Destructible>,
        ),
        With<Hittable>,
    >,
    mut score: ResMut<Score>,
) {
    collision_assets.refresh();
    let delta_secs = time.delta().as_secs_f32();
    let camera_world = camera.0;

    for (
        attack_entity,
        attack,
        attack_position,
        attack_anchor,
        attack_canvas,
        attack_frame,
        attack_sprite,
        attack_atlas_sprite,
        mut hit_tracker,
        mut effect_state,
        bomb_depth,
    ) in &mut attack_query
    {
        hit_tracker.tick(delta_secs);
        let attack_definition = attack_definitions.get(attack.attack_id);
        if matches!(attack_definition.collision, AttackCollisionMode::None) {
            continue;
        }

        let attack_world = match *attack_canvas {
            CxRenderSpace::World => attack_position.0,
            CxRenderSpace::Camera => attack_position.0 + camera_world,
        }
        .as_vec2();

        // Resolve sprite mask data. Store Arc-owned data locally so the mask
        // can borrow from it for the duration of the target loop.
        let is_sprite_mask = matches!(attack_definition.collision, AttackCollisionMode::SpriteMask);
        let attack_sprite_data = is_sprite_mask
            .then(|| attack_sprite.and_then(|s| collision_assets.sprite_pixels(&s.0)))
            .flatten();
        let (attack_atlas_data, attack_atlas_region_size, attack_atlas_region) = if is_sprite_mask
            && attack_sprite_data.is_none()
            && let Some(atlas_sprite) = attack_atlas_sprite
        {
            let data = collision_assets.atlas_pixels(&atlas_sprite.atlas);
            let region_size = collision_assets.atlas_sprite_region_size(atlas_sprite);
            // Clone the region so we don't hold an immutable borrow on
            // collision_assets through the target loop.
            let region = collision_assets.atlas_sprite_region(atlas_sprite).cloned();
            (data, region_size, region)
        } else {
            (None, None, None)
        };

        let attack_mask = if let Some(ref data) = attack_sprite_data {
            world_mask_rect_from_spatial(
                data.frame_size(),
                *attack_position,
                *attack_anchor,
                *attack_canvas,
                camera_world,
                None,
            )
            .map(|world| build_attack_mask(data, attack_frame.copied(), world.rect))
        } else if let Some(ref atlas_data) = attack_atlas_data
            && let Some(region_size) = attack_atlas_region_size
            && let Some(ref region) = attack_atlas_region
        {
            world_mask_rect_from_spatial(
                region_size,
                *attack_position,
                *attack_anchor,
                *attack_canvas,
                camera_world,
                None,
            )
            .map(|world| AttackMask {
                mask: WorldMaskInstance {
                    source: PixelMaskSource::Atlas {
                        atlas: atlas_data.as_ref(),
                        frames: AtlasMaskFrames::Region(region),
                    },
                    frame: attack_frame.copied(),
                    world: carcinisation_collision::WorldMaskRect {
                        rect: world.rect,
                        flip_x: false,
                        flip_y: false,
                    },
                    closed: false,
                },
            })
        } else {
            None
        };
        if is_sprite_mask && attack_mask.is_none() {
            continue;
        }

        let attack_points = if matches!(attack_definition.collision, AttackCollisionMode::Point) {
            let offsets = if attack_definition.hit_offsets.is_empty() {
                &[IVec2::ZERO][..]
            } else {
                attack_definition.hit_offsets.as_slice()
            };
            Some(
                offsets
                    .iter()
                    .map(|offset| attack_world + offset.as_vec2())
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };

        for (
            entity,
            entity_position,
            entity_world_pos,
            (entity_anchor, entity_canvas),
            (entity_frame, entity_sprite, entity_atlas_sprite, entity_presentation),
            collider_data,
            composed_collision_state,
            composed_resolved_parts,
            composed_atlas_bindings,
            entity_depth,
            enemy,
            destructible,
        ) in hittable_query
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

            let target = CollisionTarget {
                position: entity_position,
                world_pos: entity_world_pos,
                anchor: entity_anchor,
                canvas: entity_canvas,
                frame: entity_frame,
                sprite: entity_sprite,
                atlas_sprite: entity_atlas_sprite,
                presentation: entity_presentation,
                collider_data,
                composed_collision_state,
                composed_resolved_parts,
                composed_atlas_bindings,
                enemy,
                destructible,
            };

            let collision = match attack_definition.collision {
                AttackCollisionMode::Point => resolve_target_point_hit(
                    target,
                    attack_points
                        .as_deref()
                        .expect("point collisions always build point probes"),
                    camera_world,
                    &mut collision_assets,
                ),
                AttackCollisionMode::SpriteMask => resolve_target_mask_hit(
                    target,
                    attack_mask.expect("SpriteMask attacks skip the loop when mask is None"),
                    camera_world,
                    &mut collision_assets,
                ),
                AttackCollisionMode::Radial { radius } => {
                    // Visual-space position: include collision_offset so the
                    // radial check aligns with what the player sees.
                    let target_pos = entity_world_pos.0
                        + entity_presentation.map_or(Vec2::ZERO, |p| p.collision_offset);
                    let distance = attack_world.distance(target_pos);
                    if distance <= radius {
                        TargetCollisionResult {
                            evaluated: true,
                            hit: Some(TargetCollisionHit {
                                hit_position: target_pos,
                                defense: target
                                    .collider_data
                                    .and_then(|data| data.point_collides(target_pos, attack_world))
                                    .map_or(1.0, |c| c.defense),
                                semantic_part: None,
                            }),
                        }
                    } else {
                        TargetCollisionResult {
                            evaluated: true,
                            hit: None,
                        }
                    }
                }
                AttackCollisionMode::None => continue,
            };

            if !collision.evaluated {
                continue;
            }

            let Some(hit) = collision.hit else {
                continue;
            };

            if attack_definition.category == AttackCategory::Melee
                && let Some(depth) = entity_depth
                && (*depth < MELEE_DEPTH_MIN || *depth > MELEE_DEPTH_MAX)
            {
                continue;
            }

            if attack_definition.detonates_on_hit {
                if !effect_state.follow_up_spawned {
                    if let Some(next_id) = attack_definition.spawn_on_expire {
                        let next_definition = attack_definitions.get(next_id);
                        let next_attack = PlayerAttack {
                            position: attack_world,
                            attack_id: next_id,
                        };
                        next_attack.spawn_attack(
                            &mut commands,
                            next_definition,
                            &mut assets_sprite,
                            asset_server.as_ref(),
                            collision_assets.atlas_asset_store(),
                            volume_settings.as_ref(),
                        );
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
            let damage = (damage as f32 / hit.defense) as u32;

            if let Some(part_id) = hit.semantic_part {
                part_event_writer.write(PartDamageMessage::new(entity, part_id, damage));
            } else {
                event_writer.write(DamageMessage::new(entity, damage));
            }

            if attack_definition.effects.screen_shake && !effect_state.screen_shake_triggered {
                commands.trigger(CameraShakeEvent);
                effect_state.screen_shake_triggered = true;
            }

            match attack_definition.category {
                AttackCategory::Melee => {
                    if hit.defense <= CRITICAL_THRESHOLD {
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
                    if hit.defense <= CRITICAL_THRESHOLD {
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
