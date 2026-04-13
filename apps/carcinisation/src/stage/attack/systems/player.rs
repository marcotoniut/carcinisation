use bevy::asset::AssetEvent;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use carapace::prelude::*;
use carcinisation_collision::pixel_mask::{
    AtlasPixelCollisionCache, PixelCollisionCache, SpritePixelData, atlas_data,
    atlas_region_contains_point, atlas_region_overlaps_sprite_mask, mask_contains_point,
    pixel_overlap, sprite_data, sprite_rect,
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
                ResolvedPartCollision, ResolvedPartFragmentState, ResolvedPartState,
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
        let attack_rect_world = attack_rect.map(|rect| IRect {
            min: rect.min + camera.0,
            max: rect.max + camera.0,
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
            let mut composed_hit_selection = None;
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

            // Pixel-authoritative composed hit resolution. When pixel data
            // is available and evaluated, its verdict is final — no coarse
            // fallback can override a pixel miss.
            let mut composed_pixel_evaluated = false;
            if hit.is_none()
                && composed_resolved_parts.is_some()
                && composed_atlas_bindings.is_some()
            {
                match attack_definition.collision {
                    AttackCollisionMode::Point => {
                        composed_pixel_evaluated = true;
                        evaluated = true;
                        if let Some(selection) = resolve_composed_hit_at_point(
                            attack_world,
                            composed_collision_state,
                            composed_resolved_parts,
                            composed_atlas_bindings,
                            &pixel_assets.atlas_assets,
                            &mut pixel_assets.atlas_cache,
                        ) {
                            hit = Some(selection.hit_position);
                            composed_hit_selection = Some(selection);
                        }
                    }
                    AttackCollisionMode::SpriteMask => {
                        if let (Some(attack_data), Some(attack_rect_world)) =
                            (attack_data.as_deref(), attack_rect_world)
                        {
                            composed_pixel_evaluated = true;
                            evaluated = true;
                            if let Some(selection) = resolve_composed_hit_with_sprite_mask(
                                SpriteMaskAttackHit {
                                    data: attack_data,
                                    frame: attack_frame.copied(),
                                    rect_world: attack_rect_world,
                                },
                                composed_collision_state,
                                composed_resolved_parts,
                                composed_atlas_bindings,
                                &pixel_assets.atlas_assets,
                                &mut pixel_assets.atlas_cache,
                            ) {
                                hit = Some(selection.hit_position);
                                composed_hit_selection = Some(selection);
                            }
                        }
                    }
                    AttackCollisionMode::None => {}
                }
            }

            // Coarse-only fallback: used ONLY when pixel data was not
            // available (assets not loaded, no resolved parts, etc.).
            // When pixel evaluation ran and missed, that miss is final.
            if hit.is_none()
                && !composed_pixel_evaluated
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

            let composed_hit = composed_hit_selection.or_else(|| {
                resolve_composed_hit_at_point(
                    hit_position,
                    composed_collision_state,
                    composed_resolved_parts,
                    composed_atlas_bindings,
                    &pixel_assets.atlas_assets,
                    &mut pixel_assets.atlas_cache,
                )
            });
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
    hit_position: Vec2,
}

#[derive(Clone, Copy)]
struct SpriteMaskAttackHit<'a> {
    data: &'a SpritePixelData,
    frame: Option<PxFrameView>,
    rect_world: IRect,
}

/// Selects a semantic part from front-to-back fragment pixel hits.
/// Pixel-authoritative point hit resolution for composed enemies.
///
/// Returns a hit only when an opaque fragment pixel is found at `hit_position`.
/// No coarse fallback — transparent/outside pixels are a miss.
fn resolve_composed_hit_at_point(
    hit_position: Vec2,
    collision_state: Option<&ComposedCollisionState>,
    resolved_parts: Option<&ComposedResolvedParts>,
    atlas_bindings: Option<&ComposedAtlasBindings>,
    atlas_assets: &Assets<PxSpriteAtlasAsset>,
    atlas_cache: &mut AtlasPixelCollisionCache,
) -> Option<ComposedHitSelection> {
    let (resolved_parts, atlas_bindings) = (resolved_parts?, atlas_bindings?);
    let atlas_pixels = atlas_data(atlas_cache, atlas_assets, atlas_bindings.atlas_handle())?;
    // Convert hit point to screen convention (negate Y) to match
    // fragment_sprite_rect's screen-space rects.
    let screen_point = {
        let p = hit_position.round().as_ivec2();
        IVec2::new(p.x, -p.y)
    };
    let (selected_part, _fragment, pixel_hit_position) =
        select_resolved_composed_fragment(resolved_parts, |fragment| {
            let region_rect = atlas_bindings.sprite_rect(fragment.sprite_id.as_str())?;
            let sprite_rect = fragment_sprite_rect(fragment);
            atlas_region_contains_point(
                atlas_pixels.as_ref(),
                region_rect,
                sprite_rect,
                screen_point,
                fragment.flip_x,
                fragment.flip_y,
            )
            .then_some(hit_position)
        })?;

    Some(ComposedHitSelection {
        part_id: selected_part.part_id.clone(),
        defense: part_defense_at_point(
            collision_state,
            selected_part.part_id.as_str(),
            pixel_hit_position,
        ),
        hit_position: pixel_hit_position,
    })
}

fn resolve_composed_hit_with_sprite_mask(
    attack: SpriteMaskAttackHit<'_>,
    collision_state: Option<&ComposedCollisionState>,
    resolved_parts: Option<&ComposedResolvedParts>,
    atlas_bindings: Option<&ComposedAtlasBindings>,
    atlas_assets: &Assets<PxSpriteAtlasAsset>,
    atlas_cache: &mut AtlasPixelCollisionCache,
) -> Option<ComposedHitSelection> {
    // Convert the attack rect from world Y-up to screen convention (negate Y)
    // to match fragment_sprite_rect.
    let screen_attack_rect = IRect {
        min: IVec2::new(attack.rect_world.min.x, -attack.rect_world.max.y),
        max: IVec2::new(attack.rect_world.max.x, -attack.rect_world.min.y),
    };
    if let (Some(resolved_parts), Some(atlas_bindings)) = (resolved_parts, atlas_bindings)
        && let Some(atlas_pixels) =
            atlas_data(atlas_cache, atlas_assets, atlas_bindings.atlas_handle())
        && let Some((selected_part, _fragment, pixel_hit_position)) =
            select_resolved_composed_fragment(resolved_parts, |fragment| {
                let region_rect = atlas_bindings.sprite_rect(fragment.sprite_id.as_str())?;
                atlas_region_overlaps_sprite_mask(
                    atlas_pixels.as_ref(),
                    region_rect,
                    fragment_sprite_rect(fragment),
                    (fragment.flip_x, fragment.flip_y),
                    attack.data,
                    attack.frame,
                    screen_attack_rect,
                )
                .map(|point| {
                    // Convert overlap point back to world Y-up.
                    IVec2::new(point.x, -point.y).as_vec2()
                })
            })
    {
        return Some(ComposedHitSelection {
            part_id: selected_part.part_id.clone(),
            defense: part_defense_at_point(
                collision_state,
                selected_part.part_id.as_str(),
                pixel_hit_position,
            ),
            hit_position: pixel_hit_position,
        });
    }

    None
}

fn select_resolved_composed_fragment<F>(
    resolved_parts: &ComposedResolvedParts,
    mut pixel_hit: F,
) -> Option<(&ResolvedPartState, &ResolvedPartFragmentState, Vec2)>
where
    F: FnMut(&ResolvedPartFragmentState) -> Option<Vec2>,
{
    for fragment in resolved_parts.fragments().iter().rev() {
        let Some(part) = resolved_parts
            .parts()
            .iter()
            .find(|part| part.part_id == fragment.part_id)
        else {
            continue;
        };
        if !part.targetable {
            continue;
        }
        if let Some(hit_position) = pixel_hit(fragment) {
            return Some((part, fragment, hit_position));
        }
    }

    None
}

/// Builds a screen-convention rect for pixel collision testing.
///
/// The atlas pixel functions (`atlas_region_contains_point`,
/// `atlas_region_overlaps_sprite_mask`) expect screen-convention rects
/// where Y increases downward and `min` is the top-left. This matches
/// the atlas pixel row order (row 0 = authored top of sprite).
///
/// `visual_top_left_position` is in world Y-up space (top = highest Y).
/// We negate Y to convert to screen space, giving `min.y` = screen top.
fn fragment_sprite_rect(fragment: &ResolvedPartFragmentState) -> IRect {
    let top_left = fragment.visual_top_left_position.round().as_ivec2();
    let size = fragment.frame_size.as_ivec2();
    // Negate Y: world top-left (x, +y) → screen top-left (x, -y).
    let screen_top_left = IVec2::new(top_left.x, -top_left.y);
    IRect {
        min: screen_top_left,
        max: screen_top_left + size,
    }
}

fn part_defense_at_point(
    collision_state: Option<&ComposedCollisionState>,
    part_id: &str,
    hit_position: Vec2,
) -> f32 {
    let Some(collision_state) = collision_state else {
        return 1.0;
    };
    part_collision_at_point(collision_state.collisions(), part_id, hit_position)
        .or_else(|| {
            collision_state
                .collisions()
                .iter()
                .rev()
                .find(|collision| collision.part_id == part_id)
        })
        .map_or(1.0, |collision| collision.collider.defense)
}

/// Test-only helper exercising the coarse-with-pixel-override selection model.
///
/// This does NOT match the production pixel-authoritative path (which uses
/// `select_resolved_composed_fragment` on fragments). It models the legacy
/// "no pixel data" fallback where coarse colliders decide the hit and pixel
/// refinement can override but not veto. Retained to test that specific path.
#[cfg(test)]
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
                    part_pivot: IVec2::ZERO,
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

    fn resolved_fragments_for(parts: &[(&str, &str, u32, u32)]) -> Vec<ResolvedPartFragmentState> {
        parts
            .iter()
            .map(
                |(part_id, sprite_id, fragment, render_order)| ResolvedPartFragmentState {
                    part_id: (*part_id).to_string(),
                    sprite_id: (*sprite_id).to_string(),
                    draw_order: 0,
                    fragment: *fragment,
                    render_order: *render_order,
                    frame_size: UVec2::new(4, 4),
                    flip_x: false,
                    flip_y: false,
                    world_top_left_position: Vec2::new(-2.0, -2.0),
                    visual_top_left_position: Vec2::new(-2.0, -2.0),
                },
            )
            .collect()
    }

    #[test]
    fn fragment_selection_uses_exact_render_order() {
        let parts =
            resolved_parts_for(&[("body", 20, "body", false), ("shield", 10, "shield", false)]);
        let fragments =
            resolved_fragments_for(&[("body", "body", 0, 0), ("shield", "shield", 0, 1)]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        let (selected, fragment, _) =
            select_resolved_composed_fragment(&resolved, |_| Some(Vec2::ZERO))
                .expect("front-most emitted fragment should win");

        assert_eq!(selected.part_id, "shield");
        assert_eq!(fragment.render_order, 1);
    }

    #[test]
    fn transparent_front_fragment_allows_back_fragment() {
        let parts =
            resolved_parts_for(&[("body", 10, "body", false), ("shield", 20, "shield", false)]);
        let fragments =
            resolved_fragments_for(&[("body", "body", 0, 0), ("shield", "shield", 0, 1)]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        let (selected, _, _) = select_resolved_composed_fragment(&resolved, |fragment| {
            (fragment.part_id == "body").then_some(Vec2::ZERO)
        })
        .expect("back visible fragment should win when front pixel is transparent");

        assert_eq!(selected.part_id, "body");
    }

    #[test]
    fn split_logical_part_resolves_via_secondary_fragment() {
        let parts = resolved_parts_for(&[("wing", 10, "wing_l", false)]);
        let fragments =
            resolved_fragments_for(&[("wing", "wing_l", 0, 0), ("wing", "wing_r", 1, 1)]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        let (selected, fragment, _) = select_resolved_composed_fragment(&resolved, |fragment| {
            (fragment.fragment == 1).then_some(Vec2::new(5.0, 0.0))
        })
        .expect("secondary fragment should resolve to owning semantic part");

        assert_eq!(selected.part_id, "wing");
        assert_eq!(fragment.sprite_id, "wing_r");
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

    // --- Phase 3 regression tests: pixel-authoritative composed hit ---

    #[test]
    fn opaque_fragment_pixel_hit_succeeds() {
        let parts = resolved_parts_for(&[("body", 10, "body_sprite", false)]);
        let fragments = resolved_fragments_for(&[("body", "body_sprite", 0, 0)]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        let result = select_resolved_composed_fragment(&resolved, |_| Some(Vec2::ZERO));

        assert!(result.is_some(), "opaque pixel hit should succeed");
        assert_eq!(result.unwrap().0.part_id, "body");
    }

    #[test]
    fn transparent_pixel_miss_returns_none() {
        let parts = resolved_parts_for(&[("body", 10, "body_sprite", false)]);
        let fragments = resolved_fragments_for(&[("body", "body_sprite", 0, 0)]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        let result = select_resolved_composed_fragment(&resolved, |_| None);

        assert!(result.is_none(), "transparent pixel should be a miss");
    }

    #[test]
    fn point_inside_coarse_but_outside_opaque_pixels_is_miss() {
        // Coarse collider covers the point, but all fragment pixels are transparent.
        // With pixel-authoritative resolution, this must be a miss.
        let parts = resolved_parts_for(&[("body", 10, "body_sprite", false)]);
        let fragments = resolved_fragments_for(&[("body", "body_sprite", 0, 0)]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        // Pixel check returns None (transparent), simulating coarse hit + pixel miss.
        let result = select_resolved_composed_fragment(&resolved, |_| None);

        assert!(
            result.is_none(),
            "coarse collider must not override pixel miss when fragment data exists"
        );
    }

    #[test]
    fn front_fragment_wins_over_back_fragment_pixel_hit() {
        let parts = resolved_parts_for(&[
            ("back", 10, "back_sprite", false),
            ("front", 20, "front_sprite", false),
        ]);
        let fragments = resolved_fragments_for(&[
            ("back", "back_sprite", 0, 0),
            ("front", "front_sprite", 0, 1),
        ]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        // Both fragments report opaque pixel hit.
        let result = select_resolved_composed_fragment(&resolved, |_| Some(Vec2::ZERO));

        assert!(result.is_some());
        assert_eq!(
            result.unwrap().0.part_id,
            "front",
            "front-most fragment should win when both are opaque"
        );
    }

    #[test]
    fn non_targetable_front_fragment_skipped_for_back_hit() {
        let mut parts = resolved_parts_for(&[
            ("body", 10, "body_sprite", false),
            ("overlay", 20, "overlay_sprite", false),
        ]);
        parts[1].targetable = false; // overlay is non-targetable
        let fragments = resolved_fragments_for(&[
            ("body", "body_sprite", 0, 0),
            ("overlay", "overlay_sprite", 0, 1),
        ]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        let result = select_resolved_composed_fragment(&resolved, |_| Some(Vec2::ZERO));

        assert!(result.is_some());
        assert_eq!(
            result.unwrap().0.part_id,
            "body",
            "non-targetable front fragment should be skipped"
        );
    }

    // --- Phase 4 regression tests: SpriteMask pixel-authoritative ---
    //
    // These exercise the same `select_resolved_composed_fragment` production
    // path that `resolve_composed_hit_with_sprite_mask` uses. The closure
    // simulates mask overlap results (Some = opaque overlap, None = miss).

    #[test]
    fn mask_overlap_opaque_fragment_hit_succeeds() {
        let parts = resolved_parts_for(&[("body", 10, "body_sprite", false)]);
        let fragments = resolved_fragments_for(&[("body", "body_sprite", 0, 0)]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        // Simulate: attack mask overlaps opaque fragment pixels.
        let result = select_resolved_composed_fragment(&resolved, |_| Some(Vec2::new(1.0, 1.0)));

        assert!(result.is_some(), "opaque mask overlap should be a hit");
        assert_eq!(result.unwrap().0.part_id, "body");
    }

    #[test]
    fn mask_overlap_transparent_only_is_miss() {
        let parts = resolved_parts_for(&[("body", 10, "body_sprite", false)]);
        let fragments = resolved_fragments_for(&[("body", "body_sprite", 0, 0)]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        // Simulate: attack mask overlaps fragment but only transparent pixels.
        let result = select_resolved_composed_fragment(&resolved, |_| None);

        assert!(
            result.is_none(),
            "transparent-only mask overlap should be a miss"
        );
    }

    #[test]
    fn mask_split_part_selects_via_overlapping_fragment() {
        let parts = resolved_parts_for(&[("wing", 10, "wing_l", false)]);
        let fragments =
            resolved_fragments_for(&[("wing", "wing_l", 0, 0), ("wing", "wing_r", 1, 1)]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        // Only the secondary fragment (wing_r) has opaque overlap.
        let result = select_resolved_composed_fragment(&resolved, |fragment| {
            (fragment.sprite_id == "wing_r").then_some(Vec2::new(3.0, 0.0))
        });

        assert!(result.is_some());
        let (part, frag, _) = result.unwrap();
        assert_eq!(part.part_id, "wing");
        assert_eq!(frag.sprite_id, "wing_r");
    }

    #[test]
    fn mask_frontmost_overlapping_fragment_wins() {
        let parts = resolved_parts_for(&[
            ("back", 10, "back_sprite", false),
            ("front", 20, "front_sprite", false),
        ]);
        let fragments = resolved_fragments_for(&[
            ("back", "back_sprite", 0, 0),
            ("front", "front_sprite", 0, 1),
        ]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        // Both fragments have opaque mask overlap.
        let result = select_resolved_composed_fragment(&resolved, |_| Some(Vec2::new(1.0, 1.0)));

        assert!(result.is_some());
        assert_eq!(
            result.unwrap().0.part_id,
            "front",
            "frontmost overlapping fragment should win"
        );
    }

    #[test]
    fn mask_coarse_collider_cannot_resurrect_pixel_miss() {
        // This test validates the production invariant: when pixel/mask
        // evaluation runs and finds no opaque overlap, the result is None.
        // The coarse collider (not tested here directly) is gated by
        // composed_pixel_evaluated in check_got_hit.
        let parts = resolved_parts_for(&[("body", 10, "body_sprite", false)]);
        let fragments = resolved_fragments_for(&[("body", "body_sprite", 0, 0)]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        // All fragments transparent → miss.
        let result = select_resolved_composed_fragment(&resolved, |_| None);

        assert!(
            result.is_none(),
            "coarse collider must not resurrect a mask miss when pixel data exists"
        );
    }

    // --- Phase 6 tests: front-part blocking / armour semantics ---

    #[test]
    fn front_armour_part_selected_over_body_behind() {
        let mut parts = resolved_parts_for(&[
            ("body", 10, "body_sprite", false),
            ("armour", 20, "armour_sprite", false),
        ]);
        parts[1].armour = 5;
        let fragments = resolved_fragments_for(&[
            ("body", "body_sprite", 0, 0),
            ("armour", "armour_sprite", 0, 1),
        ]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        // Both fragments opaque at the hit point.
        let result = select_resolved_composed_fragment(&resolved, |_| Some(Vec2::ZERO));

        let (selected, _, _) = result.expect("should hit the front armour");
        assert_eq!(selected.part_id, "armour");
        assert_ne!(
            selected.part_id, "body",
            "body behind opaque armour must not be selected for non-piercing hit"
        );
    }

    #[test]
    fn transparent_front_armour_allows_body_behind() {
        let mut parts = resolved_parts_for(&[
            ("body", 10, "body_sprite", false),
            ("armour", 20, "armour_sprite", false),
        ]);
        parts[1].armour = 5;
        let fragments = resolved_fragments_for(&[
            ("body", "body_sprite", 0, 0),
            ("armour", "armour_sprite", 0, 1),
        ]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        // Armour fragment transparent at hit point, body opaque.
        let result = select_resolved_composed_fragment(&resolved, |fragment| {
            (fragment.part_id == "body").then_some(Vec2::ZERO)
        });

        let (selected, _, _) = result.expect("body should be reachable through transparent armour");
        assert_eq!(selected.part_id, "body");
    }
}
