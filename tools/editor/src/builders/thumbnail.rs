use std::{collections::HashMap, fs};

use anyhow::{Context, Result};
use asset_pipeline::aseprite::{AtlasSprite, CompositionAtlas, PartInstance, PartPose};
use bevy::{
    asset::{AssetServer, RenderAssetUsages},
    image::Image,
    math::{IVec2, Rect, URect, UVec2, Vec2},
    prelude::Assets,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    sprite::{Anchor, Sprite},
};
use carcinisation::stage::{
    components::placement::Depth,
    data::{ObjectType, PickupType, StageSpawn},
    destructible::{components::DestructibleType, data::DestructibleSpawn},
    enemy::entity::EnemyType,
};
use image::{Rgba, RgbaImage, imageops};

use crate::{
    constants::assets_root,
    resources::{CachedThumbnail, ThumbnailCache},
};

#[derive(Clone)]
pub struct ResolvedThumbnail {
    pub sprite: Sprite,
    pub anchor: Anchor,
    /// Scale factor to match runtime fallback when the native asset is missing.
    /// `1.0` when the asset is at the exact requested depth.
    pub fallback_scale: f32,
}

#[derive(Clone, Debug)]
struct ComposedPreview {
    pixels: RgbaImage,
    anchor: Anchor,
}

#[derive(Clone, Debug)]
struct FragmentPlacement {
    rect: asset_pipeline::aseprite::Rect,
    top_left: IVec2,
    size: UVec2,
    flip_x: bool,
    flip_y: bool,
    draw_order: u32,
}

#[derive(Clone, Copy, Debug)]
struct PreviewMetrics {
    min: IVec2,
    size: UVec2,
}

pub fn resolve_stage_spawn_thumbnail(
    spawn: &StageSpawn,
    asset_server: &AssetServer,
    image_assets: &mut Assets<Image>,
    cache: &mut ThumbnailCache,
    depth_scale_config: &carcinisation::stage::depth_scale::DepthScaleConfig,
    animation_tag: Option<&str>,
) -> ResolvedThumbnail {
    match spawn {
        StageSpawn::Destructible(DestructibleSpawn {
            destructible_type, ..
        }) => asset_thumbnail(
            asset_server,
            get_destructible_thumbnail(*destructible_type, spawn.get_depth()),
        ),
        StageSpawn::Enemy(enemy_spawn) => resolve_enemy_thumbnail(
            enemy_spawn.enemy_type,
            spawn.get_depth(),
            asset_server,
            image_assets,
            cache,
            depth_scale_config,
            animation_tag,
        ),
        StageSpawn::Object(object_spawn) => asset_thumbnail(
            asset_server,
            get_object_thumbnail(object_spawn.object_type, spawn.get_depth()),
        ),
        StageSpawn::Pickup(pickup_spawn) => asset_thumbnail(
            asset_server,
            get_pickup_thumbnail(pickup_spawn.pickup_type, spawn.get_depth()),
        ),
    }
}

/// Returns a placeholder thumbnail for spawn types without a valid sprite at the given depth.
fn placeholder_thumbnail() -> ResolvedThumbnail {
    ResolvedThumbnail {
        sprite: Sprite::from_color(
            bevy::color::Color::srgba(1.0, 0.0, 1.0, 0.5),
            Vec2::new(16.0, 16.0),
        ),
        anchor: Anchor::BOTTOM_CENTER,
        fallback_scale: 1.0,
    }
}

fn resolve_enemy_thumbnail(
    enemy_type: EnemyType,
    depth: Depth,
    asset_server: &AssetServer,
    image_assets: &mut Assets<Image>,
    cache: &mut ThumbnailCache,
    depth_scale_config: &carcinisation::stage::depth_scale::DepthScaleConfig,
    animation_tag: Option<&str>,
) -> ResolvedThumbnail {
    use crate::placement::SpawnTemplate;

    let tag = animation_tag
        .or_else(|| SpawnTemplate::Enemy(enemy_type).default_animation_tag())
        .unwrap_or("idle");
    let cache_key = (enemy_type, depth, tag.to_string());

    // Return cached result if available.
    if let Some(cached) = cache.composed_enemies.get(&cache_key) {
        return ResolvedThumbnail {
            sprite: Sprite::from_image(cached.image.clone()),
            anchor: cached.anchor,
            fallback_scale: cached.fallback_scale,
        };
    }

    // Try composed atlas at exact depth.
    if let Ok(preview) = compose_enemy_preview(enemy_type, depth, tag) {
        let cached = CachedThumbnail {
            image: image_assets.add(rgba_image_to_bevy_image(&preview.pixels)),
            anchor: preview.anchor,
            fallback_scale: 1.0,
        };
        cache.composed_enemies.insert(cache_key, cached.clone());
        return ResolvedThumbnail {
            sprite: Sprite::from_image(cached.image),
            anchor: cached.anchor,
            fallback_scale: 1.0,
        };
    }

    // Try spritesheet at exact depth.
    if let Some(thumb) = get_enemy_thumbnail(enemy_type, depth) {
        return asset_thumbnail(asset_server, thumb);
    }

    // Fallback: try nearest authored depth with a composed atlas or spritesheet.
    for delta in 1..9_i8 {
        for candidate_i8 in [depth.to_i8() - delta, depth.to_i8() + delta] {
            let Ok(candidate) = Depth::try_from(candidate_i8) else {
                continue;
            };
            if candidate.to_i8() == 0 {
                continue;
            }
            let scale = depth_scale_config
                .fallback_scale(depth, candidate)
                .unwrap_or(1.0);
            if let Ok(preview) = compose_enemy_preview(enemy_type, candidate, tag) {
                let cached = CachedThumbnail {
                    image: image_assets.add(rgba_image_to_bevy_image(&preview.pixels)),
                    anchor: preview.anchor,
                    fallback_scale: scale,
                };
                cache.composed_enemies.insert(cache_key, cached.clone());
                return ResolvedThumbnail {
                    sprite: Sprite::from_image(cached.image),
                    anchor: cached.anchor,
                    fallback_scale: scale,
                };
            }
            if let Some(thumb) = get_enemy_thumbnail(enemy_type, candidate) {
                let mut result = asset_thumbnail(asset_server, thumb);
                result.fallback_scale = scale;
                return result;
            }
        }
    }

    placeholder_thumbnail()
}

fn asset_thumbnail(
    asset_server: &AssetServer,
    (image_path, rect): (String, Option<Rect>),
) -> ResolvedThumbnail {
    let mut sprite = Sprite::from_image(asset_server.load(image_path));
    sprite.rect = rect;
    ResolvedThumbnail {
        sprite,
        anchor: Anchor::BOTTOM_CENTER,
        fallback_scale: 1.0,
    }
}

/// Try to compose a preview for any enemy type at the given depth and animation tag.
fn compose_enemy_preview(
    enemy_type: EnemyType,
    depth: Depth,
    animation_tag: &str,
) -> Result<ComposedPreview> {
    let preview_dir = assets_root().join(format!(
        "sprites/enemies/{}_{}",
        enemy_type.sprite_base_name(),
        depth.to_i8()
    ));
    let atlas_path = preview_dir.join("atlas.json");
    let atlas: CompositionAtlas = serde_json::from_str(
        &fs::read_to_string(&atlas_path)
            .with_context(|| format!("failed to read {}", atlas_path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", atlas_path.display()))?;

    let atlas_image_path = preview_dir.join(&atlas.atlas_image);
    let atlas_image = image::open(&atlas_image_path)
        .with_context(|| format!("failed to open {}", atlas_image_path.display()))?
        .to_rgba8();

    compose_preview_frame(&atlas, &atlas_image, animation_tag, 0)
}

#[allow(clippy::too_many_lines)]
fn compose_preview_frame(
    atlas: &CompositionAtlas,
    atlas_image: &RgbaImage,
    animation_tag: &str,
    frame_index: usize,
) -> Result<ComposedPreview> {
    let animation = atlas
        .animations
        .iter()
        .find(|animation| animation.tag == animation_tag)
        .with_context(|| format!("missing animation tag '{animation_tag}'"))?;
    let frame = animation.frames.get(frame_index).with_context(|| {
        format!("animation '{animation_tag}' is missing frame index {frame_index}")
    })?;

    let parts_by_id: HashMap<&str, &PartInstance> = atlas
        .parts
        .iter()
        .map(|part| (part.id.as_str(), part))
        .collect();
    let sprites_by_id: HashMap<&str, &AtlasSprite> = atlas
        .sprites
        .iter()
        .map(|sprite| (sprite.id.as_str(), sprite))
        .collect();
    let mut poses_by_part: HashMap<&str, Vec<&PartPose>> = HashMap::new();
    for pose in &frame.parts {
        poses_by_part
            .entry(pose.part_id.as_str())
            .or_default()
            .push(pose);
    }
    for poses in poses_by_part.values_mut() {
        poses.sort_unstable_by_key(|pose| pose.fragment);
    }

    let mut visual_parts: Vec<&PartInstance> = atlas
        .parts
        .iter()
        .filter(|part| part.source_layer.is_some() || part.source_region.is_some())
        .collect();
    visual_parts.sort_unstable_by_key(|part| part.draw_order);

    let mut resolved_pivots = HashMap::new();
    let mut placements = Vec::new();
    for part in visual_parts {
        let Some(poses) = poses_by_part.get(part.id.as_str()) else {
            continue;
        };
        if poses.is_empty() || !poses[0].visible {
            continue;
        }
        let absolute_pivot =
            resolve_part_pivot(part, &parts_by_id, &poses_by_part, &mut resolved_pivots)
                .with_context(|| format!("failed to resolve preview pivot for '{}'", part.id))?;

        for (index, pose) in poses.iter().enumerate() {
            let sprite = sprites_by_id
                .get(pose.sprite_id.as_str())
                .with_context(|| format!("missing atlas sprite '{}'", pose.sprite_id))?;
            let size = UVec2::new(sprite.rect.w, sprite.rect.h);
            let fragment_pivot = if index == 0 {
                absolute_pivot
            } else if part.parent_id.is_some() {
                resolve_parent_pivot(part, &parts_by_id, &poses_by_part, &mut resolved_pivots)
                    .with_context(|| {
                        format!("failed to resolve parent preview pivot for '{}'", part.id)
                    })?
                    + ivec2_from_point(pose.local_offset)
            } else {
                ivec2_from_point(pose.local_offset)
            };
            placements.push(FragmentPlacement {
                rect: sprite.rect.clone(),
                top_left: fragment_pivot - ivec2_from_point(part.pivot),
                size,
                flip_x: pose.flip_x,
                flip_y: pose.flip_y,
                draw_order: part.draw_order,
            });
        }
    }

    let metrics = compute_preview_metrics(&placements)
        .context("preview frame contains no visible placements")?;
    let mut composed = RgbaImage::new(metrics.size.x, metrics.size.y);
    placements.sort_unstable_by_key(|placement| placement.draw_order);
    for placement in &placements {
        blit_fragment(&mut composed, atlas_image, placement, metrics.min);
    }

    let anchor = match atlas.spawn_anchor {
        asset_pipeline::composed_ron::SpawnAnchorMode::Origin => {
            // Origin mode: entity position = composition origin (0,0).
            // In the cropped preview image, the origin pixel is at
            // (-metrics.min.x, metrics.min.y) from bottom-left.
            // Bevy Anchor is normalised −0.5..0.5 from centre.
            if metrics.size.x > 0 && metrics.size.y > 0 {
                Anchor(Vec2::new(
                    -metrics.min.x as f32 / metrics.size.x as f32 - 0.5,
                    metrics.min.y as f32 / metrics.size.y as f32 + 0.5,
                ))
            } else {
                Anchor::CENTER
            }
        }
        asset_pipeline::composed_ron::SpawnAnchorMode::BottomOrigin => {
            carcinisation::stage::enemy::composed::bevy_anchor_for_composed(
                (atlas.canvas.w as u16, atlas.canvas.h as u16),
                (atlas.origin.x as i16, atlas.origin.y as i16),
                atlas.spawn_anchor,
            )
        }
    };

    Ok(ComposedPreview {
        pixels: composed,
        anchor,
    })
}

fn resolve_part_pivot<'a>(
    part: &'a PartInstance,
    parts_by_id: &HashMap<&'a str, &'a PartInstance>,
    poses_by_part: &HashMap<&'a str, Vec<&'a PartPose>>,
    resolved_pivots: &mut HashMap<&'a str, IVec2>,
) -> Option<IVec2> {
    if let Some(resolved) = resolved_pivots.get(part.id.as_str()) {
        return Some(*resolved);
    }
    let primary = poses_by_part.get(part.id.as_str())?.first()?;
    let resolved = if part.parent_id.is_some() {
        let parent_pivot = resolve_parent_pivot(part, parts_by_id, poses_by_part, resolved_pivots)?;
        parent_pivot + ivec2_from_point(primary.local_offset)
    } else {
        ivec2_from_point(primary.local_offset)
    };
    resolved_pivots.insert(part.id.as_str(), resolved);
    Some(resolved)
}

fn resolve_parent_pivot<'a>(
    part: &'a PartInstance,
    parts_by_id: &HashMap<&'a str, &'a PartInstance>,
    poses_by_part: &HashMap<&'a str, Vec<&'a PartPose>>,
    resolved_pivots: &mut HashMap<&'a str, IVec2>,
) -> Option<IVec2> {
    let mut parent_id = part.parent_id.as_deref();
    while let Some(current_parent_id) = parent_id {
        let parent = parts_by_id.get(current_parent_id)?;
        if parent.source_layer.is_some() || parent.source_region.is_some() {
            if poses_by_part.contains_key(current_parent_id) {
                return resolve_part_pivot(parent, parts_by_id, poses_by_part, resolved_pivots);
            }
            return None;
        }
        parent_id = parent.parent_id.as_deref();
    }

    Some(IVec2::ZERO)
}

fn compute_preview_metrics(placements: &[FragmentPlacement]) -> Option<PreviewMetrics> {
    let mut iter = placements.iter();
    let first = iter.next()?;
    let mut min = first.top_left;
    let mut max = first.top_left + first.size.as_ivec2();

    for placement in iter {
        min = min.min(placement.top_left);
        max = max.max(placement.top_left + placement.size.as_ivec2());
    }

    let size = max - min;
    Some(PreviewMetrics {
        min,
        size: UVec2::new(size.x.max(0) as u32, size.y.max(0) as u32),
    })
}

fn blit_fragment(
    target: &mut RgbaImage,
    atlas_image: &RgbaImage,
    placement: &FragmentPlacement,
    min: IVec2,
) {
    let source = imageops::crop_imm(
        atlas_image,
        placement.rect.x,
        placement.rect.y,
        placement.rect.w,
        placement.rect.h,
    )
    .to_image();
    let source = match (placement.flip_x, placement.flip_y) {
        (true, true) => imageops::flip_vertical(&imageops::flip_horizontal(&source)),
        (true, false) => imageops::flip_horizontal(&source),
        (false, true) => imageops::flip_vertical(&source),
        (false, false) => source,
    };
    let destination = placement.top_left - min;

    for y in 0..source.height() {
        for x in 0..source.width() {
            let pixel = *source.get_pixel(x, y);
            if pixel[3] == 0 {
                continue;
            }
            let dst_x = destination.x + x as i32;
            let dst_y = destination.y + y as i32;
            let existing = *target.get_pixel(dst_x as u32, dst_y as u32);
            target.put_pixel(dst_x as u32, dst_y as u32, blend_pixel(existing, pixel));
        }
    }
}

fn blend_pixel(dst: Rgba<u8>, src: Rgba<u8>) -> Rgba<u8> {
    if src[3] == u8::MAX || dst[3] == 0 {
        return src;
    }

    let alpha = f32::from(src[3]) / 255.0;
    let inv_alpha = 1.0 - alpha;
    let dst_alpha = f32::from(dst[3]) / 255.0;
    let out_alpha = alpha + dst_alpha * inv_alpha;
    if out_alpha <= f32::EPSILON {
        return Rgba([0, 0, 0, 0]);
    }

    let mut out = [0u8; 4];
    for index in 0..3 {
        let blended = (f32::from(src[index]) * alpha
            + f32::from(dst[index]) * dst_alpha * inv_alpha)
            / out_alpha;
        out[index] = blended.round().clamp(0.0, 255.0) as u8;
    }
    out[3] = (out_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgba(out)
}

fn rgba_image_to_bevy_image(image: &RgbaImage) -> Image {
    Image::new(
        Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        image.clone().into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    )
}

fn ivec2_from_point(point: asset_pipeline::aseprite::Point) -> IVec2 {
    IVec2::new(point.x, point.y)
}

pub fn get_enemy_thumbnail(enemy_type: EnemyType, depth: Depth) -> Option<(String, Option<Rect>)> {
    match enemy_type {
        EnemyType::Mosquito => {
            let loc = "sprites/enemies/mosquito_idle_";
            let ext = ".px_sprite.png";
            match depth {
                Depth::Three => Some((
                    format!("{loc}3{ext}"),
                    URect::new(0, 0, 49, 49).as_rect().into(),
                )),
                Depth::Four => Some((
                    format!("{loc}4{ext}"),
                    URect::new(0, 0, 35, 35).as_rect().into(),
                )),
                Depth::Five => Some((
                    format!("{loc}5{ext}"),
                    URect::new(0, 0, 23, 23).as_rect().into(),
                )),
                Depth::Six => Some((
                    format!("{loc}6{ext}"),
                    URect::new(0, 0, 15, 15).as_rect().into(),
                )),
                Depth::Seven => Some((
                    format!("{loc}7{ext}"),
                    URect::new(0, 0, 9, 9).as_rect().into(),
                )),
                Depth::Eight => Some((
                    format!("{loc}8{ext}"),
                    URect::new(0, 0, 5, 5).as_rect().into(),
                )),
                _ => None,
            }
        }
        EnemyType::Mosquiton => match depth {
            Depth::Three => Some(("sprites/enemies/mosquiton_3/source.png".into(), None)),
            _ => None,
        },
        EnemyType::Tardigrade => {
            let loc = "sprites/enemies/tardigrade_idle_";
            let ext = ".px_sprite.png";
            match depth {
                Depth::Six => Some((
                    format!("{loc}6{ext}"),
                    URect::new(0, 0, 63, 63).as_rect().into(),
                )),
                Depth::Seven => Some((
                    format!("{loc}7{ext}"),
                    URect::new(0, 0, 42, 42).as_rect().into(),
                )),
                Depth::Eight => Some((
                    format!("{loc}8{ext}"),
                    URect::new(0, 0, 23, 23).as_rect().into(),
                )),
                _ => None,
            }
        }
        EnemyType::Spidey | EnemyType::Marauder | EnemyType::Spidomonsta | EnemyType::Kyle => None,
    }
}

pub fn get_destructible_thumbnail(
    destructible_type: DestructibleType,
    _depth: Depth,
) -> (String, Option<Rect>) {
    match destructible_type {
        DestructibleType::Crystal => ("sprites/objects/crystal_base_5.px_sprite.png".into(), None),
        DestructibleType::Lamp => ("sprites/objects/lamp_base_3.px_sprite.png".into(), None),
        DestructibleType::Mushroom => {
            ("sprites/objects/mushroom_base_4.px_sprite.png".into(), None)
        }
        DestructibleType::Trashcan => {
            ("sprites/objects/trashcan_base_6.px_sprite.png".into(), None)
        }
    }
}

pub fn get_object_thumbnail(object_type: ObjectType, _depth: Depth) -> (String, Option<Rect>) {
    match object_type {
        ObjectType::BenchBig => ("sprites/objects/bench_big.px_sprite.png".into(), None),
        ObjectType::BenchSmall => ("sprites/objects/bench_small.px_sprite.png".into(), None),
        ObjectType::Fibertree => ("sprites/objects/fiber_tree.px_sprite.png".into(), None),
        ObjectType::RugparkSign => ("sprites/objects/rugpark_sign.px_sprite.png".into(), None),
    }
}

pub fn get_pickup_thumbnail(pickup_type: PickupType, _depth: Depth) -> (String, Option<Rect>) {
    match pickup_type {
        PickupType::BigHealthpack => ("sprites/pickups/health_6.px_sprite.png".into(), None),
        PickupType::SmallHealthpack => ("sprites/pickups/health_4.px_sprite.png".into(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asset_pipeline::aseprite::{
        Animation, AnimationFrame, CompositionGameplay, PartDefinition, PartGameplayMetadata,
        Point, Rect as AtlasRect, Size,
    };

    fn make_test_atlas(parts: Vec<PartPose>) -> CompositionAtlas {
        CompositionAtlas {
            schema_version: 3,
            entity: "preview_test".into(),
            depth: 3,
            source: "test.aseprite".into(),
            canvas: Size { w: 8, h: 8 },
            origin: Point { x: 0, y: 0 },
            spawn_anchor: asset_pipeline::composed_ron::SpawnAnchorMode::default(),
            ground_anchor_y: None,
            air_anchor_y: None,
            atlas_image: "atlas.png".into(),
            part_definitions: vec![PartDefinition {
                id: "root".into(),
                tags: vec![],
                gameplay: PartGameplayMetadata::default(),
            }],
            parts: vec![PartInstance {
                id: "body".into(),
                definition_id: "root".into(),
                name: "body".into(),
                parent_id: None,
                source_layer: Some("body".into()),
                source_region: None,
                split: None,
                draw_order: 10,
                pivot: Point { x: 0, y: 0 },
                tags: vec![],
                visible_by_default: true,
                gameplay: PartGameplayMetadata::default(),
            }],
            sprites: vec![
                AtlasSprite {
                    id: "left".into(),
                    rect: AtlasRect {
                        x: 0,
                        y: 0,
                        w: 2,
                        h: 1,
                    },
                },
                AtlasSprite {
                    id: "centre".into(),
                    rect: AtlasRect {
                        x: 2,
                        y: 0,
                        w: 1,
                        h: 1,
                    },
                },
            ],
            animations: vec![Animation {
                tag: "idle".into(),
                direction: "forward".into(),
                repeats: None,
                frames: vec![AnimationFrame {
                    source_frame: 0,
                    duration_ms: 100,
                    events: vec![],
                    parts,
                }],
                part_overrides: vec![],
            }],
            gameplay: CompositionGameplay::default(),
        }
    }

    #[test]
    fn preview_composes_split_fragments_and_anchor() {
        let mut atlas_image = RgbaImage::new(3, 1);
        atlas_image.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        atlas_image.put_pixel(1, 0, Rgba([0, 255, 0, 255]));
        atlas_image.put_pixel(2, 0, Rgba([0, 0, 255, 255]));

        let atlas = make_test_atlas(vec![
            PartPose {
                part_id: "body".into(),
                sprite_id: "left".into(),
                local_offset: Point { x: -2, y: 0 },
                flip_x: false,
                flip_y: false,
                visible: true,
                opacity: 255,
                fragment: 0,
            },
            PartPose {
                part_id: "body".into(),
                sprite_id: "centre".into(),
                local_offset: Point { x: 0, y: 0 },
                flip_x: false,
                flip_y: false,
                visible: true,
                opacity: 255,
                fragment: 1,
            },
            PartPose {
                part_id: "body".into(),
                sprite_id: "left".into(),
                local_offset: Point { x: 1, y: 0 },
                flip_x: true,
                flip_y: false,
                visible: true,
                opacity: 255,
                fragment: 2,
            },
        ]);

        let preview = compose_preview_frame(&atlas, &atlas_image, "idle", 0)
            .expect("split frame should compose");

        assert_eq!(preview.pixels.dimensions(), (5, 1));
        // Anchor derived from atlas metadata: canvas=(8,8), origin=(0,0), BottomOrigin
        // → PxAnchor::Custom(0.0, 0.0) → Bevy Anchor(-0.5, -0.5) = BottomLeft
        assert!(
            preview
                .anchor
                .as_vec()
                .abs_diff_eq(Vec2::new(-0.5, -0.5), 1e-6),
            "unexpected preview anchor {:?}",
            preview.anchor,
        );
        let row: Vec<[u8; 4]> = (0..5).map(|x| preview.pixels.get_pixel(x, 0).0).collect();
        assert_eq!(
            row,
            vec![
                [255, 0, 0, 255],
                [0, 255, 0, 255],
                [0, 0, 255, 255],
                [0, 255, 0, 255],
                [255, 0, 0, 255],
            ]
        );
    }

    #[test]
    fn preview_anchor_can_point_outside_visible_bounds() {
        let mut atlas_image = RgbaImage::new(3, 1);
        atlas_image.put_pixel(2, 0, Rgba([255, 255, 255, 255]));

        let atlas = make_test_atlas(vec![PartPose {
            part_id: "body".into(),
            sprite_id: "centre".into(),
            local_offset: Point { x: 5, y: 0 },
            flip_x: false,
            flip_y: false,
            visible: true,
            opacity: 255,
            fragment: 0,
        }]);

        let preview = compose_preview_frame(&atlas, &atlas_image, "idle", 0)
            .expect("off-origin frame should compose");

        assert_eq!(preview.pixels.dimensions(), (1, 1));
        // Anchor derived from atlas metadata, not preview bounds.
        // canvas=(8,8), origin=(0,0), BottomOrigin → Anchor(-0.5, -0.5)
        assert_eq!(preview.anchor, Anchor(Vec2::new(-0.5, -0.5)));
    }

    #[test]
    fn mosquiton_preview_uses_composed_idle_frame_instead_of_raw_sheet() {
        let preview = compose_enemy_preview(EnemyType::Mosquiton, Depth::Three, "idle_fly")
            .expect("mosquiton preview should load");
        let raw_sheet = image::open(assets_root().join("sprites/enemies/mosquiton_3/source.png"))
            .expect("raw sheet should load")
            .to_rgba8();

        assert_ne!(
            preview.pixels.dimensions(),
            raw_sheet.dimensions(),
            "editor preview should not reuse the raw atlas sheet dimensions"
        );
        assert!(
            preview.pixels.pixels().any(|pixel| pixel[3] != 0),
            "preview should contain visible pixels"
        );
    }

    #[test]
    fn origin_mode_anchor_uses_preview_metrics() {
        // A 2×1 sprite placed at offset (3, 0) in an Origin-mode atlas.
        // Preview image is cropped to 2×1, min=(3,0).
        // Origin (0,0) is 3px left of the cropped image → anchor.x = -3/2 - 0.5 = -2.0
        let mut atlas_image = RgbaImage::new(3, 1);
        atlas_image.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        atlas_image.put_pixel(1, 0, Rgba([0, 255, 0, 255]));

        let mut atlas = make_test_atlas(vec![PartPose {
            part_id: "body".into(),
            sprite_id: "left".into(),
            local_offset: Point { x: 3, y: 0 },
            flip_x: false,
            flip_y: false,
            visible: true,
            opacity: 255,
            fragment: 0,
        }]);
        atlas.spawn_anchor = asset_pipeline::composed_ron::SpawnAnchorMode::Origin;
        atlas.canvas = Size { w: 16, h: 16 };
        atlas.origin = Point { x: 5, y: 4 };

        let preview = compose_preview_frame(&atlas, &atlas_image, "idle", 0)
            .expect("origin-mode preview should compose");

        assert_eq!(preview.pixels.dimensions(), (2, 1));
        // min = (3, 0), size = (2, 1)
        // anchor.x = -3/2 - 0.5 = -2.0
        // anchor.y = 0/1 + 0.5 = 0.5
        let v = preview.anchor.as_vec();
        assert!(
            (v.x - (-2.0)).abs() < 1e-6 && (v.y - 0.5).abs() < 1e-6,
            "Origin-mode anchor should use metrics, got {:?}",
            preview.anchor
        );
    }
}
