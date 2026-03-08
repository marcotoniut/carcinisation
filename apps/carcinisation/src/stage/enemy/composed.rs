use crate::stage::{
    components::placement::Depth, enemy::entity::EnemyType, resources::StageTimeDomain,
};
use asset_pipeline::aseprite::{CompositionAtlas, PartDefinition};
use bevy::{
    asset::{Asset, LoadState},
    prelude::*,
    reflect::TypePath,
};
use seldom_pixel::prelude::{
    AtlasRegionId, PxAnchor, PxCanvas, PxCompositePart, PxCompositeSprite, PxPosition,
    PxSpriteAtlasAsset, PxSubPosition,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

/// Current runtime schema version accepted by the composed-enemy renderer.
pub const SUPPORTED_COMPOSITION_SCHEMA_VERSION: u32 = 1;
const COMPOSED_ENEMY_ASSET_ROOT: &str = "sprites/enemies";
const COMPOSED_ENEMY_ATLAS_BASENAME: &str = "atlas";

#[derive(Asset, Clone, Debug, Deserialize, TypePath)]
pub struct CompositionAtlasAsset {
    #[serde(flatten)]
    pub atlas: CompositionAtlas,
    #[serde(skip)]
    runtime: CompositionAtlasRuntime,
}

#[derive(Clone, Debug, Default)]
enum CompositionAtlasRuntime {
    #[default]
    Unprepared,
    Ready(CompositionAtlasCache),
    Invalid(String),
}

#[derive(Clone, Debug)]
struct CompositionAtlasCache {
    parts: Vec<CachedPart>,
    animations: HashMap<String, CachedAnimation>,
}

#[derive(Clone, Debug)]
struct CachedPart {
    id: String,
    draw_order: u32,
}

#[derive(Clone, Debug)]
struct CachedAnimation {
    direction: CachedAnimationDirection,
    repeats: Option<u32>,
    frames: Vec<CachedAnimationFrame>,
}

#[derive(Clone, Debug)]
struct CachedAnimationFrame {
    duration_ms: u32,
    placements: HashMap<String, CachedPlacement>,
}

#[derive(Clone, Debug)]
struct CachedPlacement {
    sprite_id: String,
    bottom_left_offset: IVec2,
    size: UVec2,
    flip_x: bool,
    flip_y: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CachedAnimationDirection {
    Forward,
    Reverse,
    PingPong,
    PingPongReverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CachedCompositeMetrics {
    origin: IVec2,
    size: UVec2,
}

/// Generic animation-state selection surface for composed enemies.
///
/// Species-specific enemy logic should update only this component; the composed
/// renderer consumes it without knowing which enemy type authored the request.
#[derive(Component, Clone, Debug)]
pub struct ComposedAnimationState {
    pub requested_tag: String,
}

impl ComposedAnimationState {
    #[must_use]
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            requested_tag: tag.into(),
        }
    }
}

#[derive(Component, Clone, Debug, Default)]
pub struct ComposedAtlasBindings {
    atlas: Handle<PxSpriteAtlasAsset>,
    sprite_regions: HashMap<String, AtlasRegionId>,
}

#[derive(Component, Clone, Debug)]
pub struct ComposedEnemyVisual {
    pub atlas_manifest: Handle<CompositionAtlasAsset>,
    pub sprite_atlas: Handle<PxSpriteAtlasAsset>,
    active_tag: String,
    frame_index: usize,
    frame_started_at_ms: u64,
    ping_pong_forward: bool,
    last_error: Option<String>,
}

impl ComposedEnemyVisual {
    #[must_use]
    pub fn for_enemy(asset_server: &AssetServer, enemy_type: EnemyType, depth: Depth) -> Self {
        let base_path = composed_enemy_asset_base_path(enemy_type, depth);

        Self {
            atlas_manifest: asset_server.load(composed_enemy_manifest_path(&base_path)),
            sprite_atlas: asset_server.load(composed_enemy_sprite_atlas_path(&base_path)),
            active_tag: String::new(),
            frame_index: 0,
            frame_started_at_ms: 0,
            ping_pong_forward: true,
            last_error: None,
        }
    }
}

#[derive(Component, Debug)]
pub struct ComposedEnemyVisualReady;

#[derive(Component, Debug)]
pub struct ComposedEnemyVisualFailed;

/// Returns the canonical asset base path used by the runtime for composed enemy atlases.
#[must_use]
pub fn composed_enemy_asset_base_path(enemy_type: EnemyType, depth: Depth) -> String {
    format!(
        "{}/{}_{}/{}",
        COMPOSED_ENEMY_ASSET_ROOT,
        enemy_type.sprite_base_name(),
        depth.to_i8(),
        COMPOSED_ENEMY_ATLAS_BASENAME
    )
}

fn composed_enemy_manifest_path(base_path: &str) -> String {
    format!("{base_path}.json")
}

fn composed_enemy_sprite_atlas_path(base_path: &str) -> String {
    format!("{base_path}.px_atlas.ron")
}

pub fn prepare_composed_atlas_assets(mut atlas_assets: ResMut<Assets<CompositionAtlasAsset>>) {
    for (_, atlas_asset) in atlas_assets.iter_mut() {
        atlas_asset.prepare_runtime();
    }
}

pub fn ensure_composed_enemy_parts(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    atlas_assets: Res<Assets<CompositionAtlasAsset>>,
    sprite_atlases: Res<Assets<PxSpriteAtlasAsset>>,
    query: Query<
        (Entity, &ComposedEnemyVisual, &Depth),
        (
            Without<ComposedEnemyVisualReady>,
            Without<ComposedEnemyVisualFailed>,
        ),
    >,
) {
    for (entity, visual, depth) in &query {
        let Some(atlas_asset) = atlas_assets.get(&visual.atlas_manifest) else {
            if let LoadState::Failed(error) = asset_server.load_state(visual.atlas_manifest.id()) {
                error!(
                    "Failed to load composed atlas manifest for {:?}: {error}",
                    entity
                );
                commands.entity(entity).insert(ComposedEnemyVisualFailed);
            }
            continue;
        };
        let Some(sprite_atlas) = sprite_atlases.get(&visual.sprite_atlas) else {
            if let LoadState::Failed(error) = asset_server.load_state(visual.sprite_atlas.id()) {
                error!(
                    "Failed to load composed sprite atlas for {:?}: {error}",
                    entity
                );
                commands.entity(entity).insert(ComposedEnemyVisualFailed);
            }
            continue;
        };

        match atlas_asset.runtime() {
            Ok(_) => match build_atlas_bindings(
                &atlas_asset.atlas,
                sprite_atlas,
                visual.sprite_atlas.clone(),
            ) {
                Ok(bindings) => {
                    commands.entity(entity).insert((
                        bindings,
                        PxCompositeSprite::default(),
                        PxPosition::default(),
                        PxAnchor::Center,
                        depth.to_layer(),
                        PxCanvas::World,
                        Visibility::Hidden,
                        ComposedEnemyVisualReady,
                    ));
                }
                Err(reason) => {
                    error!(
                        "Rejected composed atlas bindings for '{} {}': {}",
                        atlas_asset.atlas.entity, atlas_asset.atlas.depth, reason,
                    );
                    commands.entity(entity).insert(ComposedEnemyVisualFailed);
                }
            },
            Err(_) if atlas_asset.is_invalid() => {
                commands.entity(entity).insert(ComposedEnemyVisualFailed);
            }
            Err("unprepared") => continue,
            Err(reason) => {
                error!(
                    "Unexpected composed atlas runtime state for '{} {}': {}",
                    atlas_asset.atlas.entity, atlas_asset.atlas.depth, reason,
                );
                commands.entity(entity).insert(ComposedEnemyVisualFailed);
            }
        }
    }
}

pub fn update_composed_enemy_visuals(
    mut commands: Commands,
    atlas_assets: Res<Assets<CompositionAtlasAsset>>,
    stage_time: Res<Time<StageTimeDomain>>,
    mut root_query: Query<
        (
            Entity,
            &mut ComposedEnemyVisual,
            &ComposedAnimationState,
            &PxSubPosition,
            &ComposedAtlasBindings,
            &mut PxCompositeSprite,
            &mut PxPosition,
            &mut PxAnchor,
            &mut Visibility,
        ),
        With<ComposedEnemyVisualReady>,
    >,
) {
    let now_ms = stage_time.elapsed().as_millis() as u64;

    for (
        entity,
        mut visual,
        animation_state,
        position,
        atlas_bindings,
        mut composite,
        mut px_position,
        mut anchor,
        mut visibility,
    ) in &mut root_query
    {
        let Some(atlas_asset) = atlas_assets.get(&visual.atlas_manifest) else {
            fail_ready_composed_enemy(
                &mut commands,
                entity,
                &mut composite,
                &mut visibility,
                "composed atlas manifest became unavailable after the visual was marked ready",
            );
            continue;
        };
        let Ok(cache) = atlas_asset.runtime() else {
            fail_ready_composed_enemy(
                &mut commands,
                entity,
                &mut composite,
                &mut visibility,
                "composed atlas cache became unavailable after the visual was marked ready",
            );
            continue;
        };

        let Some(animation) = cache.animations.get(animation_state.requested_tag.as_str()) else {
            let error_key = format!("missing_tag:{}", animation_state.requested_tag);
            if visual.last_error.as_deref() != Some(error_key.as_str()) {
                error!(
                    "Composed enemy {:?} requested missing animation tag '{}' from '{} {}'",
                    entity,
                    animation_state.requested_tag,
                    atlas_asset.atlas.entity,
                    atlas_asset.atlas.depth,
                );
                visual.last_error = Some(error_key);
            }
            visual.active_tag.clear();
            hide_composite(&mut composite, &mut visibility);
            continue;
        };

        if visual.active_tag != animation_state.requested_tag {
            visual.active_tag = animation_state.requested_tag.clone();
            visual.frame_index = initial_frame_index(animation);
            visual.frame_started_at_ms = now_ms;
            visual.ping_pong_forward =
                animation.direction != CachedAnimationDirection::PingPongReverse;
            visual.last_error = None;
        } else {
            advance_animation_frame(&mut visual, animation, now_ms);
        }

        let frame = &animation.frames[visual.frame_index];
        let Some((parts, metrics)) = compose_frame(frame, cache, atlas_bindings, entity) else {
            hide_composite(&mut composite, &mut visibility);
            continue;
        };

        composite.parts = parts;
        composite.origin = metrics.origin;
        composite.size = metrics.size;
        composite.frame_count = 1;
        *px_position = PxPosition(IVec2::new(
            position.x.round() as i32,
            position.y.round() as i32,
        ));
        *anchor = anchor_for_origin(metrics);
        *visibility = Visibility::Visible;
    }
}

impl CompositionAtlasAsset {
    fn prepare_runtime(&mut self) {
        if !matches!(self.runtime, CompositionAtlasRuntime::Unprepared) {
            return;
        }

        let prepared = build_runtime_cache(&self.atlas);
        self.runtime = match prepared {
            Ok(cache) => CompositionAtlasRuntime::Ready(cache),
            Err(reason) => {
                error!(
                    "Rejected composed atlas '{} {}': {}",
                    self.atlas.entity, self.atlas.depth, reason,
                );
                CompositionAtlasRuntime::Invalid(reason)
            }
        };
    }

    fn runtime(&self) -> Result<&CompositionAtlasCache, &str> {
        match &self.runtime {
            CompositionAtlasRuntime::Ready(cache) => Ok(cache),
            CompositionAtlasRuntime::Invalid(reason) => Err(reason.as_str()),
            CompositionAtlasRuntime::Unprepared => Err("unprepared"),
        }
    }

    fn is_invalid(&self) -> bool {
        matches!(self.runtime, CompositionAtlasRuntime::Invalid(_))
    }
}

fn build_runtime_cache(atlas: &CompositionAtlas) -> Result<CompositionAtlasCache, String> {
    if atlas.schema_version != SUPPORTED_COMPOSITION_SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema_version {} (expected {})",
            atlas.schema_version, SUPPORTED_COMPOSITION_SCHEMA_VERSION
        ));
    }

    let mut parts = Vec::with_capacity(atlas.parts.len());
    let mut part_ids = HashSet::with_capacity(atlas.parts.len());
    let mut draw_orders = HashSet::with_capacity(atlas.parts.len());
    for part in &atlas.parts {
        validate_part(part, &mut part_ids, &mut draw_orders)?;
        parts.push(CachedPart {
            id: part.id.clone(),
            draw_order: part.draw_order,
        });
    }
    parts.sort_by_key(|part| part.draw_order);

    let mut sprites = HashMap::with_capacity(atlas.sprites.len());
    for sprite in &atlas.sprites {
        if sprites.insert(sprite.id.clone(), sprite.clone()).is_some() {
            return Err(format!("duplicate sprite id '{}'", sprite.id));
        }
    }

    let mut animations = HashMap::with_capacity(atlas.animations.len());
    for animation in &atlas.animations {
        if animations.contains_key(animation.tag.as_str()) {
            return Err(format!("duplicate animation tag '{}'", animation.tag));
        }

        let direction = parse_animation_direction(animation.direction.as_str())?;
        let mut cached_frames = Vec::with_capacity(animation.frames.len());
        for frame in &animation.frames {
            let mut placements = HashMap::with_capacity(frame.parts.len());
            for placement in &frame.parts {
                if placement.opacity != u8::MAX {
                    return Err(format!(
                        "animation '{}' frame {} uses opacity {}; composed runtime currently requires fully opaque part placements",
                        animation.tag, frame.source_frame, placement.opacity
                    ));
                }
                if !part_ids.contains(placement.part_id.as_str()) {
                    return Err(format!(
                        "animation '{}' frame {} references missing part '{}'",
                        animation.tag, frame.source_frame, placement.part_id
                    ));
                }
                let sprite = sprites.get(placement.sprite_id.as_str()).ok_or_else(|| {
                    format!(
                        "animation '{}' frame {} references missing sprite '{}'",
                        animation.tag, frame.source_frame, placement.sprite_id
                    )
                })?;
                let cached = CachedPlacement {
                    sprite_id: placement.sprite_id.clone(),
                    bottom_left_offset: IVec2::new(
                        placement.offset.x,
                        -(placement.offset.y + sprite.rect.h as i32),
                    ),
                    size: UVec2::new(sprite.rect.w, sprite.rect.h),
                    flip_x: placement.flip_x,
                    flip_y: placement.flip_y,
                };

                if placements
                    .insert(placement.part_id.clone(), cached)
                    .is_some()
                {
                    return Err(format!(
                        "animation '{}' frame {} defines part '{}' more than once",
                        animation.tag, frame.source_frame, placement.part_id
                    ));
                }
            }

            cached_frames.push(CachedAnimationFrame {
                duration_ms: frame.duration_ms,
                placements,
            });
        }

        animations.insert(
            animation.tag.clone(),
            CachedAnimation {
                direction,
                repeats: animation.repeats,
                frames: cached_frames,
            },
        );
    }

    Ok(CompositionAtlasCache { parts, animations })
}

fn build_atlas_bindings(
    atlas: &CompositionAtlas,
    sprite_atlas: &PxSpriteAtlasAsset,
    atlas_handle: Handle<PxSpriteAtlasAsset>,
) -> Result<ComposedAtlasBindings, String> {
    let mut sprite_regions = HashMap::with_capacity(atlas.sprites.len());

    for sprite in &atlas.sprites {
        let region_id = sprite_atlas
            .region_id(&sprite.id)
            .ok_or_else(|| format!("sprite atlas is missing region '{}'", sprite.id))?;
        let region = sprite_atlas.region(region_id).ok_or_else(|| {
            format!(
                "sprite atlas resolved region {:?} for '{}' but it does not exist",
                region_id, sprite.id
            )
        })?;
        if region.frame_count() != 1 {
            return Err(format!(
                "sprite atlas region '{}' must contain exactly 1 frame, found {}",
                sprite.id,
                region.frame_count()
            ));
        }
        if region.frame_size != UVec2::new(sprite.rect.w, sprite.rect.h) {
            return Err(format!(
                "sprite atlas region '{}' has frame_size {:?}, expected {}x{}",
                sprite.id, region.frame_size, sprite.rect.w, sprite.rect.h
            ));
        }
        let frame = region.frame(0).ok_or_else(|| {
            format!(
                "sprite atlas region '{}' is missing its first frame rectangle",
                sprite.id
            )
        })?;
        if frame.x != sprite.rect.x
            || frame.y != sprite.rect.y
            || frame.w != sprite.rect.w
            || frame.h != sprite.rect.h
        {
            return Err(format!(
                "sprite atlas region '{}' frame rect {:?} does not match atlas.json rect ({}, {}, {}, {})",
                sprite.id, frame, sprite.rect.x, sprite.rect.y, sprite.rect.w, sprite.rect.h
            ));
        }
        if sprite_regions
            .insert(sprite.id.clone(), region_id)
            .is_some()
        {
            return Err(format!("duplicate sprite region binding '{}'", sprite.id));
        }
    }

    Ok(ComposedAtlasBindings {
        atlas: atlas_handle,
        sprite_regions,
    })
}

fn validate_part(
    part: &PartDefinition,
    part_ids: &mut HashSet<String>,
    draw_orders: &mut HashSet<u32>,
) -> Result<(), String> {
    if !part_ids.insert(part.id.clone()) {
        return Err(format!("duplicate part id '{}'", part.id));
    }
    if !draw_orders.insert(part.draw_order) {
        return Err(format!("duplicate draw_order {}", part.draw_order));
    }
    Ok(())
}

fn parse_animation_direction(direction: &str) -> Result<CachedAnimationDirection, String> {
    match direction {
        "forward" => Ok(CachedAnimationDirection::Forward),
        "reverse" => Ok(CachedAnimationDirection::Reverse),
        "ping_pong" => Ok(CachedAnimationDirection::PingPong),
        "ping_pong_reverse" => Ok(CachedAnimationDirection::PingPongReverse),
        _ => Err(format!("unsupported animation direction '{direction}'")),
    }
}

fn compose_frame(
    frame: &CachedAnimationFrame,
    cache: &CompositionAtlasCache,
    atlas_bindings: &ComposedAtlasBindings,
    entity: Entity,
) -> Option<(Vec<PxCompositePart>, CachedCompositeMetrics)> {
    let mut parts = Vec::new();
    let mut metrics_source = Vec::new();

    for part in &cache.parts {
        // Frames are stateless compositions: a missing placement means this part
        // is invisible for the current frame rather than inherited.
        let Some(placement) = frame.placements.get(part.id.as_str()) else {
            continue;
        };
        let Some(region_id) = atlas_bindings
            .sprite_regions
            .get(placement.sprite_id.as_str())
        else {
            error!(
                "Composed enemy {:?} is missing atlas region '{}' for part '{}'",
                entity, placement.sprite_id, part.id,
            );
            return None;
        };

        parts.push(
            PxCompositePart::atlas_region(atlas_bindings.atlas.clone(), *region_id)
                .with_offset(placement.bottom_left_offset)
                .with_flip(placement.flip_x, placement.flip_y),
        );
        metrics_source.push(placement);
    }

    let metrics = compute_composite_metrics(metrics_source.into_iter())?;
    Some((parts, metrics))
}

fn compute_composite_metrics<'a>(
    placements: impl IntoIterator<Item = &'a CachedPlacement>,
) -> Option<CachedCompositeMetrics> {
    let mut min = IVec2::ZERO;
    let mut max = IVec2::ZERO;
    let mut any = false;

    for placement in placements {
        let part_min = placement.bottom_left_offset;
        let part_max = part_min + placement.size.as_ivec2();

        if any {
            min = min.min(part_min);
            max = max.max(part_max);
        } else {
            min = part_min;
            max = part_max;
            any = true;
        }
    }

    if !any {
        return None;
    }

    let size = max - min;
    Some(CachedCompositeMetrics {
        origin: min,
        size: UVec2::new(size.x.max(0) as u32, size.y.max(0) as u32),
    })
}

fn anchor_for_origin(metrics: CachedCompositeMetrics) -> PxAnchor {
    if metrics.size.x == 0 || metrics.size.y == 0 {
        return PxAnchor::Center;
    }

    let origin = -metrics.origin;
    PxAnchor::Custom(Vec2::new(
        origin.x as f32 / metrics.size.x as f32,
        origin.y as f32 / metrics.size.y as f32,
    ))
}

fn advance_animation_frame(
    visual: &mut ComposedEnemyVisual,
    animation: &CachedAnimation,
    now_ms: u64,
) {
    if animation.frames.is_empty() {
        return;
    }

    if let Some(0) = animation.repeats {
        visual.frame_index = initial_frame_index(animation);
        return;
    }

    loop {
        let frame_duration = u64::from(animation.frames[visual.frame_index].duration_ms.max(1));
        if now_ms.saturating_sub(visual.frame_started_at_ms) < frame_duration {
            break;
        }

        visual.frame_started_at_ms = visual.frame_started_at_ms.saturating_add(frame_duration);
        visual.frame_index = next_frame_index(
            animation.direction,
            visual.frame_index,
            animation.frames.len(),
            &mut visual.ping_pong_forward,
        );
    }
}

fn initial_frame_index(animation: &CachedAnimation) -> usize {
    if animation.frames.is_empty() {
        return 0;
    }

    match animation.direction {
        CachedAnimationDirection::Reverse | CachedAnimationDirection::PingPongReverse => {
            animation.frames.len() - 1
        }
        CachedAnimationDirection::Forward | CachedAnimationDirection::PingPong => 0,
    }
}

fn next_frame_index(
    direction: CachedAnimationDirection,
    current: usize,
    frame_count: usize,
    ping_pong_forward: &mut bool,
) -> usize {
    if frame_count <= 1 {
        return 0;
    }

    match direction {
        CachedAnimationDirection::Reverse => {
            if current == 0 {
                frame_count - 1
            } else {
                current - 1
            }
        }
        CachedAnimationDirection::PingPong | CachedAnimationDirection::PingPongReverse => {
            if *ping_pong_forward {
                if current + 1 >= frame_count {
                    *ping_pong_forward = false;
                    current.saturating_sub(1)
                } else {
                    current + 1
                }
            } else if current == 0 {
                *ping_pong_forward = true;
                1.min(frame_count - 1)
            } else {
                current - 1
            }
        }
        CachedAnimationDirection::Forward => {
            if current + 1 >= frame_count {
                0
            } else {
                current + 1
            }
        }
    }
}

fn hide_composite(composite: &mut PxCompositeSprite, visibility: &mut Visibility) {
    composite.parts.clear();
    composite.origin = IVec2::ZERO;
    composite.size = UVec2::ZERO;
    composite.frame_count = 0;
    *visibility = Visibility::Hidden;
}

fn fail_ready_composed_enemy(
    commands: &mut Commands,
    entity: Entity,
    composite: &mut PxCompositeSprite,
    visibility: &mut Visibility,
    reason: &str,
) {
    error!(
        "Composed enemy {:?} failed after becoming ready: {}",
        entity, reason
    );
    hide_composite(composite, visibility);
    commands
        .entity(entity)
        .remove::<ComposedEnemyVisualReady>()
        .insert(ComposedEnemyVisualFailed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use asset_pipeline::aseprite::{
        Animation, AnimationFrame, AtlasSprite, PartPlacement, Point, Rect, Size,
    };
    use std::{fs, path::PathBuf};

    fn load_exported_mosquiton() -> CompositionAtlasAsset {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/sprites/enemies/mosquiton_3/atlas.json");
        let body = fs::read_to_string(path).expect("generated mosquiton atlas.json should exist");
        serde_json::from_str(&body).expect("generated mosquiton atlas.json should deserialize")
    }

    fn minimal_atlas() -> CompositionAtlas {
        CompositionAtlas {
            schema_version: SUPPORTED_COMPOSITION_SCHEMA_VERSION,
            entity: "example".to_string(),
            depth: 3,
            source: "example.aseprite".to_string(),
            canvas: Size { w: 16, h: 16 },
            origin: asset_pipeline::aseprite::Point { x: 8, y: 8 },
            atlas_image: "atlas.png".to_string(),
            parts: vec![PartDefinition {
                id: "body".to_string(),
                name: "Body".to_string(),
                draw_order: 0,
            }],
            sprites: vec![AtlasSprite {
                id: "sprite_0000".to_string(),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 4,
                    h: 4,
                },
            }],
            animations: vec![Animation {
                tag: "idle_stand".to_string(),
                direction: "forward".to_string(),
                repeats: None,
                frames: vec![AnimationFrame {
                    source_frame: 0,
                    duration_ms: 100,
                    parts: vec![PartPlacement {
                        part_id: "body".to_string(),
                        sprite_id: "sprite_0000".to_string(),
                        offset: Point { x: 0, y: 0 },
                        flip_x: false,
                        flip_y: false,
                        opacity: 255,
                    }],
                }],
            }],
        }
    }

    #[test]
    fn exported_mosquiton_manifest_deserializes() {
        let atlas = load_exported_mosquiton();
        assert_eq!(
            atlas.atlas.schema_version,
            SUPPORTED_COMPOSITION_SCHEMA_VERSION
        );
        assert_eq!(atlas.atlas.entity, "mosquiton");
        assert_eq!(atlas.atlas.depth, 3);
        assert!(!atlas.atlas.parts.is_empty());
        assert!(!atlas.atlas.sprites.is_empty());
        assert!(!atlas.atlas.animations.is_empty());
        assert_eq!(atlas.atlas.atlas_image, "atlas.png");
    }

    #[test]
    fn exported_mosquiton_parts_have_unique_ids() {
        let atlas = load_exported_mosquiton();
        let mut ids = HashSet::new();
        for part in &atlas.atlas.parts {
            assert!(
                ids.insert(part.id.clone()),
                "duplicate part id '{}': {:?}",
                part.id,
                atlas.atlas.parts
            );
        }
    }

    #[test]
    fn exported_mosquiton_contains_expected_tags() {
        let atlas = load_exported_mosquiton();
        let tags: HashSet<_> = atlas
            .atlas
            .animations
            .iter()
            .map(|animation| animation.tag.as_str())
            .collect();
        assert!(
            tags.contains("idle_stand"),
            "expected idle_stand tag in exported mosquiton atlas"
        );
        assert!(
            tags.contains("shoot_stand"),
            "expected shoot_stand tag in exported mosquiton atlas"
        );
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let mut atlas = minimal_atlas();
        atlas.schema_version = SUPPORTED_COMPOSITION_SCHEMA_VERSION + 1;

        let error = build_runtime_cache(&atlas).expect_err("schema version should be validated");
        assert!(error.contains("unsupported schema_version"));
    }

    #[test]
    fn rejects_frames_that_reference_missing_sprites() {
        let mut atlas = minimal_atlas();
        atlas.animations[0].frames[0].parts[0].sprite_id = "missing".to_string();

        let error = build_runtime_cache(&atlas).expect_err("missing sprite ids should fail");
        assert!(error.contains("references missing sprite"));
    }

    #[test]
    fn rejects_non_opaque_part_placements() {
        let mut atlas = minimal_atlas();
        atlas.animations[0].frames[0].parts[0].opacity = 200;

        let error = build_runtime_cache(&atlas).expect_err("non-opaque placements should fail");
        assert!(error.contains("requires fully opaque part placements"));
    }

    #[test]
    fn anchor_tracks_shared_origin_inside_composite_bounds() {
        let metrics = CachedCompositeMetrics {
            origin: IVec2::new(-10, -4),
            size: UVec2::new(20, 10),
        };

        let PxAnchor::Custom(anchor) = anchor_for_origin(metrics) else {
            panic!("expected custom anchor");
        };
        assert_eq!(anchor, Vec2::new(0.5, 0.4));
    }
}
