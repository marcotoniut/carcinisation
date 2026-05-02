use bevy::asset::AssetEvent;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use carapace::prelude::*;
use carcinisation_collision::{
    AtlasMaskFrames, AtlasPixelCollisionCache, AtlasPixelData, Collider, ColliderShape,
    PixelCollisionCache, PixelMaskSource, SpritePixelData, WorldMaskInstance, atlas_data,
    sprite_data, world_mask_contains_point, world_mask_overlap, world_mask_rect_from_spatial,
    world_mask_rect_from_top_left,
};
use std::sync::Arc;

use crate::stage::{
    components::interactive::ColliderData,
    destructible::components::Destructible,
    enemy::{
        components::Enemy,
        composed::{
            ComposedAtlasBindings, ComposedCollisionState, ComposedResolvedParts,
            ResolvedPartCollision, ResolvedPartFragmentState, ResolvedPartState,
        },
    },
};

#[derive(SystemParam)]
pub struct MaskCollisionAssets<'w, 's> {
    sprite_assets: Res<'w, Assets<CxSpriteAsset>>,
    atlas_assets: Res<'w, Assets<CxSpriteAtlasAsset>>,
    sprite_asset_events: MessageReader<'w, 's, AssetEvent<CxSpriteAsset>>,
    atlas_asset_events: MessageReader<'w, 's, AssetEvent<CxSpriteAtlasAsset>>,
    sprite_cache: Local<'s, PixelCollisionCache>,
    atlas_cache: Local<'s, AtlasPixelCollisionCache>,
}

impl MaskCollisionAssets<'_, '_> {
    pub fn refresh(&mut self) {
        if self.sprite_asset_events.read().next().is_some() {
            self.sprite_cache.clear();
        }
        if self.atlas_asset_events.read().next().is_some() {
            self.atlas_cache.clear();
        }
    }

    pub fn sprite_pixels(
        &mut self,
        handle: &Handle<CxSpriteAsset>,
    ) -> Option<Arc<SpritePixelData>> {
        sprite_data(&mut self.sprite_cache, &self.sprite_assets, handle)
    }

    pub fn atlas_pixels(
        &mut self,
        handle: &Handle<CxSpriteAtlasAsset>,
    ) -> Option<Arc<AtlasPixelData>> {
        atlas_data(&mut self.atlas_cache, &self.atlas_assets, handle)
    }

    /// Returns a reference to the atlas assets for use in spawn helpers.
    #[must_use]
    pub fn atlas_asset_store(&self) -> &Assets<CxSpriteAtlasAsset> {
        &self.atlas_assets
    }

    /// Returns the atlas region size for a given sprite, if the atlas is loaded.
    #[must_use]
    pub fn atlas_sprite_region_size(&self, sprite: &CxAtlasSprite) -> Option<UVec2> {
        atlas_region_size(self, sprite)
    }

    /// Returns the atlas region for a given sprite, if the atlas is loaded.
    #[must_use]
    pub fn atlas_sprite_region(
        &self,
        sprite: &CxAtlasSprite,
    ) -> Option<&carapace::prelude::AtlasRegion> {
        self.atlas_assets
            .get(&sprite.atlas)
            .and_then(|atlas| atlas.region(sprite.region))
    }
}

#[derive(Clone, Copy)]
pub struct CollisionTarget<'a> {
    pub position: &'a CxPosition,
    pub world_pos: &'a WorldPos,
    pub anchor: &'a CxAnchor,
    pub canvas: &'a CxRenderSpace,
    pub frame: Option<&'a CxFrameView>,
    pub sprite: Option<&'a CxSprite>,
    pub atlas_sprite: Option<&'a CxAtlasSprite>,
    pub presentation: Option<&'a CxPresentationTransform>,
    pub collider_data: Option<&'a ColliderData>,
    pub composed_collision_state: Option<&'a ComposedCollisionState>,
    pub composed_resolved_parts: Option<&'a ComposedResolvedParts>,
    pub composed_atlas_bindings: Option<&'a ComposedAtlasBindings>,
    pub enemy: Option<&'a Enemy>,
    pub destructible: Option<&'a Destructible>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TargetCollisionHit {
    pub hit_position: Vec2,
    pub defense: f32,
    pub semantic_part: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TargetCollisionResult {
    pub evaluated: bool,
    pub hit: Option<TargetCollisionHit>,
}

#[derive(Clone, Copy)]
pub struct AttackMask<'a> {
    pub mask: WorldMaskInstance<'a>,
}

#[derive(Clone, Debug, PartialEq)]
struct ComposedHitSelection {
    part_id: String,
    defense: f32,
    hit_position: Vec2,
}

#[must_use]
pub fn build_attack_mask(
    attack_data: &SpritePixelData,
    attack_frame: Option<CxFrameView>,
    rect_world: IRect,
) -> AttackMask<'_> {
    AttackMask {
        mask: WorldMaskInstance {
            source: PixelMaskSource::Sprite(attack_data),
            frame: attack_frame,
            world: carcinisation_collision::WorldMaskRect {
                rect: rect_world,
                flip_x: false,
                flip_y: false,
            },
            closed: false,
        },
    }
}

pub fn resolve_target_point_hit(
    target: CollisionTarget<'_>,
    world_points: &[Vec2],
    camera_world: IVec2,
    assets: &mut MaskCollisionAssets<'_, '_>,
) -> TargetCollisionResult {
    resolve_target_hit(
        target,
        TargetCollisionProbe::Point { world_points },
        camera_world,
        assets,
    )
}

pub fn resolve_target_mask_hit(
    target: CollisionTarget<'_>,
    attack: AttackMask<'_>,
    camera_world: IVec2,
    assets: &mut MaskCollisionAssets<'_, '_>,
) -> TargetCollisionResult {
    resolve_target_hit(
        target,
        TargetCollisionProbe::Mask {
            attack: attack.mask,
        },
        camera_world,
        assets,
    )
}

pub fn visit_target_debug_collider<FMask, FPrimitive>(
    target: CollisionTarget<'_>,
    camera_world: IVec2,
    assets: &mut MaskCollisionAssets<'_, '_>,
    mut visit_mask: FMask,
    mut visit_primitive: FPrimitive,
) -> bool
where
    FMask: FnMut(WorldMaskInstance<'_>),
    FPrimitive: FnMut(Vec2, &Collider),
{
    let simple_collision_origin = simple_collision_origin(target);

    if target.composed_collision_state.is_some() {
        let mut drew_mask = false;
        visit_composed_target_masks(target, assets, |mask, _, _| {
            drew_mask = true;
            visit_mask(mask);
        });
        if drew_mask {
            return true;
        }

        if let Some(collision_state) = target.composed_collision_state {
            for collision in collision_state.collisions() {
                visit_primitive(collision.pivot_position, &collision.collider);
            }
            return !collision_state.collisions().is_empty();
        }
    }

    let mut drew_mask = false;
    visit_simple_target_masks(target, camera_world, assets, |mask| {
        drew_mask = true;
        visit_mask(mask);
    });
    if drew_mask {
        return true;
    }

    if let Some(collider_data) = target.collider_data {
        for collider in &collider_data.0 {
            if matches!(
                collider.shape,
                ColliderShape::SpriteMask | ColliderShape::SpriteMaskClosed
            ) {
                continue;
            }
            visit_primitive(simple_collision_origin, collider);
        }
        return collider_data.0.iter().any(|c| {
            !matches!(
                c.shape,
                ColliderShape::SpriteMask | ColliderShape::SpriteMaskClosed
            )
        });
    }

    false
}

enum TargetCollisionProbe<'a> {
    Point { world_points: &'a [Vec2] },
    Mask { attack: WorldMaskInstance<'a> },
}

fn resolve_target_hit(
    target: CollisionTarget<'_>,
    probe: TargetCollisionProbe<'_>,
    camera_world: IVec2,
    assets: &mut MaskCollisionAssets<'_, '_>,
) -> TargetCollisionResult {
    let (simple_hit, simple_visited) =
        resolve_simple_mask_hit(target, &probe, camera_world, assets);
    if simple_visited {
        return TargetCollisionResult {
            evaluated: true,
            hit: simple_hit,
        };
    }

    let (composed_hit, composed_pixel_evaluated) =
        resolve_composed_mask_hit(target, &probe, assets);
    if let Some(hit) = composed_hit {
        return TargetCollisionResult {
            evaluated: true,
            hit: Some(TargetCollisionHit {
                hit_position: hit.hit_position,
                defense: hit.defense,
                semantic_part: Some(hit.part_id),
            }),
        };
    }

    if !composed_pixel_evaluated
        && let Some(composed_collision_state) = target.composed_collision_state
        && let Some(hit_position) = fallback_probe_point(&probe)
        && composed_collision_state
            .point_collides(hit_position)
            .is_some()
    {
        return TargetCollisionResult {
            evaluated: true,
            hit: Some(TargetCollisionHit {
                hit_position,
                defense: 1.0,
                semantic_part: None,
            }),
        };
    }

    if let Some(hit_position) = fallback_probe_point(&probe)
        && let Some(collider_data) = target.collider_data
    {
        let hit = collider_data
            .point_collides(simple_collision_origin(target), hit_position)
            .map(|collider| TargetCollisionHit {
                hit_position,
                defense: collider.defense,
                semantic_part: None,
            });
        if hit.is_some() || target.enemy.is_some() {
            return TargetCollisionResult {
                evaluated: true,
                hit,
            };
        }
    }

    TargetCollisionResult {
        evaluated: false,
        hit: None,
    }
}

/// Returns `(hit, mask_was_visited)`.
///
/// When a simple sprite mask is available and visited, the pixel verdict is
/// final — a miss must NOT fall through to coarse colliders.
fn resolve_simple_mask_hit(
    target: CollisionTarget<'_>,
    probe: &TargetCollisionProbe<'_>,
    camera_world: IVec2,
    assets: &mut MaskCollisionAssets<'_, '_>,
) -> (Option<TargetCollisionHit>, bool) {
    let mut visited = false;
    let mut hit = None;
    visit_simple_target_masks(target, camera_world, assets, |mask| {
        visited = true;
        if hit.is_none() {
            hit = probe_simple_mask(mask, probe, target);
        }
    });
    (hit, visited)
}

fn probe_simple_mask(
    mask: WorldMaskInstance<'_>,
    probe: &TargetCollisionProbe<'_>,
    target: CollisionTarget<'_>,
) -> Option<TargetCollisionHit> {
    let collision_origin = simple_collision_origin(target);
    match probe {
        TargetCollisionProbe::Point { world_points } => world_points.iter().find_map(|point| {
            let world = point.round().as_ivec2();
            world_mask_contains_point(mask, world).then(|| TargetCollisionHit {
                hit_position: world.as_vec2(),
                defense: target
                    .collider_data
                    .and_then(|data| data.point_collides(collision_origin, world.as_vec2()))
                    .map_or(1.0, |collider| collider.defense),
                semantic_part: None,
            })
        }),
        TargetCollisionProbe::Mask { attack } => {
            world_mask_overlap(mask, *attack).map(|point| TargetCollisionHit {
                hit_position: point.as_vec2(),
                defense: target
                    .collider_data
                    .and_then(|data| data.point_collides(collision_origin, point.as_vec2()))
                    .map_or(1.0, |collider| collider.defense),
                semantic_part: None,
            })
        }
    }
}

fn simple_collision_origin(target: CollisionTarget<'_>) -> Vec2 {
    target.world_pos.0
        + target
            .presentation
            .map_or(Vec2::ZERO, |pt| pt.collision_offset)
}

fn resolve_composed_mask_hit(
    target: CollisionTarget<'_>,
    probe: &TargetCollisionProbe<'_>,
    assets: &mut MaskCollisionAssets<'_, '_>,
) -> (Option<ComposedHitSelection>, bool) {
    let mut pixel_evaluated = false;
    let mut result = None;
    visit_composed_target_masks(target, assets, |mask, part, _fragment| {
        pixel_evaluated = true;
        if result.is_some() {
            return;
        }

        let hit_position = match probe {
            TargetCollisionProbe::Point { world_points } => world_points.iter().find_map(|point| {
                let world = point.round().as_ivec2();
                world_mask_contains_point(mask, world).then_some(world.as_vec2())
            }),
            TargetCollisionProbe::Mask { attack } => {
                world_mask_overlap(mask, *attack).map(|point| point.as_vec2())
            }
        };
        let Some(hit_position) = hit_position else {
            return;
        };

        result = Some(ComposedHitSelection {
            part_id: part.part_id.clone(),
            defense: part_collision_defense(
                target.composed_collision_state,
                &part.part_id,
                hit_position,
            ),
            hit_position,
        });
    });

    (result, pixel_evaluated)
}

/// Whether the target's collider data requests closed (scanline-filled) mask testing.
fn target_wants_closed_mask(target: &CollisionTarget<'_>) -> bool {
    target.collider_data.is_some_and(|d| {
        d.0.iter()
            .any(|c| matches!(c.shape, ColliderShape::SpriteMaskClosed))
    })
}

fn visit_simple_target_masks<F>(
    target: CollisionTarget<'_>,
    camera_world: IVec2,
    assets: &mut MaskCollisionAssets<'_, '_>,
    mut visit: F,
) where
    F: FnMut(WorldMaskInstance<'_>),
{
    if target.destructible.is_some() {
        return;
    }

    let closed = target_wants_closed_mask(&target);
    let frame = target.frame.copied();
    if let Some(sprite) = target.sprite
        && let Some(data) = assets.sprite_pixels(sprite)
        && let Some(world) = world_mask_rect_from_spatial(
            data.frame_size(),
            *target.position,
            *target.anchor,
            *target.canvas,
            camera_world,
            target.presentation.copied(),
        )
    {
        visit(WorldMaskInstance {
            source: PixelMaskSource::Sprite(data.as_ref()),
            frame,
            world,
            closed,
        });
        return;
    }

    if let Some(atlas_sprite) = target.atlas_sprite
        && let Some(region_size) = atlas_region_size(assets, atlas_sprite)
        && let Some(atlas_pixels) = assets.atlas_pixels(&atlas_sprite.atlas)
        && let Some(world) = world_mask_rect_from_spatial(
            region_size,
            *target.position,
            *target.anchor,
            *target.canvas,
            camera_world,
            target.presentation.copied(),
        )
        && let Some(region) = assets
            .atlas_assets
            .get(&atlas_sprite.atlas)
            .and_then(|atlas| atlas.region(atlas_sprite.region))
    {
        visit(WorldMaskInstance {
            source: PixelMaskSource::Atlas {
                atlas: atlas_pixels.as_ref(),
                frames: AtlasMaskFrames::Region(region),
            },
            frame,
            world,
            closed,
        });
    }
}

fn visit_composed_target_masks<F>(
    target: CollisionTarget<'_>,
    assets: &mut MaskCollisionAssets<'_, '_>,
    mut visit: F,
) where
    F: FnMut(WorldMaskInstance<'_>, &ResolvedPartState, &ResolvedPartFragmentState),
{
    let (Some(resolved_parts), Some(bindings)) = (
        target.composed_resolved_parts,
        target.composed_atlas_bindings,
    ) else {
        return;
    };

    let atlas_handle = bindings.atlas_handle();
    let Some(atlas_pixels) = assets.atlas_pixels(atlas_handle) else {
        return;
    };

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

        let Some(rect) = bindings.sprite_rect(fragment.sprite_id.as_str()) else {
            continue;
        };
        let Some(world) = world_mask_rect_from_top_left(
            fragment.visual_top_left_position,
            fragment.frame_size,
            fragment.flip_x,
            fragment.flip_y,
        ) else {
            continue;
        };
        visit(
            WorldMaskInstance {
                source: PixelMaskSource::Atlas {
                    atlas: atlas_pixels.as_ref(),
                    frames: AtlasMaskFrames::Single(rect),
                },
                frame: None,
                world,
                closed: false,
            },
            part,
            fragment,
        );
    }
}

fn select_resolved_composed_fragment<F>(
    resolved_parts: &ComposedResolvedParts,
    mut hit_test: F,
) -> Option<ComposedHitSelection>
where
    F: FnMut(&ResolvedPartFragmentState, &ResolvedPartState) -> Option<ComposedHitSelection>,
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
        if let Some(hit) = hit_test(fragment, part) {
            return Some(hit);
        }
    }

    None
}

fn part_collision_defense(
    collision_state: Option<&ComposedCollisionState>,
    part_id: &str,
    hit_position: Vec2,
) -> f32 {
    collision_state
        .and_then(|state| part_collision_at_point(state.collisions(), part_id, hit_position))
        .map_or(1.0, |collision| collision.collider.defense)
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

fn fallback_probe_point(probe: &TargetCollisionProbe<'_>) -> Option<Vec2> {
    match probe {
        TargetCollisionProbe::Point { world_points } => world_points.first().copied(),
        TargetCollisionProbe::Mask { attack } => Some(center_of_world_rect(attack.world.rect)),
    }
}

fn center_of_world_rect(rect: IRect) -> Vec2 {
    Vec2::new(
        (rect.min.x + rect.max.x) as f32 * 0.5,
        (rect.min.y + rect.max.y) as f32 * 0.5,
    )
}

fn atlas_region_size(
    assets: &MaskCollisionAssets<'_, '_>,
    sprite: &CxAtlasSprite,
) -> Option<UVec2> {
    assets
        .atlas_assets
        .get(&sprite.atlas)
        .and_then(|atlas| atlas.region(sprite.region))
        .map(|region| region.frame_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collisions_for(parts: &[(&str, f32)]) -> Vec<ResolvedPartCollision> {
        parts
            .iter()
            .map(|(part_id, defense)| ResolvedPartCollision {
                part_id: (*part_id).to_string(),
                collider: carcinisation_collision::Collider::new(
                    carcinisation_collision::ColliderShape::Circle(4.0),
                )
                .with_defense(*defense),
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
    fn composed_fragment_selection_uses_exact_render_order() {
        let parts =
            resolved_parts_for(&[("body", 20, "body", false), ("shield", 10, "shield", false)]);
        let fragments =
            resolved_fragments_for(&[("body", "body", 0, 0), ("shield", "shield", 0, 1)]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        let selected = select_resolved_composed_fragment(&resolved, |_, part| {
            Some(ComposedHitSelection {
                part_id: part.part_id.clone(),
                defense: 1.0,
                hit_position: Vec2::ZERO,
            })
        })
        .expect("front-most emitted fragment should win");

        assert_eq!(selected.part_id, "shield");
    }

    #[test]
    fn composed_transparent_front_fragment_allows_back_fragment() {
        let parts =
            resolved_parts_for(&[("body", 10, "body", false), ("shield", 20, "shield", false)]);
        let fragments =
            resolved_fragments_for(&[("body", "body", 0, 0), ("shield", "shield", 0, 1)]);
        let resolved =
            ComposedResolvedParts::with_parts_fragments_and_offset(parts, fragments, Vec2::ZERO);

        let selected = select_resolved_composed_fragment(&resolved, |_, part| {
            (part.part_id == "body").then_some(ComposedHitSelection {
                part_id: part.part_id.clone(),
                defense: 1.0,
                hit_position: Vec2::ZERO,
            })
        })
        .expect("back visible fragment should win when front pixel is transparent");

        assert_eq!(selected.part_id, "body");
    }

    #[test]
    fn part_collision_at_point_uses_matching_collision() {
        let collisions = collisions_for(&[("body", 0.5), ("shield", 1.0)]);
        let collision = part_collision_at_point(&collisions, "body", Vec2::ZERO).unwrap();

        assert!((collision.collider.defense - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn fallback_mask_probe_uses_attack_rect_center() {
        let point = center_of_world_rect(IRect {
            min: IVec2::new(2, 4),
            max: IVec2::new(6, 8),
        });

        assert_eq!(point, Vec2::new(4.0, 6.0));
    }

    #[test]
    fn simple_collision_origin_includes_collision_offset() {
        let position = CxPosition::default();
        let world_pos = WorldPos(Vec2::new(12.0, 34.0));
        let anchor = CxAnchor::Center;
        let canvas = CxRenderSpace::World;
        let presentation = CxPresentationTransform {
            collision_offset: Vec2::new(-6.0, 3.0),
            ..Default::default()
        };

        let origin = simple_collision_origin(CollisionTarget {
            position: &position,
            world_pos: &world_pos,
            anchor: &anchor,
            canvas: &canvas,
            frame: None,
            sprite: None,
            atlas_sprite: None,
            presentation: Some(&presentation),
            collider_data: None,
            composed_collision_state: None,
            composed_resolved_parts: None,
            composed_atlas_bindings: None,
            enemy: None,
            destructible: None,
        });

        assert_eq!(origin, Vec2::new(6.0, 37.0));
    }
}
