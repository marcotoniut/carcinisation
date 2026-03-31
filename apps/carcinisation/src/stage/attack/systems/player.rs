use bevy::asset::AssetEvent;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use carapace::prelude::*;
use carcinisation_collision::pixel_mask::{
    AtlasPixelCollisionCache, PixelCollisionCache, atlas_data, atlas_region_contains_point,
    mask_contains_point, pixel_overlap, sprite_data, sprite_rect,
};

use crate::{
    components::{DespawnMark, VolumeSettings},
    game::score::components::Score,
    pixel::PxAssets,
    stage::{
        attack::components::{
            SCORE_MELEE_CRITICAL_HIT, SCORE_MELEE_REGULAR_HIT, SCORE_RANGED_CRITICAL_HIT,
            SCORE_RANGED_REGULAR_HIT,
        },
        components::interactive::{ColliderData, Hittable},
        components::placement::Depth,
        enemy::{
            components::Enemy,
            composed::{
                ComposedAtlasBindings, ComposedCollisionState, ComposedResolvedParts,
                ResolvedPartCollision, ResolvedPartState,
            },
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
use colored::Colorize;

const CRITICAL_THRESHOLD: f32 = 0.5;
const MELEE_DEPTH_MIN: crate::stage::components::placement::Depth =
    crate::stage::components::placement::Depth::One;
const MELEE_DEPTH_MAX: crate::stage::components::placement::Depth =
    crate::stage::components::placement::Depth::Three;

#[derive(SystemParam)]
pub(crate) struct PixelHitAssets<'w, 's> {
    sprite_assets: Res<'w, Assets<PxSpriteAsset>>,
    atlas_assets: Res<'w, Assets<PxSpriteAtlasAsset>>,
    sprite_asset_events: MessageReader<'w, 's, AssetEvent<PxSpriteAsset>>,
    atlas_asset_events: MessageReader<'w, 's, AssetEvent<PxSpriteAtlasAsset>>,
    sprite_cache: Local<'s, PixelCollisionCache>,
    atlas_cache: Local<'s, AtlasPixelCollisionCache>,
}

/// @system Checks player attacks against hittable entities using pixel-mask and collider tests.
// TODO could split between box and circle collider
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn check_got_hit(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    asset_server: Res<AssetServer>,
    camera: Res<PxCamera>,
    mut pixel_assets: PixelHitAssets,
    mut event_writer: MessageWriter<DamageMessage>,
    mut part_event_writer: MessageWriter<PartDamageMessage>,
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
    hittable_query: Query<
        (
            Entity,
            &PxPosition,
            &PxSubPosition,
            &PxAnchor,
            &PxCanvas,
            Option<&PxFrameView>,
            Option<&PxSprite>,
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
    if pixel_assets.sprite_asset_events.read().next().is_some() {
        pixel_assets.sprite_cache.clear();
    }
    if pixel_assets.atlas_asset_events.read().next().is_some() {
        pixel_assets.atlas_cache.clear();
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
    ) in &mut attack_query
    {
        hit_tracker.tick(delta_secs);
        let attack_definition = attack_definitions.get(attack.attack_id);
        if matches!(attack_definition.collision, AttackCollisionMode::None) {
            continue;
        }
        let attack_data = if matches!(attack_definition.collision, AttackCollisionMode::SpriteMask)
        {
            sprite_data(
                &mut pixel_assets.sprite_cache,
                &pixel_assets.sprite_assets,
                attack_sprite,
            )
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

            let mut hit = None;
            let mut evaluated = false;
            let wants_pixel = destructible.is_none() && entity_sprite.is_some();
            let entity_data = if wants_pixel {
                // TODO: allow opting into a dedicated collision sprite/mask component.
                entity_sprite.and_then(|sprite| {
                    sprite_data(
                        &mut pixel_assets.sprite_cache,
                        &pixel_assets.sprite_assets,
                        sprite,
                    )
                })
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

            if hit.is_none()
                && let Some(composed_collision_state) = composed_collision_state
            {
                evaluated = true;
                if composed_collision_state
                    .point_collides(attack_world)
                    .is_some()
                {
                    hit = Some(attack_world);
                }
            }

            if hit.is_none()
                && !evaluated
                && let Some(collider_data) = collider_data
            {
                if collider_data
                    .point_collides(entity_sub_position.0, attack_world)
                    .is_some()
                {
                    hit = Some(attack_world);
                }
                evaluated = true;
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

            let composed_hit = resolve_composed_hit_at_point(
                hit_position,
                composed_collision_state,
                composed_resolved_parts,
                composed_atlas_bindings,
                &pixel_assets.atlas_assets,
                &mut pixel_assets.atlas_cache,
            );
            let defense = composed_hit.as_ref().map_or_else(
                || {
                    collider_data
                        .and_then(|data| data.point_collides(entity_sub_position.0, hit_position))
                        .map_or(1.0, |value| value.defense)
                },
                |value| value.defense,
            );

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
            let damage = (damage as f32 / defense) as u32;
            if let Some(composed_hit) = composed_hit {
                part_event_writer.write(PartDamageMessage::new(
                    entity,
                    composed_hit.part_id,
                    damage,
                ));
            } else {
                event_writer.write(DamageMessage::new(entity, damage));
            }

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

#[derive(Clone, Debug, PartialEq)]
struct ComposedHitSelection {
    part_id: String,
    defense: f32,
}

/// Selects a semantic part from coarse composed collisions plus optional pixel refinement.
///
/// Policy:
/// - coarse collision volumes decide which parts are even eligible
/// - pixel-perfect atlas alpha disambiguates overlapping eligible parts
/// - visual draw order breaks ties and remains the fallback when no pixel hit resolves
fn resolve_composed_hit_at_point(
    hit_position: Vec2,
    collision_state: Option<&ComposedCollisionState>,
    resolved_parts: Option<&ComposedResolvedParts>,
    atlas_bindings: Option<&ComposedAtlasBindings>,
    atlas_assets: &Assets<PxSpriteAtlasAsset>,
    atlas_cache: &mut AtlasPixelCollisionCache,
) -> Option<ComposedHitSelection> {
    let collision_state = collision_state?;
    if let (Some(resolved_parts), Some(atlas_bindings)) = (resolved_parts, atlas_bindings)
        && let Some(atlas_pixels) =
            atlas_data(atlas_cache, atlas_assets, atlas_bindings.atlas_handle())
        && let Some(selected_part) = select_resolved_composed_part(
            collision_state.collisions(),
            resolved_parts.parts(),
            hit_position,
            |part| {
                let Some(region_rect) = atlas_bindings.sprite_rect(part.sprite_id.as_str()) else {
                    return false;
                };
                let sprite_rect = IRect {
                    min: part.world_top_left_position.round().as_ivec2(),
                    max: part.world_top_left_position.round().as_ivec2()
                        + part.frame_size.as_ivec2(),
                };
                atlas_region_contains_point(
                    atlas_pixels.as_ref(),
                    region_rect,
                    sprite_rect,
                    hit_position.round().as_ivec2(),
                    part.flip_x,
                    part.flip_y,
                )
            },
        )
        && let Some(collision) = part_collision_at_point(
            collision_state.collisions(),
            selected_part.part_id.as_str(),
            hit_position,
        )
    {
        return Some(ComposedHitSelection {
            part_id: selected_part.part_id.clone(),
            defense: collision.collider.defense,
        });
    }

    collision_state
        .point_collides(hit_position)
        .map(|collision| ComposedHitSelection {
            part_id: collision.part_id.clone(),
            defense: collision.collider.defense,
        })
}

fn select_resolved_composed_part<'a, F>(
    collisions: &[ResolvedPartCollision],
    resolved_parts: &'a [ResolvedPartState],
    hit_position: Vec2,
    mut pixel_contains: F,
) -> Option<&'a ResolvedPartState>
where
    F: FnMut(&ResolvedPartState) -> bool,
{
    let mut front_most_coarse = None;

    for part in resolved_parts.iter().rev() {
        if !part.targetable {
            continue;
        }
        if part_collision_at_point(collisions, part.part_id.as_str(), hit_position).is_none() {
            continue;
        }
        front_most_coarse.get_or_insert(part);
        if pixel_contains(part) {
            return Some(part);
        }
    }

    front_most_coarse
}

fn part_collision_at_point<'a>(
    collisions: &'a [ResolvedPartCollision],
    part_id: &str,
    hit_position: Vec2,
) -> Option<&'a ResolvedPartCollision> {
    collisions.iter().rev().find(|collision| {
        collision.part_id == part_id
            && collision.collider.shape.point_collides(
                collision.pivot_position + collision.collider.offset,
                hit_position,
            )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use carcinisation_collision::{Collider, ColliderShape};

    fn collisions_for(parts: &[(&str, f32)]) -> Vec<ResolvedPartCollision> {
        parts
            .iter()
            .map(|(part_id, defense)| ResolvedPartCollision {
                part_id: (*part_id).to_string(),
                collider: Collider::new(ColliderShape::Circle(4.0)).with_defense(*defense),
                pivot_position: Vec2::ZERO,
            })
            .collect()
    }

    fn resolved_parts_for(parts: &[(&str, u32, &str, bool)]) -> Vec<ResolvedPartState> {
        parts
            .iter()
            .map(
                |(part_id, draw_order, sprite_id, flip_x)| ResolvedPartState {
                    part_id: (*part_id).to_string(),
                    parent_id: None,
                    draw_order: *draw_order,
                    sprite_id: (*sprite_id).to_string(),
                    frame_size: UVec2::new(4, 4),
                    flip_x: *flip_x,
                    flip_y: false,
                    world_top_left_position: Vec2::new(-2.0, -2.0),
                    world_pivot_position: Vec2::ZERO,
                    tags: vec![],
                    targetable: true,
                    health_pool: Some("core".to_string()),
                    armour: 0,
                    current_durability: None,
                    max_durability: None,
                    breakable: false,
                    broken: false,
                    blinking: false,
                    collisions: vec![],
                },
            )
            .collect()
    }

    #[test]
    fn pixel_selection_overrides_front_most_coarse_part() {
        let collisions = collisions_for(&[("body", 1.0), ("head", 0.5)]);
        let parts =
            resolved_parts_for(&[("body", 10, "shared", false), ("head", 20, "shared", false)]);

        let selected = select_resolved_composed_part(&collisions, &parts, Vec2::ZERO, |part| {
            part.part_id == "body"
        })
        .expect("pixel refinement should choose a colliding part");

        assert_eq!(selected.part_id, "body");
    }

    #[test]
    fn front_most_coarse_part_wins_when_no_pixel_hit_resolves() {
        let collisions = collisions_for(&[("body", 1.0), ("head", 0.5)]);
        let parts =
            resolved_parts_for(&[("body", 10, "shared", false), ("head", 20, "shared", false)]);

        let selected = select_resolved_composed_part(&collisions, &parts, Vec2::ZERO, |_| false)
            .expect("coarse fallback should still choose the front-most part");

        assert_eq!(selected.part_id, "head");
    }

    #[test]
    fn shared_sprite_semantics_remain_distinct_during_pixel_selection() {
        let collisions = collisions_for(&[("arm_l", 1.0), ("arm_r", 1.0)]);
        let parts = resolved_parts_for(&[
            ("arm_l", 10, "arm_shared", false),
            ("arm_r", 20, "arm_shared", true),
        ]);

        let selected = select_resolved_composed_part(&collisions, &parts, Vec2::ZERO, |part| {
            part.part_id == "arm_l"
        })
        .expect("pixel refinement should preserve semantic ids even when sprite ids match");

        assert_eq!(selected.part_id, "arm_l");
    }
}
