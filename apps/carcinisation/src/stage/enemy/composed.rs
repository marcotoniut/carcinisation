//! Runtime support for composed enemies exported from the Aseprite composition pipeline.
//!
//! The runtime keeps three concerns separate:
//! - visual composition from deduplicated atlas sprites
//! - semantic part identity used for tags and hierarchy
//! - gameplay routing through semantic part ids and shared health pools
//!
//! Some exported schema names are legacy. In particular, `CompositionAtlas`
//! represents the full composed asset manifest, not only atlas rectangles, and
//! `PartPose` represents a per-frame visual placement, not a pure transform
//! track. This module documents those contracts close to the runtime that
//! consumes them.

#![allow(
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

use crate::pixel::PxAssets;
use crate::stage::{
    components::{
        interactive::{Dead, Flickerer, Health},
        placement::Depth,
    },
    enemy::{components::composed_state::Dying, entity::EnemyType},
    messages::{ComposedAnimationCueMessage, PartDamageMessage},
    resources::StageTimeDomain,
};
use assert_assets_path::assert_assets_path;
use asset_pipeline::aseprite::{
    AnimationEventKind, CollisionShape as CompositionCollisionShape, CompositionAtlas,
    PartGameplayMetadata, validate_composition_atlas,
};
use bevy::{
    asset::{Asset, LoadState},
    prelude::*,
    reflect::{Reflect, TypePath},
};
use carcinisation_collision::{Collider, ColliderShape};
use seldom_pixel::{
    filter::{PxFilter, PxFilterAsset},
    prelude::{
        AtlasRect, AtlasRegionId, PxAnchor, PxCanvas, PxCompositePart, PxCompositeSprite,
        PxPosition, PxSpriteAtlasAsset, PxSubPosition,
    },
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

/// Current runtime schema version accepted by the composed-enemy renderer.
pub const SUPPORTED_COMPOSITION_SCHEMA_VERSION: u32 = 3;
const COMPOSED_ENEMY_ASSET_ROOT: &str = "sprites/enemies";
const COMPOSED_ENEMY_ATLAS_BASENAME: &str = "atlas";
/// Composed-part hit feedback should read as a brief local blink, not as the
/// longer whole-entity damage flicker used elsewhere in the stage.
const COMPOSED_PART_HIT_BLINK_PHASE: std::time::Duration = std::time::Duration::from_millis(60);
/// Two additional invert phases after the initial flash yields:
/// invert -> regular -> invert -> regular -> invert
/// for a total blink window of 300ms.
const COMPOSED_PART_HIT_BLINK_INVERT_CYCLES: u8 = 2;

#[derive(Asset, Clone, Debug, Deserialize, TypePath)]
/// Asset wrapper around the exported composed manifest plus the lazily-built runtime cache.
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
/// Runtime cache derived from a validated [`CompositionAtlas`].
///
/// This cache deliberately separates stable semantic ids from the visual draw
/// order used by rendering and collision resolution.
struct CompositionAtlasCache {
    visual_parts_in_draw_order: Vec<String>,
    parts_by_id: HashMap<String, CachedPart>,
    animations: HashMap<String, CachedAnimation>,
    entity_health_pool: Option<String>,
    health_pools: HashMap<String, u32>,
}

#[derive(Clone, Debug)]
/// Runtime view of one semantic part after definition/instance metadata merge.
struct CachedPart {
    id: String,
    parent_id: Option<String>,
    is_visual: bool,
    draw_order: u32,
    pivot: IVec2,
    tags: Vec<String>,
    gameplay: CachedPartGameplay,
}

#[derive(Clone, Debug)]
struct CachedAnimation {
    direction: CachedAnimationDirection,
    repeats: Option<u32>,
    frames: Vec<CachedAnimationFrame>,
}

#[derive(Clone, Debug)]
struct CachedAnimationFrame {
    source_frame: usize,
    duration_ms: u32,
    events: Vec<CachedAnimationEvent>,
    poses: HashMap<String, CachedPose>,
}

#[derive(Clone, Debug)]
struct CachedAnimationEvent {
    kind: AnimationEventKind,
    id: String,
    part_id: Option<String>,
    local_offset: IVec2,
}

#[derive(Clone, Debug)]
struct CachedPose {
    sprite_id: String,
    local_offset: IVec2,
    size: UVec2,
    flip_x: bool,
    flip_y: bool,
    visible: bool,
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

#[derive(Clone, Copy, Debug)]
struct ResolvedPartTransform {
    top_left: IVec2,
    pivot: IVec2,
}

#[derive(Clone, Debug, Default)]
struct CachedPartGameplay {
    targetable: bool,
    health_pool: Option<String>,
    armour: u32,
    durability: Option<u32>,
    breakable: bool,
    collisions: Vec<CachedCollisionVolume>,
}

#[derive(Clone, Debug)]
struct CachedCollisionVolume {
    shape: ColliderShape,
    offset: Vec2,
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
/// Resolved collision state for the active composed frame.
///
/// Entries are stored in ascending draw order; [`Self::point_collides`] walks
/// them in reverse so the visually front-most colliding part wins.
pub struct ComposedCollisionState {
    collisions: Vec<ResolvedPartCollision>,
}

#[derive(Clone, Debug, Reflect)]
/// A resolved gameplay-targetable collision volume bound to one semantic part id.
pub struct ResolvedPartCollision {
    pub part_id: String,
    pub collider: Collider,
    pub pivot_position: Vec2,
}

impl ComposedCollisionState {
    #[must_use]
    /// Resolves a world-space point to the visually front-most colliding semantic part.
    pub fn point_collides(&self, point_position: Vec2) -> Option<&ResolvedPartCollision> {
        self.collisions.iter().rev().find(|collision| {
            collision.collider.shape.point_collides(
                collision.pivot_position + collision.collider.offset,
                point_position,
            )
        })
    }

    #[must_use]
    pub fn collisions(&self) -> &[ResolvedPartCollision] {
        &self.collisions
    }

    fn clear(&mut self) {
        self.collisions.clear();
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
/// Mutable gameplay health pools for a live composed enemy instance.
pub struct ComposedHealthPools {
    pools: HashMap<String, u32>,
}

impl ComposedHealthPools {
    fn from_cache(cache: &CompositionAtlasCache) -> Self {
        Self::from_cache_with_entity_health_override(cache, None)
    }

    fn from_cache_with_entity_health_override(
        cache: &CompositionAtlasCache,
        entity_health_override: Option<u32>,
    ) -> Self {
        let mut pools = cache.health_pools.clone();
        if let (Some(override_health), Some(entity_health_pool)) =
            (entity_health_override, cache.entity_health_pool.as_deref())
            && let Some(pool_health) = pools.get_mut(entity_health_pool)
        {
            *pool_health = override_health;
        }

        Self { pools }
    }

    #[must_use]
    pub fn pools(&self) -> &HashMap<String, u32> {
        &self.pools
    }

    fn apply_damage(&mut self, pool_id: &str, amount: u32) -> Option<u32> {
        let value = self.pools.get_mut(pool_id)?;
        *value = value.saturating_sub(amount);
        Some(*value)
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
/// Mutable per-part gameplay state for a live composed enemy instance.
///
/// This tracks transient gameplay/runtime state such as durability, breakage,
/// and hit-blink presentation. Part ids remain the authoritative gameplay key;
/// sprite ids are never used to own part state.
pub struct ComposedPartStates {
    parts: HashMap<String, PartGameplayState>,
}

impl ComposedPartStates {
    fn from_cache(cache: &CompositionAtlasCache) -> Self {
        let parts = cache
            .parts_by_id
            .values()
            .filter_map(|part| {
                if !part.gameplay.targetable && part.gameplay.durability.is_none() {
                    return None;
                }

                let durability = part.gameplay.durability.unwrap_or_default();
                Some((
                    part.id.clone(),
                    PartGameplayState {
                        current_durability: durability,
                        max_durability: durability,
                        breakable: part.gameplay.breakable,
                        broken: false,
                        visible: true,
                        hit_blink: None,
                    },
                ))
            })
            .collect();

        Self { parts }
    }

    #[must_use]
    pub fn part(&self, part_id: &str) -> Option<&PartGameplayState> {
        self.parts.get(part_id)
    }

    fn part_mut(&mut self, part_id: &str) -> Option<&mut PartGameplayState> {
        self.parts.get_mut(part_id)
    }

    /// Returns an iterator over all (`part_id`, `part_state`) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &PartGameplayState)> {
        self.parts.iter()
    }

    /// Returns a mutable iterator over all (`part_id`, `part_state`) pairs.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut PartGameplayState)> {
        self.parts.iter_mut()
    }
}

#[derive(Clone, Debug, Reflect)]
/// Mutable gameplay state for one semantic part instance.
pub struct PartGameplayState {
    pub current_durability: u32,
    pub max_durability: u32,
    pub breakable: bool,
    pub broken: bool,
    /// Controls whether this part should be rendered. Used for death effects
    /// and other runtime visibility changes.
    pub visible: bool,
    /// Active part-local hit blink. This is runtime-only presentation state:
    /// authored gameplay metadata decides which parts are targetable, but the
    /// blink lifecycle is resolved entirely at runtime from damage messages.
    pub hit_blink: Option<PartHitBlinkState>,
}

#[derive(Clone, Debug, Reflect)]
pub struct PartHitBlinkState {
    pub phase_started_at_ms: u64,
    pub showing_invert: bool,
    pub remaining_invert_cycles: u8,
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
/// Debug/inspection view of the resolved visual parts in the active frame.
///
/// This list intentionally contains only resolved visual parts. Non-visual
/// semantic nodes are validated at load time but do not appear here until the
/// runtime supports transform-only semantic nodes.
pub struct ComposedResolvedParts {
    parts: Vec<ResolvedPartState>,
}

impl ComposedResolvedParts {
    #[must_use]
    pub fn parts(&self) -> &[ResolvedPartState] {
        &self.parts
    }

    fn clear(&mut self) {
        self.parts.clear();
    }
}

#[derive(Clone, Debug, Reflect)]
/// One resolved visual part with semantic gameplay metadata attached.
pub struct ResolvedPartState {
    pub part_id: String,
    pub parent_id: Option<String>,
    pub draw_order: u32,
    pub sprite_id: String,
    pub frame_size: UVec2,
    pub flip_x: bool,
    pub flip_y: bool,
    pub world_top_left_position: Vec2,
    pub world_pivot_position: Vec2,
    pub tags: Vec<String>,
    pub targetable: bool,
    pub health_pool: Option<String>,
    pub armour: u32,
    pub current_durability: Option<u32>,
    pub max_durability: Option<u32>,
    pub breakable: bool,
    pub broken: bool,
    pub blinking: bool,
    pub collisions: Vec<ResolvedCollisionVolume>,
}

impl ResolvedPartState {
    #[must_use]
    /// Resolves an authored local cue offset into world space for this visual part.
    ///
    /// The offset is authored in unflipped sprite-local pixels relative to the
    /// part pivot in authored top-left/Y-down space. Runtime mirrors the offset
    /// when the resolved part is flipped so event origins stay attached to
    /// semantic features like a mouth or hand, then converts that authored
    /// point into world bottom-left/Y-up space.
    pub fn world_point_from_local_offset(&self, local_offset: IVec2) -> Vec2 {
        let x = if self.flip_x {
            self.frame_size.x as f32 - 1.0 - local_offset.x as f32
        } else {
            local_offset.x as f32
        };
        let y = if self.flip_y {
            self.frame_size.y as f32 - 1.0 - local_offset.y as f32
        } else {
            local_offset.y as f32
        };

        self.world_pivot_position + Vec2::new(x, -y)
    }
}

#[derive(Clone, Debug, Reflect)]
/// One resolved collision shape expressed in world space relative to a part pivot.
///
/// Runtime debug state currently preserves geometry only. Collision ids, roles,
/// and tags remain in asset metadata and should be threaded through once live
/// gameplay consumes them.
pub struct ResolvedCollisionVolume {
    pub shape: ColliderShape,
    pub offset: Vec2,
}

/// Generic animation-state selection surface for composed enemies.
///
/// Species-specific enemy logic should update only this component; the composed
/// renderer consumes it without knowing which enemy type authored the request.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct ComposedAnimationState {
    pub requested_tag: String,
    pub part_overrides: Vec<ComposedAnimationOverride>,
}

impl ComposedAnimationState {
    #[must_use]
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            requested_tag: tag.into(),
            part_overrides: Vec::new(),
        }
    }

    /// Replaces the part-scoped override tracks that should resolve on top of
    /// [`Self::requested_tag`].
    ///
    /// Resolution order is deterministic:
    /// 1. overrides are checked in slice order
    /// 2. the first matching override that authors a pose for a part wins
    /// 3. the base `requested_tag` is used as the fallback source
    ///
    /// Missing per-part poses in an override are not treated as errors; they
    /// fall back to the next lower-priority source. Missing tags are still
    /// hard failures because they indicate a broken runtime/asset contract.
    pub fn set_part_overrides(
        &mut self,
        overrides: impl IntoIterator<Item = ComposedAnimationOverride>,
    ) {
        self.part_overrides.clear();
        self.part_overrides.extend(overrides);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
/// A higher-priority animation source applied only to matching semantic parts.
pub struct ComposedAnimationOverride {
    pub tag: String,
    pub selector: ComposedPartSelector,
    /// When true, only the `sprite_id` is taken from the override animation,
    /// while position (`local_offset`) is preserved from the base animation.
    /// This prevents misalignment when swapping only visual sprites (like death faces).
    pub sprite_only: bool,
}

impl ComposedAnimationOverride {
    #[must_use]
    pub fn for_part_tags(
        tag: impl Into<String>,
        part_tags: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            tag: tag.into(),
            selector: ComposedPartSelector::for_tags(part_tags),
            sprite_only: false,
        }
    }

    /// Creates an override that only swaps `sprite_ids` while preserving base animation positions.
    ///
    /// This is useful for visual-only changes like death faces that should overlay
    /// the current pose without causing position misalignment.
    #[must_use]
    pub fn for_part_tags_sprite_only(
        tag: impl Into<String>,
        part_tags: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            tag: tag.into(),
            selector: ComposedPartSelector::for_tags(part_tags),
            sprite_only: true,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Reflect)]
/// Semantic selector used to route an override track to a subset of parts.
///
/// A selector matches when either:
/// - the semantic part id is listed in `part_ids`, or
/// - any merged semantic tag is listed in `part_tags`
///
/// Empty selectors are rejected for override tracks so a fallback/base source
/// remains explicit.
pub struct ComposedPartSelector {
    pub part_ids: Vec<String>,
    pub part_tags: Vec<String>,
}

impl ComposedPartSelector {
    #[must_use]
    pub fn for_tags(tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            part_ids: Vec::new(),
            part_tags: tags.into_iter().map(Into::into).collect(),
        }
    }

    fn is_empty(&self) -> bool {
        self.part_ids.is_empty() && self.part_tags.is_empty()
    }

    fn matches(&self, part: &CachedPart) -> bool {
        self.part_ids.iter().any(|id| id == &part.id)
            || self
                .part_tags
                .iter()
                .any(|tag| part.tags.iter().any(|part_tag| part_tag == tag))
    }
}

#[derive(Component, Clone, Debug, Default)]
pub struct ComposedAtlasBindings {
    atlas: Handle<PxSpriteAtlasAsset>,
    sprite_regions: HashMap<String, AtlasRegionId>,
    sprite_rects: HashMap<String, AtlasRect>,
}

impl ComposedAtlasBindings {
    #[must_use]
    pub fn atlas_handle(&self) -> &Handle<PxSpriteAtlasAsset> {
        &self.atlas
    }

    #[must_use]
    pub fn sprite_rect(&self, sprite_id: &str) -> Option<AtlasRect> {
        self.sprite_rects.get(sprite_id).copied()
    }
}

#[derive(Component, Clone, Debug)]
pub struct ComposedEnemyVisual {
    pub atlas_manifest: Handle<CompositionAtlasAsset>,
    pub sprite_atlas: Handle<PxSpriteAtlasAsset>,
    track_states: Vec<ComposedTrackPlaybackState>,
    last_error: Option<String>,
}

impl ComposedEnemyVisual {
    #[must_use]
    pub fn for_enemy(asset_server: &AssetServer, enemy_type: EnemyType, depth: Depth) -> Self {
        let base_path = composed_enemy_asset_base_path(enemy_type, depth);

        Self {
            atlas_manifest: asset_server.load(composed_enemy_manifest_path(&base_path)),
            sprite_atlas: asset_server.load(composed_enemy_sprite_atlas_path(&base_path)),
            track_states: Vec::new(),
            last_error: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RequestedAnimationTrack {
    tag: String,
    selector: Option<ComposedPartSelector>,
    sprite_only: bool,
}

#[derive(Clone, Debug)]
struct ComposedTrackPlaybackState {
    request: RequestedAnimationTrack,
    frame_index: usize,
    frame_started_at_ms: u64,
    ping_pong_forward: bool,
    completed_loops: u32,
}

#[derive(Clone, Debug)]
struct FiredAnimationCue {
    tag: String,
    frame_index: usize,
    source_frame: usize,
    kind: AnimationEventKind,
    id: String,
    part_id: Option<String>,
    local_offset: IVec2,
}

#[derive(Clone, Debug)]
struct ResolvedAnimationFrame {
    poses: HashMap<String, CachedPose>,
    fired_cues: Vec<FiredAnimationCue>,
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
    mut query: Query<
        (
            Entity,
            &ComposedEnemyVisual,
            &Depth,
            Option<&mut Health>,
            Option<&crate::stage::components::interactive::HealthOverride>,
        ),
        (
            Without<ComposedEnemyVisualReady>,
            Without<ComposedEnemyVisualFailed>,
        ),
    >,
) {
    for (entity, visual, depth, health, health_override) in &mut query {
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
            Ok(cache) => match build_atlas_bindings(
                &atlas_asset.atlas,
                sprite_atlas,
                visual.sprite_atlas.clone(),
            ) {
                Ok(bindings) => {
                    let health_override = health_override.map(|override_health| override_health.0);
                    let initial_health_pools =
                        ComposedHealthPools::from_cache_with_entity_health_override(
                            cache,
                            health_override,
                        );
                    if let (Some(entity_health_pool), Some(mut health)) =
                        (cache.entity_health_pool.as_deref(), health)
                        && let Some(current) = initial_health_pools.pools().get(entity_health_pool)
                    {
                        health.0 = *current;
                    }
                    commands.entity(entity).insert((
                        bindings,
                        ComposedCollisionState::default(),
                        initial_health_pools,
                        ComposedPartStates::from_cache(cache),
                        ComposedResolvedParts::default(),
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
            Err("unprepared") => {}
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
    mut cue_writer: MessageWriter<ComposedAnimationCueMessage>,
    filters: PxAssets<PxFilter>,
    mut root_query: Query<
        (
            Entity,
            &mut ComposedEnemyVisual,
            &ComposedAnimationState,
            &PxSubPosition,
            &ComposedAtlasBindings,
            &mut ComposedCollisionState,
            &mut ComposedPartStates,
            &mut ComposedResolvedParts,
            &mut PxCompositeSprite,
            &mut PxPosition,
            &mut PxAnchor,
            &mut Visibility,
            Option<&Dying>,
        ),
        With<ComposedEnemyVisualReady>,
    >,
) {
    let now_ms = stage_time.elapsed().as_millis() as u64;
    let invert_filter = filters.load(assert_assets_path!("filter/invert.px_filter.png"));

    for (
        entity,
        mut visual,
        animation_state,
        position,
        atlas_bindings,
        mut collision_state,
        mut part_states,
        mut resolved_part_states,
        mut composite,
        mut px_position,
        mut anchor,
        mut visibility,
        dying,
    ) in &mut root_query
    {
        // Freeze animation time for dying entities so they show death face on their last pose frame
        let animation_time_ms = if let Some(dying) = dying {
            dying.started.as_millis() as u64
        } else {
            now_ms
        };

        let Some(atlas_asset) = atlas_assets.get(&visual.atlas_manifest) else {
            fail_ready_composed_enemy(
                &mut commands,
                entity,
                &mut collision_state,
                &mut resolved_part_states,
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
                &mut collision_state,
                &mut resolved_part_states,
                &mut composite,
                &mut visibility,
                "composed atlas cache became unavailable after the visual was marked ready",
            );
            continue;
        };

        let requested_tracks = requested_animation_tracks(animation_state);
        let resolved_frame = match resolve_requested_animation_frame(
            &mut visual,
            &requested_tracks,
            cache,
            animation_time_ms,
        ) {
            Ok(frame) => frame,
            Err(reason) => {
                let error_key = format!("resolution:{reason}");
                if visual.last_error.as_deref() != Some(error_key.as_str()) {
                    error!(
                        "Composed enemy {:?} failed to resolve animation sources for '{} {}': {}",
                        entity, atlas_asset.atlas.entity, atlas_asset.atlas.depth, reason,
                    );
                    visual.last_error = Some(error_key);
                }
                hide_composite(
                    &mut collision_state,
                    &mut resolved_part_states,
                    &mut composite,
                    &mut visibility,
                );
                continue;
            }
        };
        visual.last_error = None;

        // Don't advance hit blinks for dying entities - freeze all flickering
        if dying.is_none() {
            advance_part_hit_blinks(&mut part_states, animation_time_ms);
        }

        for cue in &resolved_frame.fired_cues {
            cue_writer.write(ComposedAnimationCueMessage {
                entity,
                tag: cue.tag.clone(),
                frame_index: cue.frame_index,
                source_frame: cue.source_frame,
                kind: cue.kind,
                id: cue.id.clone(),
                part_id: cue.part_id.clone(),
                local_offset: asset_pipeline::aseprite::Point {
                    x: cue.local_offset.x,
                    y: cue.local_offset.y,
                },
            });
        }

        let Some((parts, metrics, resolved_parts)) = compose_frame(
            &resolved_frame.poses,
            cache,
            atlas_bindings,
            &part_states,
            &invert_filter,
            entity,
        ) else {
            hide_composite(
                &mut collision_state,
                &mut resolved_part_states,
                &mut composite,
                &mut visibility,
            );
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
        collision_state.collisions =
            build_collision_state(cache, &resolved_parts, &part_states, position.0);
        resolved_part_states.parts = build_resolved_part_states(
            cache,
            &resolved_frame.poses,
            &resolved_parts,
            &part_states,
            position.0,
        );
        *anchor = anchor_for_origin(metrics);
        *visibility = Visibility::Visible;
    }
}

pub fn apply_composed_part_damage(
    mut commands: Commands,
    mut event_reader: MessageReader<PartDamageMessage>,
    atlas_assets: Res<Assets<CompositionAtlasAsset>>,
    mut query: Query<
        (
            &ComposedEnemyVisual,
            &mut ComposedHealthPools,
            &mut ComposedPartStates,
            Option<&mut Health>,
        ),
        Without<Dead>,
    >,
) {
    for event in event_reader.read() {
        let Ok((visual, mut health_pools, mut part_states, health)) = query.get_mut(event.entity)
        else {
            continue;
        };
        let Some(atlas_asset) = atlas_assets.get(&visual.atlas_manifest) else {
            error!(
                "Composed part damage for {:?} dropped because atlas manifest is unavailable",
                event.entity
            );
            continue;
        };
        let Ok(cache) = atlas_asset.runtime() else {
            error!(
                "Composed part damage for {:?} dropped because atlas runtime cache is unavailable",
                event.entity
            );
            continue;
        };

        let Ok(result) = apply_part_damage(
            cache,
            &mut health_pools,
            &mut part_states,
            &event.part_id,
            event.value,
        ) else {
            error!(
                "Rejected composed part damage for {:?} part '{}'",
                event.entity, event.part_id
            );
            continue;
        };
        info!(
            "Composed damage {:?} part '{}' -> durability {:?}, pool {:?} (remaining {:?})",
            event.entity,
            event.part_id,
            result.remaining_durability,
            result.pool_id,
            result.remaining_health
        );

        if let Some(entity_health_pool) = cache.entity_health_pool.as_deref()
            && result.pool_id.as_deref() == Some(entity_health_pool)
        {
            if let Some(mut health) = health
                && let Some(remaining_health) = result.remaining_health
            {
                health.0 = remaining_health;
            }
            if result.remaining_health == Some(0) {
                commands.entity(event.entity).insert(Dead);
            }
        }
    }
}

pub fn check_composed_damage_flicker_taken(
    stage_time: Res<Time<StageTimeDomain>>,
    mut reader: MessageReader<PartDamageMessage>,
    mut query: Query<&mut ComposedPartStates, (With<Flickerer>, Without<Dead>)>,
) {
    let now_ms = stage_time.elapsed().as_millis() as u64;

    for event in reader.read() {
        let Ok(mut part_states) = query.get_mut(event.entity) else {
            continue;
        };
        let Some(part_state) = part_states.part_mut(&event.part_id) else {
            continue;
        };
        if part_state.broken {
            continue;
        }

        part_state.hit_blink = Some(PartHitBlinkState {
            phase_started_at_ms: now_ms,
            showing_invert: true,
            remaining_invert_cycles: COMPOSED_PART_HIT_BLINK_INVERT_CYCLES,
        });
    }
}

#[derive(Debug, PartialEq, Eq)]
struct AppliedPartDamage {
    pool_id: Option<String>,
    remaining_health: Option<u32>,
    remaining_durability: Option<u32>,
    broke_part: bool,
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
    validate_composition_atlas(atlas).map_err(|error| error.to_string())?;

    let definition_lookup: HashMap<&str, _> = atlas
        .part_definitions
        .iter()
        .map(|definition| (definition.id.as_str(), definition))
        .collect();

    let mut parts_by_id = HashMap::with_capacity(atlas.parts.len());
    let mut visual_parts_in_draw_order = Vec::new();
    for part in &atlas.parts {
        let Some(definition) = definition_lookup.get(part.definition_id.as_str()) else {
            return Err(format!(
                "part '{}' references missing definition '{}'",
                part.id, part.definition_id
            ));
        };
        parts_by_id.insert(
            part.id.clone(),
            CachedPart {
                id: part.id.clone(),
                parent_id: part.parent_id.clone(),
                is_visual: part.source_layer.is_some(),
                draw_order: part.draw_order,
                pivot: IVec2::new(part.pivot.x, part.pivot.y),
                tags: merge_tags(&definition.tags, &part.tags),
                gameplay: merge_gameplay(&definition.gameplay, &part.gameplay),
            },
        );
        if part.source_layer.is_some() {
            visual_parts_in_draw_order.push(part.id.clone());
        }
    }
    visual_parts_in_draw_order.sort_by_key(|part_id| {
        parts_by_id
            .get(part_id.as_str())
            .map_or(u32::MAX, |part| part.draw_order)
    });

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
            let mut poses = HashMap::with_capacity(frame.parts.len());
            for pose in &frame.parts {
                if pose.opacity != u8::MAX {
                    return Err(format!(
                        "animation '{}' frame {} uses opacity {}; composed runtime currently requires fully opaque part placements",
                        animation.tag, frame.source_frame, pose.opacity
                    ));
                }
                let Some(_part) = parts_by_id.get(pose.part_id.as_str()) else {
                    return Err(format!(
                        "animation '{}' frame {} references missing part '{}'",
                        animation.tag, frame.source_frame, pose.part_id
                    ));
                };
                let sprite = sprites.get(pose.sprite_id.as_str()).ok_or_else(|| {
                    format!(
                        "animation '{}' frame {} references missing sprite '{}'",
                        animation.tag, frame.source_frame, pose.sprite_id
                    )
                })?;
                let cached = CachedPose {
                    sprite_id: pose.sprite_id.clone(),
                    local_offset: IVec2::new(pose.local_offset.x, pose.local_offset.y),
                    size: UVec2::new(sprite.rect.w, sprite.rect.h),
                    flip_x: pose.flip_x,
                    flip_y: pose.flip_y,
                    visible: pose.visible,
                };

                if poses.insert(pose.part_id.clone(), cached).is_some() {
                    return Err(format!(
                        "animation '{}' frame {} defines part '{}' more than once",
                        animation.tag, frame.source_frame, pose.part_id
                    ));
                }
            }

            for (part_id, pose) in &poses {
                if !pose.visible {
                    continue;
                }
                let part = parts_by_id
                    .get(part_id.as_str())
                    .expect("validated part id");
                validate_parent_pose_chain(
                    &parts_by_id,
                    &poses,
                    part,
                    &animation.tag,
                    frame.source_frame,
                )?;
            }

            cached_frames.push(CachedAnimationFrame {
                source_frame: frame.source_frame,
                duration_ms: frame.duration_ms,
                events: frame
                    .events
                    .iter()
                    .map(|event| CachedAnimationEvent {
                        kind: event.kind,
                        id: event.id.clone(),
                        part_id: event.part_id.clone(),
                        local_offset: IVec2::new(event.local_offset.x, event.local_offset.y),
                    })
                    .collect(),
                poses,
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

    let health_pools = atlas
        .gameplay
        .health_pools
        .iter()
        .map(|pool| (pool.id.clone(), pool.max_health))
        .collect();

    Ok(CompositionAtlasCache {
        visual_parts_in_draw_order,
        parts_by_id,
        animations,
        entity_health_pool: atlas.gameplay.entity_health_pool.clone(),
        health_pools,
    })
}

fn build_atlas_bindings(
    atlas: &CompositionAtlas,
    sprite_atlas: &PxSpriteAtlasAsset,
    atlas_handle: Handle<PxSpriteAtlasAsset>,
) -> Result<ComposedAtlasBindings, String> {
    let mut sprite_regions = HashMap::with_capacity(atlas.sprites.len());
    let mut sprite_rects = HashMap::with_capacity(atlas.sprites.len());

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
        if sprite_rects.insert(sprite.id.clone(), frame).is_some() {
            return Err(format!("duplicate sprite rect binding '{}'", sprite.id));
        }
    }

    Ok(ComposedAtlasBindings {
        atlas: atlas_handle,
        sprite_regions,
        sprite_rects,
    })
}

fn merge_tags(definition_tags: &[String], instance_tags: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut merged = Vec::with_capacity(definition_tags.len() + instance_tags.len());

    for tag in definition_tags.iter().chain(instance_tags.iter()) {
        if seen.insert(tag.as_str()) {
            merged.push(tag.clone());
        }
    }

    merged
}

fn merge_gameplay(
    definition: &PartGameplayMetadata,
    instance: &PartGameplayMetadata,
) -> CachedPartGameplay {
    let collisions = definition
        .collision
        .iter()
        .chain(instance.collision.iter())
        .map(|collision| CachedCollisionVolume {
            shape: composition_collision_shape_to_runtime(&collision.shape),
            offset: composition_collision_offset(&collision.shape),
        })
        .collect();

    CachedPartGameplay {
        targetable: instance
            .targetable
            .or(definition.targetable)
            .unwrap_or(false),
        health_pool: instance
            .health_pool
            .clone()
            .or_else(|| definition.health_pool.clone()),
        armour: definition.armour.saturating_add(instance.armour),
        durability: instance.durability.or(definition.durability),
        breakable: instance.breakable.or(definition.breakable).unwrap_or(false),
        collisions,
    }
}

fn apply_part_damage(
    cache: &CompositionAtlasCache,
    health_pools: &mut ComposedHealthPools,
    part_states: &mut ComposedPartStates,
    part_id: &str,
    value: u32,
) -> Result<AppliedPartDamage, String> {
    let part = cache
        .parts_by_id
        .get(part_id)
        .ok_or_else(|| format!("unknown composed part '{part_id}'"))?;
    if !part.gameplay.targetable {
        return Err(format!(
            "composed part '{part_id}' is not gameplay-targetable"
        ));
    }

    let adjusted_damage = value.saturating_sub(part.gameplay.armour);
    let mut remaining_damage = adjusted_damage;
    let mut remaining_durability = None;
    let mut broke_part = false;

    if let Some(max_durability) = part.gameplay.durability {
        let state = part_states.part_mut(part_id).ok_or_else(|| {
            format!(
                "composed part '{part_id}' expected durability state for max durability {max_durability}"
            )
        })?;
        if state.broken {
            return Err(format!("composed part '{part_id}' is already broken"));
        }

        let absorbed = remaining_damage.min(state.current_durability);
        state.current_durability = state.current_durability.saturating_sub(absorbed);
        remaining_damage = remaining_damage.saturating_sub(absorbed);
        if state.breakable && state.current_durability == 0 {
            state.broken = true;
            state.visible = false; // Hide broken parts immediately
            broke_part = true;
        }
        remaining_durability = Some(state.current_durability);
    }

    let pool_id = part.gameplay.health_pool.clone();
    let remaining_health = if remaining_damage == 0 {
        None
    } else if let Some(pool_id) = pool_id.as_deref() {
        Some(
            health_pools
                .apply_damage(pool_id, remaining_damage)
                .ok_or_else(|| format!("unknown composed health pool '{pool_id}'"))?,
        )
    } else if adjusted_damage == 0 || part.gameplay.durability.is_some() {
        None
    } else {
        return Err(format!(
            "composed part '{part_id}' does not own a health pool or durability"
        ));
    };

    Ok(AppliedPartDamage {
        pool_id,
        remaining_health,
        remaining_durability,
        broke_part,
    })
}

fn composition_collision_shape_to_runtime(shape: &CompositionCollisionShape) -> ColliderShape {
    match shape {
        CompositionCollisionShape::Circle { radius, .. } => ColliderShape::Circle(*radius),
        CompositionCollisionShape::Box { size, .. } => {
            ColliderShape::Box(Vec2::new(size.x, size.y))
        }
    }
}

fn composition_collision_offset(shape: &CompositionCollisionShape) -> Vec2 {
    match shape {
        CompositionCollisionShape::Circle { offset, .. }
        | CompositionCollisionShape::Box { offset, .. } => Vec2::new(offset.x, offset.y),
    }
}

fn validate_parent_pose_chain(
    parts_by_id: &HashMap<String, CachedPart>,
    poses: &HashMap<String, CachedPose>,
    part: &CachedPart,
    animation_tag: &str,
    source_frame: usize,
) -> Result<(), String> {
    let mut parent_id = part.parent_id.as_deref();
    while let Some(parent) = parent_id {
        let parent_part = parts_by_id.get(parent).ok_or_else(|| {
            format!(
                "animation '{animation_tag}' frame {source_frame} references missing parent '{parent}'"
            )
        })?;
        if parent_part.is_visual && !poses.contains_key(parent) {
            return Err(format!(
                "animation '{}' frame {} renders child '{}' without visible parent '{}'",
                animation_tag, source_frame, part.id, parent
            ));
        }
        parent_id = parent_part.parent_id.as_deref();
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

fn requested_animation_tracks(
    animation_state: &ComposedAnimationState,
) -> Vec<RequestedAnimationTrack> {
    let mut tracks = Vec::with_capacity(animation_state.part_overrides.len() + 1);
    tracks.extend(animation_state.part_overrides.iter().map(|override_track| {
        RequestedAnimationTrack {
            tag: override_track.tag.clone(),
            selector: Some(override_track.selector.clone()),
            sprite_only: override_track.sprite_only,
        }
    }));
    tracks.push(RequestedAnimationTrack {
        tag: animation_state.requested_tag.clone(),
        selector: None,
        sprite_only: false,
    });
    tracks
}

/// Resolves the final composed frame from one base animation source plus any
/// higher-priority part-scoped override tracks.
///
/// Track timing is independent per source tag. Override tracks are evaluated in
/// order, then the base `requested_tag` is used as the final fallback.
/// Missing poses fall through to lower-priority sources; missing tags fail
/// explicitly because they indicate an invalid runtime request or stale asset.
fn resolve_requested_animation_frame(
    visual: &mut ComposedEnemyVisual,
    requested_tracks: &[RequestedAnimationTrack],
    cache: &CompositionAtlasCache,
    now_ms: u64,
) -> Result<ResolvedAnimationFrame, String> {
    let fired_cues =
        sync_animation_track_states(&mut visual.track_states, requested_tracks, cache, now_ms)?;

    let mut poses = HashMap::new();
    for part_id in &cache.visual_parts_in_draw_order {
        let Some(part) = cache.parts_by_id.get(part_id.as_str()) else {
            continue;
        };
        let Some(pose) =
            resolve_part_pose_from_tracks(part, requested_tracks, &visual.track_states, cache)?
        else {
            continue;
        };
        poses.insert(part.id.clone(), pose);
    }

    Ok(ResolvedAnimationFrame { poses, fired_cues })
}

/// Synchronizes the runtime playback cursors with the currently requested
/// animation sources.
///
/// Unchanged tracks keep their own frame clocks, which is what allows wing
/// flapping to continue while action-tag tracks switch independently.
fn sync_animation_track_states(
    track_states: &mut Vec<ComposedTrackPlaybackState>,
    requested_tracks: &[RequestedAnimationTrack],
    cache: &CompositionAtlasCache,
    now_ms: u64,
) -> Result<Vec<FiredAnimationCue>, String> {
    let mut fired_cues = Vec::new();
    if track_states.len() > requested_tracks.len() {
        track_states.truncate(requested_tracks.len());
    }

    for (index, request) in requested_tracks.iter().enumerate() {
        if let Some(selector) = request.selector.as_ref()
            && selector.is_empty()
        {
            return Err(format!(
                "override track '{}' must target at least one semantic part id or tag",
                request.tag
            ));
        }

        let animation = cache.animations.get(request.tag.as_str()).ok_or_else(|| {
            format!(
                "requested animation tag '{}' is missing from the composed atlas",
                request.tag
            )
        })?;

        let needs_reset = track_states
            .get(index)
            .is_none_or(|state| state.request != *request);

        if needs_reset {
            let replacement = ComposedTrackPlaybackState {
                request: request.clone(),
                frame_index: initial_frame_index(animation),
                frame_started_at_ms: now_ms,
                ping_pong_forward: animation.direction != CachedAnimationDirection::PingPongReverse,
                completed_loops: 0,
            };
            fired_cues.extend(fired_frame_cues(
                request.tag.as_str(),
                animation,
                replacement.frame_index,
            ));
            if index < track_states.len() {
                track_states[index] = replacement;
            } else {
                track_states.push(replacement);
            }
            continue;
        }

        let track_state = &mut track_states[index];
        fired_cues.extend(advance_track_playback(
            track_state,
            request,
            animation,
            now_ms,
        ));
    }

    Ok(fired_cues)
}

/// Resolves the final pose for a part by merging base animation with overrides.
///
/// Sprite-only overrides preserve the base animation's position while swapping the sprite.
/// This prevents misalignment when applying visual-only changes like death faces.
fn resolve_part_pose_from_tracks(
    part: &CachedPart,
    requested_tracks: &[RequestedAnimationTrack],
    track_states: &[ComposedTrackPlaybackState],
    cache: &CompositionAtlasCache,
) -> Result<Option<CachedPose>, String> {
    // Track sprite-only override if found
    let mut sprite_override: Option<CachedPose> = None;

    for (request, playback) in requested_tracks.iter().zip(track_states.iter()) {
        let selector_matches = request
            .selector
            .as_ref()
            .is_none_or(|selector| selector.matches(part));
        if !selector_matches {
            continue;
        }

        let animation = cache.animations.get(request.tag.as_str()).ok_or_else(|| {
            format!(
                "requested animation tag '{}' is missing from the composed atlas",
                request.tag
            )
        })?;
        if animation.frames.is_empty() {
            return Err(format!(
                "requested animation tag '{}' has no frames after runtime validation",
                request.tag
            ));
        }
        if playback.frame_index >= animation.frames.len() {
            return Err(format!(
                "requested animation tag '{}' resolved invalid frame index {} for {} frames",
                request.tag,
                playback.frame_index,
                animation.frames.len()
            ));
        }

        if let Some(pose) = animation.frames[playback.frame_index]
            .poses
            .get(part.id.as_str())
        {
            if request.sprite_only {
                // For sprite-only overrides, save the sprite and continue
                // looking for the base pose to preserve its position
                sprite_override = Some(pose.clone());
            } else {
                if let Some(sprite_only_pose) = sprite_override.take() {
                    // We found the base pose and have a sprite override - merge them
                    return Ok(Some(CachedPose {
                        sprite_id: sprite_only_pose.sprite_id,
                        size: sprite_only_pose.size,
                        flip_x: sprite_only_pose.flip_x,
                        flip_y: sprite_only_pose.flip_y,
                        visible: sprite_only_pose.visible,
                        // Preserve position from base animation
                        local_offset: pose.local_offset,
                    }));
                }
                // Normal override - use it entirely
                return Ok(Some(pose.clone()));
            }
        }
    }

    // If we only found a sprite override but no base, return it as-is
    // (This shouldn't normally happen with proper setup, but handle gracefully)
    Ok(sprite_override)
}

fn compose_frame(
    poses: &HashMap<String, CachedPose>,
    cache: &CompositionAtlasCache,
    atlas_bindings: &ComposedAtlasBindings,
    part_states: &ComposedPartStates,
    invert_filter: &Handle<PxFilterAsset>,
    entity: Entity,
) -> Option<(
    Vec<PxCompositePart>,
    CachedCompositeMetrics,
    HashMap<String, ResolvedPartTransform>,
)> {
    let mut parts = Vec::new();
    let mut metrics_source = Vec::new();
    let mut resolved_pivots = HashMap::new();
    let mut resolved_parts = HashMap::new();

    for part_id in &cache.visual_parts_in_draw_order {
        let Some(part) = cache.parts_by_id.get(part_id.as_str()) else {
            continue;
        };
        let Some(pose) = poses.get(part.id.as_str()) else {
            continue;
        };
        if !pose.visible {
            continue;
        }
        // Check runtime visibility state (used for death effects, etc.)
        if let Some(part_state) = part_states.part(part.id.as_str())
            && !part_state.visible
        {
            continue;
        }
        let Some(region_id) = atlas_bindings.sprite_regions.get(pose.sprite_id.as_str()) else {
            error!(
                "Composed enemy {:?} is missing atlas region '{}' for part '{}'",
                entity, pose.sprite_id, part.id,
            );
            return None;
        };
        let absolute_pivot = resolve_pivot(part, poses, cache, &mut resolved_pivots)?;
        let absolute_top_left = absolute_pivot - part.pivot;
        let bottom_left_offset = IVec2::new(
            absolute_top_left.x,
            -(absolute_top_left.y + pose.size.y as i32),
        );

        parts.push(
            PxCompositePart::atlas_region(atlas_bindings.atlas.clone(), *region_id)
                .with_offset(bottom_left_offset)
                .with_filter(
                    part_states
                        .part(part.id.as_str())
                        .and_then(|state| state.hit_blink.as_ref())
                        .filter(|state| state.showing_invert)
                        .map(|_| invert_filter.clone()),
                )
                .with_flip(pose.flip_x, pose.flip_y),
        );
        metrics_source.push((bottom_left_offset, pose.size));
        resolved_parts.insert(
            part.id.clone(),
            ResolvedPartTransform {
                top_left: absolute_top_left,
                pivot: absolute_pivot,
            },
        );
    }

    let metrics = compute_composite_metrics(metrics_source.into_iter())?;
    Some((parts, metrics, resolved_parts))
}

fn resolve_pivot(
    part: &CachedPart,
    poses: &HashMap<String, CachedPose>,
    cache: &CompositionAtlasCache,
    resolved_pivots: &mut HashMap<String, IVec2>,
) -> Option<IVec2> {
    if let Some(resolved) = resolved_pivots.get(part.id.as_str()) {
        return Some(*resolved);
    }
    let pose = poses.get(part.id.as_str())?;
    let resolved_pivot = if part.parent_id.is_some() {
        let parent_pivot = resolve_parent_pivot(part, poses, cache, resolved_pivots)?;
        parent_pivot + pose.local_offset
    } else {
        pose.local_offset
    };
    resolved_pivots.insert(part.id.clone(), resolved_pivot);
    Some(resolved_pivot)
}

fn resolve_parent_pivot(
    part: &CachedPart,
    poses: &HashMap<String, CachedPose>,
    cache: &CompositionAtlasCache,
    resolved_pivots: &mut HashMap<String, IVec2>,
) -> Option<IVec2> {
    let mut parent_id = part.parent_id.as_deref();
    while let Some(current_parent_id) = parent_id {
        let parent = cache.parts_by_id.get(current_parent_id)?;
        if parent.is_visual {
            if poses.contains_key(current_parent_id) {
                return resolve_pivot(parent, poses, cache, resolved_pivots);
            }
            return None;
        }
        parent_id = parent.parent_id.as_deref();
    }

    Some(IVec2::ZERO)
}

fn compute_composite_metrics(
    placements: impl IntoIterator<Item = (IVec2, UVec2)>,
) -> Option<CachedCompositeMetrics> {
    let mut min = IVec2::ZERO;
    let mut max = IVec2::ZERO;
    let mut any = false;

    for (bottom_left_offset, size) in placements {
        let part_min = bottom_left_offset;
        let part_max = part_min + size.as_ivec2();

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

fn build_collision_state(
    cache: &CompositionAtlasCache,
    resolved_parts: &HashMap<String, ResolvedPartTransform>,
    part_states: &ComposedPartStates,
    root_position: Vec2,
) -> Vec<ResolvedPartCollision> {
    let mut collisions = Vec::new();

    // Preserve visual draw order here so point collision resolution can prefer
    // the front-most semantic part by scanning the vector in reverse.
    for part_id in &cache.visual_parts_in_draw_order {
        let Some(part) = cache.parts_by_id.get(part_id.as_str()) else {
            continue;
        };
        let Some(transform) = resolved_parts.get(part_id.as_str()) else {
            continue;
        };
        if !part.gameplay.targetable {
            continue;
        }
        if part_states
            .part(part.id.as_str())
            .is_some_and(|state| state.broken)
        {
            continue;
        }
        for collision in &part.gameplay.collisions {
            collisions.push(ResolvedPartCollision {
                part_id: part.id.clone(),
                collider: Collider::new(collision.shape)
                    .with_offset(Vec2::new(collision.offset.x, -collision.offset.y)),
                pivot_position: world_point_from_authored(root_position, transform.pivot),
            });
        }
    }

    collisions
}

fn build_resolved_part_states(
    cache: &CompositionAtlasCache,
    poses: &HashMap<String, CachedPose>,
    resolved_parts: &HashMap<String, ResolvedPartTransform>,
    part_states: &ComposedPartStates,
    root_position: Vec2,
) -> Vec<ResolvedPartState> {
    let mut states = Vec::new();

    for part_id in &cache.visual_parts_in_draw_order {
        let Some(part) = cache.parts_by_id.get(part_id.as_str()) else {
            continue;
        };
        let Some(transform) = resolved_parts.get(part_id.as_str()) else {
            continue;
        };
        let Some(pose) = poses.get(part_id.as_str()) else {
            continue;
        };
        let part_state = part_states.part(part.id.as_str());
        let collisions = part
            .gameplay
            .collisions
            .iter()
            .map(|collision| ResolvedCollisionVolume {
                shape: collision.shape,
                offset: Vec2::new(collision.offset.x, -collision.offset.y),
            })
            .collect();

        states.push(ResolvedPartState {
            part_id: part.id.clone(),
            parent_id: part.parent_id.clone(),
            draw_order: part.draw_order,
            sprite_id: pose.sprite_id.clone(),
            frame_size: pose.size,
            flip_x: pose.flip_x,
            flip_y: pose.flip_y,
            world_top_left_position: world_point_from_authored(root_position, transform.top_left),
            world_pivot_position: world_point_from_authored(root_position, transform.pivot),
            tags: part.tags.clone(),
            targetable: part.gameplay.targetable && !part_state.is_some_and(|state| state.broken),
            health_pool: part.gameplay.health_pool.clone(),
            armour: part.gameplay.armour,
            current_durability: part
                .gameplay
                .durability
                .and(part_state.map(|state| state.current_durability)),
            max_durability: part
                .gameplay
                .durability
                .and(part_state.map(|state| state.max_durability)),
            breakable: part_state.is_some_and(|state| state.breakable),
            broken: part_state.is_some_and(|state| state.broken),
            blinking: part_state
                .and_then(|state| state.hit_blink.as_ref())
                .is_some_and(|state| state.showing_invert),
            collisions,
        });
    }

    states
}

fn advance_part_hit_blinks(part_states: &mut ComposedPartStates, now_ms: u64) {
    let phase_duration_ms = COMPOSED_PART_HIT_BLINK_PHASE.as_millis() as u64;

    for state in part_states.parts.values_mut() {
        let mut clear_blink = false;

        {
            let Some(blink) = state.hit_blink.as_mut() else {
                continue;
            };

            loop {
                if now_ms < blink.phase_started_at_ms + phase_duration_ms {
                    break;
                }

                blink.phase_started_at_ms += phase_duration_ms;
                if blink.showing_invert {
                    blink.showing_invert = false;
                    continue;
                }

                if blink.remaining_invert_cycles == 0 {
                    clear_blink = true;
                    break;
                }

                blink.remaining_invert_cycles -= 1;
                blink.showing_invert = true;
            }
        }

        if clear_blink {
            state.hit_blink = None;
        }
    }
}

fn world_point_from_authored(root_position: Vec2, point: IVec2) -> Vec2 {
    root_position + Vec2::new(point.x as f32, -(point.y as f32))
}

fn advance_track_playback(
    track_state: &mut ComposedTrackPlaybackState,
    request: &RequestedAnimationTrack,
    animation: &CachedAnimation,
    now_ms: u64,
) -> Vec<FiredAnimationCue> {
    let mut fired_cues = Vec::new();
    if animation.frames.is_empty() {
        return fired_cues;
    }

    loop {
        let frame_duration =
            u64::from(animation.frames[track_state.frame_index].duration_ms.max(1));
        if now_ms.saturating_sub(track_state.frame_started_at_ms) < frame_duration {
            break;
        }

        let Some(next_frame_index) = advance_track_to_next_frame(track_state, animation) else {
            track_state.frame_started_at_ms = now_ms;
            break;
        };
        track_state.frame_started_at_ms = track_state
            .frame_started_at_ms
            .saturating_add(frame_duration);
        fired_cues.extend(fired_frame_cues(
            request.tag.as_str(),
            animation,
            next_frame_index,
        ));
    }

    fired_cues
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

fn terminal_frame_index(animation: &CachedAnimation) -> usize {
    if animation.frames.is_empty() {
        return 0;
    }

    match animation.direction {
        CachedAnimationDirection::Forward | CachedAnimationDirection::PingPong => {
            animation.frames.len() - 1
        }
        CachedAnimationDirection::Reverse | CachedAnimationDirection::PingPongReverse => 0,
    }
}

fn can_restart_playback(
    track_state: &ComposedTrackPlaybackState,
    animation: &CachedAnimation,
) -> bool {
    match animation.repeats {
        None | Some(0) => true,
        Some(total_plays) => track_state.completed_loops + 1 < total_plays,
    }
}

/// Advances one playback step and reports the newly entered frame.
///
/// Events are tied to frame entry, so callers should emit cues for the
/// returned frame index exactly once. `None` means playback has reached its
/// finite terminal frame and should remain clamped there.
fn advance_track_to_next_frame(
    track_state: &mut ComposedTrackPlaybackState,
    animation: &CachedAnimation,
) -> Option<usize> {
    if animation.frames.is_empty() {
        return None;
    }

    if animation.frames.len() == 1 {
        if can_restart_playback(track_state, animation) {
            track_state.completed_loops = track_state.completed_loops.saturating_add(1);
            return Some(0);
        }
        return None;
    }

    let next = next_frame_index(
        animation.direction,
        track_state.frame_index,
        animation.frames.len(),
        &mut track_state.ping_pong_forward,
    );

    let wrapped = match animation.direction {
        CachedAnimationDirection::Forward | CachedAnimationDirection::Reverse => {
            track_state.frame_index == terminal_frame_index(animation)
                && next == initial_frame_index(animation)
        }
        CachedAnimationDirection::PingPong | CachedAnimationDirection::PingPongReverse => false,
    };

    if wrapped && !can_restart_playback(track_state, animation) {
        return None;
    }

    if wrapped {
        track_state.completed_loops = track_state.completed_loops.saturating_add(1);
    }

    track_state.frame_index = next;
    Some(next)
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

fn fired_frame_cues(
    tag: &str,
    animation: &CachedAnimation,
    frame_index: usize,
) -> Vec<FiredAnimationCue> {
    let Some(frame) = animation.frames.get(frame_index) else {
        return Vec::new();
    };

    frame
        .events
        .iter()
        .map(|event| FiredAnimationCue {
            tag: tag.to_string(),
            frame_index,
            source_frame: frame.source_frame,
            kind: event.kind,
            id: event.id.clone(),
            part_id: event.part_id.clone(),
            local_offset: event.local_offset,
        })
        .collect()
}

fn hide_composite(
    collision_state: &mut ComposedCollisionState,
    resolved_part_states: &mut ComposedResolvedParts,
    composite: &mut PxCompositeSprite,
    visibility: &mut Visibility,
) {
    collision_state.clear();
    resolved_part_states.clear();
    composite.parts.clear();
    composite.origin = IVec2::ZERO;
    composite.size = UVec2::ZERO;
    composite.frame_count = 0;
    *visibility = Visibility::Hidden;
}

fn fail_ready_composed_enemy(
    commands: &mut Commands,
    entity: Entity,
    collision_state: &mut ComposedCollisionState,
    resolved_part_states: &mut ComposedResolvedParts,
    composite: &mut PxCompositeSprite,
    visibility: &mut Visibility,
    reason: &str,
) {
    error!(
        "Composed enemy {:?} failed after becoming ready: {}",
        entity, reason
    );
    hide_composite(collision_state, resolved_part_states, composite, visibility);
    commands
        .entity(entity)
        .remove::<ComposedEnemyVisualReady>()
        .insert(ComposedEnemyVisualFailed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use asset_pipeline::aseprite::{
        Animation, AnimationEvent, AnimationEventKind, AnimationFrame, AtlasSprite, CollisionRole,
        CollisionShape, CollisionVolume, CompositionGameplay, HealthPool, PartDefinition,
        PartGameplayMetadata, PartInstance, PartPose, Point, Rect, Size, Vec2Value,
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
            part_definitions: vec![PartDefinition {
                id: "body".to_string(),
                tags: vec!["core".to_string()],
                gameplay: PartGameplayMetadata {
                    targetable: Some(true),
                    health_pool: Some("core".to_string()),
                    collision: vec![CollisionVolume {
                        id: "body".to_string(),
                        role: CollisionRole::Collider,
                        shape: CollisionShape::Circle {
                            radius: 2.0,
                            offset: Vec2Value::default(),
                        },
                        tags: vec![],
                    }],
                    ..Default::default()
                },
            }],
            parts: vec![PartInstance {
                id: "body".to_string(),
                definition_id: "body".to_string(),
                name: "Body".to_string(),
                parent_id: None,
                source_layer: Some("body".to_string()),
                draw_order: 0,
                pivot: Point::default(),
                tags: vec![],
                visible_by_default: true,
                gameplay: PartGameplayMetadata::default(),
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
                    events: vec![],
                    parts: vec![PartPose {
                        part_id: "body".to_string(),
                        sprite_id: "sprite_0000".to_string(),
                        local_offset: Point { x: 0, y: 0 },
                        flip_x: false,
                        flip_y: false,
                        visible: true,
                        opacity: 255,
                    }],
                }],
            }],
            gameplay: CompositionGameplay {
                entity_health_pool: Some("core".to_string()),
                health_pools: vec![HealthPool {
                    id: "core".to_string(),
                    max_health: 10,
                }],
            },
        }
    }

    fn visible_part_ids_in_frame(
        cache: &CompositionAtlasCache,
        frame: &CachedAnimationFrame,
    ) -> Vec<String> {
        cache
            .visual_parts_in_draw_order
            .iter()
            .filter(|part_id| {
                frame
                    .poses
                    .get(part_id.as_str())
                    .is_some_and(|pose| pose.visible)
            })
            .cloned()
            .collect()
    }

    fn minimal_mixed_animation_atlas() -> CompositionAtlas {
        CompositionAtlas {
            schema_version: SUPPORTED_COMPOSITION_SCHEMA_VERSION,
            entity: "mixed".to_string(),
            depth: 3,
            source: "mixed.aseprite".to_string(),
            canvas: Size { w: 16, h: 16 },
            origin: Point { x: 8, y: 8 },
            atlas_image: "atlas.png".to_string(),
            part_definitions: vec![
                PartDefinition {
                    id: "root".to_string(),
                    tags: vec!["root".to_string()],
                    gameplay: PartGameplayMetadata::default(),
                },
                PartDefinition {
                    id: "body".to_string(),
                    tags: vec!["body".to_string()],
                    gameplay: PartGameplayMetadata::default(),
                },
                PartDefinition {
                    id: "wings".to_string(),
                    tags: vec!["wings".to_string(), "wing".to_string()],
                    gameplay: PartGameplayMetadata::default(),
                },
            ],
            parts: vec![
                PartInstance {
                    id: "root".to_string(),
                    definition_id: "root".to_string(),
                    name: "Root".to_string(),
                    parent_id: None,
                    source_layer: None,
                    draw_order: 99,
                    pivot: Point::default(),
                    tags: vec![],
                    visible_by_default: true,
                    gameplay: PartGameplayMetadata::default(),
                },
                PartInstance {
                    id: "body".to_string(),
                    definition_id: "body".to_string(),
                    name: "Body".to_string(),
                    parent_id: Some("root".to_string()),
                    source_layer: Some("body".to_string()),
                    draw_order: 1,
                    pivot: Point::default(),
                    tags: vec![],
                    visible_by_default: true,
                    gameplay: PartGameplayMetadata::default(),
                },
                PartInstance {
                    id: "wings_visual".to_string(),
                    definition_id: "wings".to_string(),
                    name: "Wings".to_string(),
                    parent_id: Some("root".to_string()),
                    source_layer: Some("wings_visual".to_string()),
                    draw_order: 0,
                    pivot: Point::default(),
                    tags: vec!["visual_only".to_string()],
                    visible_by_default: true,
                    gameplay: PartGameplayMetadata::default(),
                },
            ],
            sprites: vec![
                AtlasSprite {
                    id: "body_idle".to_string(),
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 4,
                        h: 4,
                    },
                },
                AtlasSprite {
                    id: "wings_flap_a".to_string(),
                    rect: Rect {
                        x: 4,
                        y: 0,
                        w: 4,
                        h: 4,
                    },
                },
                AtlasSprite {
                    id: "wings_flap_b".to_string(),
                    rect: Rect {
                        x: 8,
                        y: 0,
                        w: 4,
                        h: 4,
                    },
                },
                AtlasSprite {
                    id: "body_shoot".to_string(),
                    rect: Rect {
                        x: 12,
                        y: 0,
                        w: 4,
                        h: 4,
                    },
                },
                AtlasSprite {
                    id: "body_melee".to_string(),
                    rect: Rect {
                        x: 16,
                        y: 0,
                        w: 4,
                        h: 4,
                    },
                },
            ],
            animations: vec![
                Animation {
                    tag: "idle_fly".to_string(),
                    direction: "forward".to_string(),
                    repeats: None,
                    frames: vec![
                        AnimationFrame {
                            source_frame: 0,
                            duration_ms: 100,
                            events: vec![],
                            parts: vec![
                                PartPose {
                                    part_id: "body".to_string(),
                                    sprite_id: "body_idle".to_string(),
                                    local_offset: Point::default(),
                                    flip_x: false,
                                    flip_y: false,
                                    visible: true,
                                    opacity: 255,
                                },
                                PartPose {
                                    part_id: "wings_visual".to_string(),
                                    sprite_id: "wings_flap_a".to_string(),
                                    local_offset: Point::default(),
                                    flip_x: false,
                                    flip_y: false,
                                    visible: true,
                                    opacity: 255,
                                },
                            ],
                        },
                        AnimationFrame {
                            source_frame: 1,
                            duration_ms: 100,
                            events: vec![],
                            parts: vec![
                                PartPose {
                                    part_id: "body".to_string(),
                                    sprite_id: "body_idle".to_string(),
                                    local_offset: Point::default(),
                                    flip_x: false,
                                    flip_y: false,
                                    visible: true,
                                    opacity: 255,
                                },
                                PartPose {
                                    part_id: "wings_visual".to_string(),
                                    sprite_id: "wings_flap_b".to_string(),
                                    local_offset: Point::default(),
                                    flip_x: false,
                                    flip_y: false,
                                    visible: true,
                                    opacity: 255,
                                },
                            ],
                        },
                    ],
                },
                Animation {
                    tag: "shoot_fly".to_string(),
                    direction: "forward".to_string(),
                    repeats: None,
                    frames: vec![
                        AnimationFrame {
                            source_frame: 2,
                            duration_ms: 100,
                            events: vec![],
                            parts: vec![PartPose {
                                part_id: "body".to_string(),
                                sprite_id: "body_shoot".to_string(),
                                local_offset: Point::default(),
                                flip_x: false,
                                flip_y: false,
                                visible: true,
                                opacity: 255,
                            }],
                        },
                        AnimationFrame {
                            source_frame: 3,
                            duration_ms: 100,
                            events: vec![AnimationEvent {
                                kind: AnimationEventKind::ProjectileSpawn,
                                id: "blood_shot".to_string(),
                                part_id: Some("body".to_string()),
                                local_offset: Point { x: 2, y: 1 },
                            }],
                            parts: vec![PartPose {
                                part_id: "body".to_string(),
                                sprite_id: "body_shoot".to_string(),
                                local_offset: Point::default(),
                                flip_x: false,
                                flip_y: false,
                                visible: true,
                                opacity: 255,
                            }],
                        },
                    ],
                },
                Animation {
                    tag: "melee_fly".to_string(),
                    direction: "forward".to_string(),
                    repeats: None,
                    frames: vec![AnimationFrame {
                        source_frame: 3,
                        duration_ms: 180,
                        events: vec![],
                        parts: vec![PartPose {
                            part_id: "body".to_string(),
                            sprite_id: "body_melee".to_string(),
                            local_offset: Point::default(),
                            flip_x: false,
                            flip_y: false,
                            visible: true,
                            opacity: 255,
                        }],
                    }],
                },
            ],
            gameplay: CompositionGameplay::default(),
        }
    }

    fn cue_ids_at_times(
        visual: &mut ComposedEnemyVisual,
        state: &ComposedAnimationState,
        cache: &CompositionAtlasCache,
        times_ms: &[u64],
    ) -> Vec<Vec<String>> {
        let tracks = requested_animation_tracks(state);
        times_ms
            .iter()
            .map(|now_ms| {
                resolve_requested_animation_frame(visual, &tracks, cache, *now_ms)
                    .expect("frame resolution should succeed")
                    .fired_cues
                    .into_iter()
                    .map(|cue| cue.id)
                    .collect()
            })
            .collect()
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
        assert!(
            tags.contains("idle_fly"),
            "expected idle_fly tag in exported mosquiton atlas"
        );
        assert!(
            tags.contains("shoot_fly"),
            "expected shoot_fly tag in exported mosquiton atlas"
        );
        assert!(
            tags.contains("melee_fly"),
            "expected melee_fly tag in exported mosquiton atlas"
        );
    }

    #[test]
    fn exported_mosquiton_shoot_fly_authors_blood_shot_cue_on_mouth_open_frame() {
        let atlas = load_exported_mosquiton();
        let shoot = atlas
            .atlas
            .animations
            .iter()
            .find(|animation| animation.tag == "shoot_fly")
            .expect("shoot_fly animation should exist");

        let authored_frames: Vec<_> = shoot
            .frames
            .iter()
            .enumerate()
            .filter_map(|(frame_index, frame)| {
                frame
                    .events
                    .iter()
                    .find(|event| {
                        event.kind == AnimationEventKind::ProjectileSpawn
                            && event.id == "blood_shot"
                            && event.part_id.as_deref() == Some("head")
                            && event.local_offset == Point { x: 6, y: 9 }
                    })
                    .map(|_| frame_index)
            })
            .collect();

        assert_eq!(authored_frames, vec![5]);
    }

    #[test]
    fn monolithic_animation_resolution_remains_unchanged_without_overrides() {
        let atlas = minimal_mixed_animation_atlas();
        let cache = build_runtime_cache(&atlas).expect("mixed atlas should validate");
        let mut visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let state = ComposedAnimationState::new("shoot_fly");

        let resolved = resolve_requested_animation_frame(
            &mut visual,
            &requested_animation_tracks(&state),
            &cache,
            0,
        )
        .expect("single-source animation should still resolve");

        assert_eq!(resolved.poses["body"].sprite_id, "body_shoot");
        assert!(!resolved.poses.contains_key("wings_visual"));
    }

    #[test]
    fn exported_mosquiton_shoot_fly_uses_action_body_and_flapping_wings() {
        let atlas = load_exported_mosquiton();
        let cache = build_runtime_cache(&atlas.atlas).expect("mosquiton atlas should validate");
        let mut visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let mut state = ComposedAnimationState::new("shoot_fly");
        state.set_part_overrides([ComposedAnimationOverride::for_part_tags(
            "idle_fly",
            ["wings"],
        )]);
        let tracks = requested_animation_tracks(&state);
        let _ = resolve_requested_animation_frame(&mut visual, &tracks, &cache, 0)
            .expect("initial shoot frame should resolve");

        let resolved = resolve_requested_animation_frame(&mut visual, &tracks, &cache, 200)
            .expect("mosquiton shoot fly should resolve");

        let mut idle_visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let idle_tracks = requested_animation_tracks(&ComposedAnimationState::new("idle_fly"));
        let _ = resolve_requested_animation_frame(&mut idle_visual, &idle_tracks, &cache, 0)
            .expect("initial idle frame should resolve");
        let idle_resolved =
            resolve_requested_animation_frame(&mut idle_visual, &idle_tracks, &cache, 200)
                .expect("idle fly should resolve");

        let mut shoot_visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let shoot_tracks = requested_animation_tracks(&ComposedAnimationState::new("shoot_fly"));
        let _ = resolve_requested_animation_frame(&mut shoot_visual, &shoot_tracks, &cache, 0)
            .expect("initial shoot frame should resolve");
        let shoot_resolved =
            resolve_requested_animation_frame(&mut shoot_visual, &shoot_tracks, &cache, 200)
                .expect("shoot fly should resolve");

        assert_eq!(
            resolved.poses["wings_visual"].sprite_id,
            idle_resolved.poses["wings_visual"].sprite_id
        );
        assert_eq!(
            resolved.poses["body"].sprite_id,
            shoot_resolved.poses["body"].sprite_id
        );
    }

    #[test]
    fn exported_mosquiton_melee_fly_uses_action_body_and_flapping_wings() {
        let atlas = load_exported_mosquiton();
        let cache = build_runtime_cache(&atlas.atlas).expect("mosquiton atlas should validate");
        let mut visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let mut state = ComposedAnimationState::new("melee_fly");
        state.set_part_overrides([ComposedAnimationOverride::for_part_tags(
            "idle_fly",
            ["wings"],
        )]);
        let tracks = requested_animation_tracks(&state);
        let _ = resolve_requested_animation_frame(&mut visual, &tracks, &cache, 0)
            .expect("initial melee frame should resolve");

        let resolved = resolve_requested_animation_frame(&mut visual, &tracks, &cache, 200)
            .expect("mosquiton melee fly should resolve");

        let mut idle_visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let idle_tracks = requested_animation_tracks(&ComposedAnimationState::new("idle_fly"));
        let _ = resolve_requested_animation_frame(&mut idle_visual, &idle_tracks, &cache, 0)
            .expect("initial idle frame should resolve");
        let idle_resolved =
            resolve_requested_animation_frame(&mut idle_visual, &idle_tracks, &cache, 200)
                .expect("idle fly should resolve");

        let mut melee_visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let melee_tracks = requested_animation_tracks(&ComposedAnimationState::new("melee_fly"));
        let _ = resolve_requested_animation_frame(&mut melee_visual, &melee_tracks, &cache, 0)
            .expect("initial melee frame should resolve");
        let melee_resolved =
            resolve_requested_animation_frame(&mut melee_visual, &melee_tracks, &cache, 200)
                .expect("melee fly should resolve");

        assert_eq!(
            resolved.poses["wings_visual"].sprite_id,
            idle_resolved.poses["wings_visual"].sprite_id
        );
        assert_eq!(
            resolved.poses["body"].sprite_id,
            melee_resolved.poses["body"].sprite_id
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
    fn mixed_animation_resolution_uses_wing_override_and_action_body() {
        let atlas = minimal_mixed_animation_atlas();
        let cache = build_runtime_cache(&atlas).expect("mixed atlas should validate");
        let mut visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let mut state = ComposedAnimationState::new("shoot_fly");
        state.set_part_overrides([ComposedAnimationOverride::for_part_tags(
            "idle_fly",
            ["wing"],
        )]);

        let resolved = resolve_requested_animation_frame(
            &mut visual,
            &requested_animation_tracks(&state),
            &cache,
            0,
        )
        .expect("mixed resolution should succeed");

        assert_eq!(resolved.poses["body"].sprite_id, "body_shoot");
        assert_eq!(resolved.poses["wings_visual"].sprite_id, "wings_flap_a");
    }

    #[test]
    fn mixed_animation_progression_keeps_wings_independent() {
        let atlas = minimal_mixed_animation_atlas();
        let cache = build_runtime_cache(&atlas).expect("mixed atlas should validate");
        let mut visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let mut state = ComposedAnimationState::new("shoot_fly");
        state.set_part_overrides([ComposedAnimationOverride::for_part_tags(
            "idle_fly",
            ["wing"],
        )]);
        let tracks = requested_animation_tracks(&state);

        let first =
            resolve_requested_animation_frame(&mut visual, &tracks, &cache, 0).expect("frame 0");
        let second =
            resolve_requested_animation_frame(&mut visual, &tracks, &cache, 100).expect("frame 1");
        let third =
            resolve_requested_animation_frame(&mut visual, &tracks, &cache, 200).expect("frame 2");

        assert_eq!(first.poses["body"].sprite_id, "body_shoot");
        assert_eq!(second.poses["body"].sprite_id, "body_shoot");
        assert_eq!(third.poses["body"].sprite_id, "body_shoot");

        assert_eq!(first.poses["wings_visual"].sprite_id, "wings_flap_a");
        assert_eq!(second.poses["wings_visual"].sprite_id, "wings_flap_b");
        assert_eq!(third.poses["wings_visual"].sprite_id, "wings_flap_a");
    }

    #[test]
    fn missing_override_poses_fall_back_to_base_animation() {
        let atlas = minimal_mixed_animation_atlas();
        let cache = build_runtime_cache(&atlas).expect("mixed atlas should validate");
        let mut visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let mut state = ComposedAnimationState::new("idle_fly");
        state.set_part_overrides([ComposedAnimationOverride::for_part_tags(
            "shoot_fly",
            ["wing"],
        )]);

        let resolved = resolve_requested_animation_frame(
            &mut visual,
            &requested_animation_tracks(&state),
            &cache,
            0,
        )
        .expect("fallback to base should succeed");

        assert_eq!(resolved.poses["body"].sprite_id, "body_idle");
        assert_eq!(resolved.poses["wings_visual"].sprite_id, "wings_flap_a");
    }

    #[test]
    fn missing_override_tag_fails_explicitly() {
        let atlas = minimal_mixed_animation_atlas();
        let cache = build_runtime_cache(&atlas).expect("mixed atlas should validate");
        let mut visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let mut state = ComposedAnimationState::new("shoot_fly");
        state.set_part_overrides([ComposedAnimationOverride::for_part_tags(
            "missing_fly",
            ["wing"],
        )]);

        let error = resolve_requested_animation_frame(
            &mut visual,
            &requested_animation_tracks(&state),
            &cache,
            0,
        )
        .expect_err("missing override tags should fail loudly");

        assert!(error.contains("missing_fly"));
    }

    #[test]
    fn animation_events_fire_when_the_authored_frame_is_entered() {
        let atlas = minimal_mixed_animation_atlas();
        let cache = build_runtime_cache(&atlas).expect("mixed atlas should validate");
        let mut visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let state = ComposedAnimationState::new("shoot_fly");

        let cues = cue_ids_at_times(&mut visual, &state, &cache, &[0, 99, 100]);
        assert!(
            cues[0].is_empty(),
            "event should not fire at animation start"
        );
        assert!(
            cues[1].is_empty(),
            "event should not fire before the authored frame"
        );
        assert_eq!(cues[2], vec!["blood_shot".to_string()]);
    }

    #[test]
    fn animation_events_refire_on_the_next_loop_only() {
        let atlas = minimal_mixed_animation_atlas();
        let cache = build_runtime_cache(&atlas).expect("mixed atlas should validate");
        let mut visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let state = ComposedAnimationState::new("shoot_fly");

        let cues = cue_ids_at_times(&mut visual, &state, &cache, &[0, 100, 150, 200, 300]);
        assert_eq!(cues[1], vec!["blood_shot".to_string()]);
        assert!(cues[2].is_empty());
        assert!(
            cues[3].is_empty(),
            "loop restart should not refire until the authored frame is entered again"
        );
        assert_eq!(cues[4], vec!["blood_shot".to_string()]);
    }

    #[test]
    fn animation_events_survive_multi_frame_advances() {
        let atlas = minimal_mixed_animation_atlas();
        let cache = build_runtime_cache(&atlas).expect("mixed atlas should validate");
        let mut visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let state = ComposedAnimationState::new("shoot_fly");

        let cues = cue_ids_at_times(&mut visual, &state, &cache, &[0, 250]);
        assert_eq!(cues[1], vec!["blood_shot".to_string()]);
    }

    #[test]
    fn finite_repeat_animations_do_not_refire_after_completion() {
        let mut atlas = minimal_mixed_animation_atlas();
        let shoot = atlas
            .animations
            .iter_mut()
            .find(|animation| animation.tag == "shoot_fly")
            .expect("shoot_fly animation should exist");
        shoot.repeats = Some(1);

        let cache = build_runtime_cache(&atlas).expect("mixed atlas should validate");
        let mut visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let state = ComposedAnimationState::new("shoot_fly");

        let cues = cue_ids_at_times(&mut visual, &state, &cache, &[0, 100, 300, 600]);
        assert_eq!(cues[1], vec!["blood_shot".to_string()]);
        assert!(cues[2].is_empty());
        assert!(
            cues[3].is_empty(),
            "non-looping clips should not refire after completion"
        );
    }

    #[test]
    fn animations_without_authored_events_remain_silent() {
        let atlas = minimal_mixed_animation_atlas();
        let cache = build_runtime_cache(&atlas).expect("mixed atlas should validate");
        let mut visual = ComposedEnemyVisual {
            atlas_manifest: Handle::default(),
            sprite_atlas: Handle::default(),
            track_states: Vec::new(),
            last_error: None,
        };
        let state = ComposedAnimationState::new("idle_fly");

        let cues = cue_ids_at_times(&mut visual, &state, &cache, &[0, 100, 200, 300]);
        assert!(cues.into_iter().all(|events| events.is_empty()));
    }

    #[test]
    fn cue_offsets_follow_part_flips_in_world_space() {
        let part = ResolvedPartState {
            part_id: "head".to_string(),
            parent_id: None,
            draw_order: 0,
            sprite_id: "head".to_string(),
            frame_size: UVec2::new(13, 16),
            flip_x: true,
            flip_y: false,
            world_top_left_position: Vec2::ZERO,
            world_pivot_position: Vec2::new(10.0, 20.0),
            tags: vec![],
            targetable: false,
            health_pool: None,
            armour: 0,
            current_durability: None,
            max_durability: None,
            breakable: false,
            broken: false,
            blinking: false,
            collisions: vec![],
        };

        let point = part.world_point_from_local_offset(IVec2::new(2, 10));
        assert_eq!(point, Vec2::new(20.0, 10.0));
    }

    #[test]
    fn authored_part_positions_flip_y_into_world_space() {
        let root_position = Vec2::new(100.0, 80.0);

        assert_eq!(
            world_point_from_authored(root_position, IVec2::new(5, 7)),
            Vec2::new(105.0, 73.0)
        );
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

    #[test]
    fn composes_child_offsets_hierarchically() {
        let mut atlas = minimal_atlas();
        atlas.part_definitions.push(PartDefinition {
            id: "head".to_string(),
            tags: vec!["head".to_string()],
            gameplay: PartGameplayMetadata::default(),
        });
        atlas.parts.push(PartInstance {
            id: "head".to_string(),
            definition_id: "head".to_string(),
            name: "Head".to_string(),
            parent_id: Some("body".to_string()),
            source_layer: Some("head".to_string()),
            draw_order: 1,
            pivot: Point::default(),
            tags: vec![],
            visible_by_default: true,
            gameplay: PartGameplayMetadata::default(),
        });
        atlas.sprites.push(AtlasSprite {
            id: "sprite_0001".to_string(),
            rect: Rect {
                x: 4,
                y: 0,
                w: 2,
                h: 2,
            },
        });
        atlas.animations[0].frames[0].parts.push(PartPose {
            part_id: "head".to_string(),
            sprite_id: "sprite_0001".to_string(),
            local_offset: Point { x: 3, y: -2 },
            flip_x: false,
            flip_y: false,
            visible: true,
            opacity: 255,
        });

        let cache = build_runtime_cache(&atlas).expect("hierarchical atlas should validate");
        let part_states = ComposedPartStates::from_cache(&cache);
        let bindings = ComposedAtlasBindings {
            atlas: Handle::default(),
            sprite_regions: HashMap::from([
                ("sprite_0000".to_string(), AtlasRegionId(0)),
                ("sprite_0001".to_string(), AtlasRegionId(1)),
            ]),
            sprite_rects: HashMap::from([
                (
                    "sprite_0000".to_string(),
                    AtlasRect {
                        x: 0,
                        y: 0,
                        w: 4,
                        h: 4,
                    },
                ),
                (
                    "sprite_0001".to_string(),
                    AtlasRect {
                        x: 4,
                        y: 0,
                        w: 2,
                        h: 2,
                    },
                ),
            ]),
        };
        let frame = &cache.animations["idle_stand"].frames[0];
        let (_parts, metrics, resolved_parts) = compose_frame(
            &frame.poses,
            &cache,
            &bindings,
            &part_states,
            &Handle::default(),
            Entity::from_bits(1),
        )
        .expect("frame should compose");

        assert_eq!(metrics.origin, IVec2::new(0, -4));
        assert_eq!(metrics.size, UVec2::new(5, 6));
        assert_eq!(
            resolved_parts.get("head").expect("head transform").top_left,
            IVec2::new(3, -2)
        );
    }

    #[test]
    fn merged_tags_and_damage_routing_remain_semantic() {
        let atlas = load_exported_mosquiton();
        let cache = build_runtime_cache(&atlas.atlas).expect("mosquiton atlas should validate");

        let arm_r = cache.parts_by_id.get("arm_r").expect("arm_r should exist");
        assert!(arm_r.tags.contains(&"arm".to_string()));
        assert!(arm_r.tags.contains(&"right".to_string()));
        assert_eq!(arm_r.gameplay.health_pool.as_deref(), Some("core"));
        assert_eq!(arm_r.gameplay.armour, 1);
        assert_eq!(arm_r.gameplay.durability, Some(2));

        let head = cache.parts_by_id.get("head").expect("head should exist");
        assert_eq!(head.gameplay.armour, 0);
        assert_eq!(head.gameplay.durability, None);

        let wing_l = cache
            .parts_by_id
            .get("wing_l")
            .expect("wing_l should exist");
        assert!(wing_l.tags.contains(&"wing".to_string()));
        assert!(wing_l.tags.contains(&"left".to_string()));
        assert_eq!(wing_l.gameplay.health_pool, None);

        let wings_visual = cache
            .parts_by_id
            .get("wings_visual")
            .expect("wings_visual should exist");
        assert!(wings_visual.tags.contains(&"targetable".to_string()));
        assert_eq!(wings_visual.gameplay.health_pool.as_deref(), Some("wings"));
        assert_eq!(wings_visual.gameplay.armour, 6);
        assert_eq!(wings_visual.gameplay.durability, Some(2));

        let legs_visual = cache
            .parts_by_id
            .get("legs_visual")
            .expect("legs_visual should exist");
        assert!(legs_visual.tags.contains(&"targetable".to_string()));
        assert_eq!(legs_visual.gameplay.health_pool.as_deref(), Some("core"));
        assert_eq!(legs_visual.gameplay.armour, 1);
        assert_eq!(legs_visual.gameplay.durability, Some(2));

        let arms_overlay = cache
            .parts_by_id
            .get("arms_overlay")
            .expect("arms_overlay should exist");
        assert!(arms_overlay.tags.contains(&"targetable".to_string()));
        assert_eq!(arms_overlay.gameplay.health_pool.as_deref(), Some("core"));
        assert_eq!(arms_overlay.gameplay.armour, 1);
        assert_eq!(arms_overlay.gameplay.durability, Some(2));

        assert_eq!(cache.entity_health_pool.as_deref(), Some("core"));
        assert_eq!(cache.health_pools.get("core"), Some(&40));
    }

    #[test]
    fn visual_draw_order_ignores_non_visual_parts() {
        let mut atlas = minimal_atlas();
        atlas.part_definitions.push(PartDefinition {
            id: "marker".to_string(),
            tags: vec!["marker".to_string()],
            gameplay: PartGameplayMetadata::default(),
        });
        atlas.part_definitions.push(PartDefinition {
            id: "head".to_string(),
            tags: vec!["head".to_string()],
            gameplay: PartGameplayMetadata::default(),
        });
        atlas.parts.push(PartInstance {
            id: "marker".to_string(),
            definition_id: "marker".to_string(),
            name: "Marker".to_string(),
            parent_id: Some("body".to_string()),
            source_layer: None,
            draw_order: 999,
            pivot: Point::default(),
            tags: vec![],
            visible_by_default: true,
            gameplay: PartGameplayMetadata::default(),
        });
        atlas.parts.push(PartInstance {
            id: "head".to_string(),
            definition_id: "head".to_string(),
            name: "Head".to_string(),
            parent_id: Some("body".to_string()),
            source_layer: Some("head".to_string()),
            draw_order: 5,
            pivot: Point::default(),
            tags: vec![],
            visible_by_default: true,
            gameplay: PartGameplayMetadata::default(),
        });
        atlas.sprites.push(AtlasSprite {
            id: "sprite_0001".to_string(),
            rect: Rect {
                x: 4,
                y: 0,
                w: 2,
                h: 2,
            },
        });
        atlas.animations[0].frames[0].parts.push(PartPose {
            part_id: "head".to_string(),
            sprite_id: "sprite_0001".to_string(),
            local_offset: Point { x: 1, y: 1 },
            flip_x: false,
            flip_y: false,
            visible: true,
            opacity: 255,
        });

        let cache = build_runtime_cache(&atlas).expect("atlas should validate");
        assert_eq!(cache.visual_parts_in_draw_order, vec!["body", "head"]);
    }

    #[test]
    fn exported_mosquiton_idle_fly_orders_wings_behind_body() {
        let atlas = load_exported_mosquiton();
        let cache = build_runtime_cache(&atlas.atlas).expect("mosquiton atlas should validate");
        let frame = &cache.animations["idle_fly"].frames[0];
        let visible = visible_part_ids_in_frame(&cache, frame);
        let wings_index = visible
            .iter()
            .position(|part_id| part_id == "wings_visual")
            .expect("wings should be visible");
        let body_index = visible
            .iter()
            .position(|part_id| part_id == "body")
            .expect("body should be visible");
        let head_index = visible
            .iter()
            .position(|part_id| part_id == "head")
            .expect("head should be visible");

        assert!(wings_index < body_index);
        assert!(wings_index < head_index);
    }

    #[test]
    fn mirrored_sprite_reuse_preserves_semantic_parts() {
        let mut atlas = minimal_atlas();
        atlas.part_definitions.push(PartDefinition {
            id: "arm".to_string(),
            tags: vec!["arm".to_string()],
            gameplay: PartGameplayMetadata::default(),
        });
        atlas.parts.push(PartInstance {
            id: "arm_l".to_string(),
            definition_id: "arm".to_string(),
            name: "Arm Left".to_string(),
            parent_id: Some("body".to_string()),
            source_layer: Some("arm_l".to_string()),
            draw_order: 1,
            pivot: Point::default(),
            tags: vec!["left".to_string()],
            visible_by_default: true,
            gameplay: PartGameplayMetadata::default(),
        });
        atlas.parts.push(PartInstance {
            id: "arm_r".to_string(),
            definition_id: "arm".to_string(),
            name: "Arm Right".to_string(),
            parent_id: Some("body".to_string()),
            source_layer: Some("arm_r".to_string()),
            draw_order: 2,
            pivot: Point::default(),
            tags: vec!["right".to_string()],
            visible_by_default: true,
            gameplay: PartGameplayMetadata::default(),
        });
        atlas.animations[0].frames[0].parts.push(PartPose {
            part_id: "arm_l".to_string(),
            sprite_id: "sprite_0000".to_string(),
            local_offset: Point { x: -2, y: 0 },
            flip_x: false,
            flip_y: false,
            visible: true,
            opacity: 255,
        });
        atlas.animations[0].frames[0].parts.push(PartPose {
            part_id: "arm_r".to_string(),
            sprite_id: "sprite_0000".to_string(),
            local_offset: Point { x: 2, y: 0 },
            flip_x: true,
            flip_y: false,
            visible: true,
            opacity: 255,
        });

        let cache = build_runtime_cache(&atlas).expect("atlas should validate");
        let frame = &cache.animations["idle_stand"].frames[0];

        assert_eq!(
            frame.poses["arm_l"].sprite_id, frame.poses["arm_r"].sprite_id,
            "canonical sprite should be reused across semantic arm instances"
        );
        assert!(!frame.poses["arm_l"].flip_x);
        assert!(frame.poses["arm_r"].flip_x);
    }

    #[test]
    fn shared_pool_damage_accumulates_across_semantic_parts() {
        let atlas = load_exported_mosquiton();
        let cache = build_runtime_cache(&atlas.atlas).expect("mosquiton atlas should validate");
        let mut health_pools = ComposedHealthPools::from_cache(&cache);
        let mut part_states = ComposedPartStates::from_cache(&cache);

        let arm_left = apply_part_damage(&cache, &mut health_pools, &mut part_states, "arm_l", 4)
            .expect("left arm should route to the core pool");
        assert_eq!(
            arm_left,
            AppliedPartDamage {
                pool_id: Some("core".to_string()),
                remaining_health: Some(39),
                remaining_durability: Some(0),
                broke_part: false,
            }
        );

        let arm_right = apply_part_damage(&cache, &mut health_pools, &mut part_states, "arm_r", 4)
            .expect("right arm should route to the shared core pool");
        assert_eq!(
            arm_right,
            AppliedPartDamage {
                pool_id: Some("core".to_string()),
                remaining_health: Some(38),
                remaining_durability: Some(0),
                broke_part: false,
            }
        );
    }

    #[test]
    fn collision_state_resolves_points_to_semantic_parts() {
        let atlas = load_exported_mosquiton();
        let cache = build_runtime_cache(&atlas.atlas).expect("mosquiton atlas should validate");
        let bindings = ComposedAtlasBindings {
            atlas: Handle::default(),
            sprite_regions: atlas
                .atlas
                .sprites
                .iter()
                .enumerate()
                .map(|(index, sprite)| (sprite.id.clone(), AtlasRegionId(index as u32)))
                .collect(),
            sprite_rects: atlas
                .atlas
                .sprites
                .iter()
                .map(|sprite| {
                    (
                        sprite.id.clone(),
                        AtlasRect {
                            x: sprite.rect.x,
                            y: sprite.rect.y,
                            w: sprite.rect.w,
                            h: sprite.rect.h,
                        },
                    )
                })
                .collect(),
        };
        let frame = &cache.animations["idle_fly"].frames[0];
        let part_states = ComposedPartStates::from_cache(&cache);
        let (_parts, _metrics, resolved_parts) = compose_frame(
            &frame.poses,
            &cache,
            &bindings,
            &part_states,
            &Handle::default(),
            Entity::from_bits(1),
        )
        .expect("frame should compose");
        let collisions =
            build_collision_state(&cache, &resolved_parts, &part_states, Vec2::new(85.0, 68.0));

        let head = collisions
            .iter()
            .find(|collision| collision.part_id == "head")
            .expect("head collision should exist");
        let body = collisions
            .iter()
            .find(|collision| collision.part_id == "body")
            .expect("body collision should exist");
        let head_point = head.pivot_position + head.collider.offset;
        let body_point = body.pivot_position + body.collider.offset + Vec2::new(8.0, 0.0);
        let collision_state = ComposedCollisionState { collisions };

        assert_eq!(
            collision_state
                .point_collides(head_point)
                .expect("head point should collide")
                .part_id,
            "head"
        );

        assert_eq!(
            collision_state
                .point_collides(body_point)
                .expect("body point should collide")
                .part_id,
            "body"
        );

        assert!(
            collision_state
                .collisions()
                .iter()
                .all(|collision| collision.part_id != "wing_l" && collision.part_id != "wing_r"),
            "non-visual contract-A wing markers must not produce runtime collisions"
        );
        assert!(
            collision_state
                .collisions()
                .iter()
                .any(|collision| collision.part_id == "wings_visual"),
            "rendered wing visuals should now own gameplay collisions"
        );
        assert!(
            collision_state
                .collisions()
                .iter()
                .any(|collision| collision.part_id == "arms_overlay"),
            "idle flying overlay arms should remain targetable when separate arm parts are absent"
        );
    }

    #[test]
    fn overlapping_collisions_resolve_to_front_most_part() {
        let mut atlas = minimal_atlas();
        atlas.part_definitions.push(PartDefinition {
            id: "head".to_string(),
            tags: vec!["head".to_string()],
            gameplay: PartGameplayMetadata {
                targetable: Some(true),
                health_pool: Some("core".to_string()),
                collision: vec![asset_pipeline::aseprite::CollisionVolume {
                    id: "head".to_string(),
                    role: asset_pipeline::aseprite::CollisionRole::Collider,
                    shape: asset_pipeline::aseprite::CollisionShape::Circle {
                        radius: 10.0,
                        offset: asset_pipeline::aseprite::Vec2Value::default(),
                    },
                    tags: vec![],
                }],
                ..Default::default()
            },
        });
        atlas.parts.push(PartInstance {
            id: "head".to_string(),
            definition_id: "head".to_string(),
            name: "Head".to_string(),
            parent_id: Some("body".to_string()),
            source_layer: Some("head".to_string()),
            draw_order: 1,
            pivot: Point::default(),
            tags: vec![],
            visible_by_default: true,
            gameplay: PartGameplayMetadata::default(),
        });
        atlas.sprites.push(AtlasSprite {
            id: "sprite_0001".to_string(),
            rect: Rect {
                x: 4,
                y: 0,
                w: 4,
                h: 4,
            },
        });
        atlas.animations[0].frames[0].parts.push(PartPose {
            part_id: "head".to_string(),
            sprite_id: "sprite_0001".to_string(),
            local_offset: Point::default(),
            flip_x: false,
            flip_y: false,
            visible: true,
            opacity: 255,
        });

        let cache = build_runtime_cache(&atlas).expect("atlas should validate");
        let bindings = ComposedAtlasBindings {
            atlas: Handle::default(),
            sprite_regions: HashMap::from([
                ("sprite_0000".to_string(), AtlasRegionId(0)),
                ("sprite_0001".to_string(), AtlasRegionId(1)),
            ]),
            sprite_rects: HashMap::from([
                (
                    "sprite_0000".to_string(),
                    AtlasRect {
                        x: 0,
                        y: 0,
                        w: 4,
                        h: 4,
                    },
                ),
                (
                    "sprite_0001".to_string(),
                    AtlasRect {
                        x: 4,
                        y: 0,
                        w: 4,
                        h: 4,
                    },
                ),
            ]),
        };
        let frame = &cache.animations["idle_stand"].frames[0];
        let part_states = ComposedPartStates::from_cache(&cache);
        let (_parts, _metrics, resolved_parts) = compose_frame(
            &frame.poses,
            &cache,
            &bindings,
            &part_states,
            &Handle::default(),
            Entity::from_bits(1),
        )
        .expect("frame should compose");
        let collision_state = ComposedCollisionState {
            collisions: build_collision_state(&cache, &resolved_parts, &part_states, Vec2::ZERO),
        };

        assert_eq!(
            collision_state
                .point_collides(Vec2::ZERO)
                .expect("overlapping body and head should collide")
                .part_id,
            "head"
        );
    }

    #[test]
    fn applies_part_damage_through_entity_health_pool() {
        let atlas = load_exported_mosquiton();
        let cache = build_runtime_cache(&atlas.atlas).expect("mosquiton atlas should validate");
        let mut health_pools = ComposedHealthPools::from_cache(&cache);
        let mut part_states = ComposedPartStates::from_cache(&cache);

        let arm_r = apply_part_damage(&cache, &mut health_pools, &mut part_states, "arm_r", 5)
            .expect("arm damage should route to the core pool");
        assert_eq!(
            arm_r,
            AppliedPartDamage {
                pool_id: Some("core".to_string()),
                remaining_health: Some(38),
                remaining_durability: Some(0),
                broke_part: false,
            }
        );

        let head = apply_part_damage(&cache, &mut health_pools, &mut part_states, "head", 6)
            .expect("head damage should route to the core pool");
        assert_eq!(
            head,
            AppliedPartDamage {
                pool_id: Some("core".to_string()),
                remaining_health: Some(32),
                remaining_durability: None,
                broke_part: false,
            }
        );
    }

    #[test]
    fn rejects_damage_on_non_gameplay_semantic_nodes() {
        let atlas = load_exported_mosquiton();
        let cache = build_runtime_cache(&atlas.atlas).expect("mosquiton atlas should validate");
        let mut health_pools = ComposedHealthPools::from_cache(&cache);
        let mut part_states = ComposedPartStates::from_cache(&cache);

        let error = apply_part_damage(&cache, &mut health_pools, &mut part_states, "wing_l", 5)
            .expect_err("non-visual semantic markers should not be targetable");
        assert!(error.contains("not gameplay-targetable"));
    }

    #[test]
    fn compose_frame_applies_invert_filter_only_to_blinking_part() {
        let mut atlas = minimal_atlas();
        atlas.part_definitions.push(PartDefinition {
            id: "head".to_string(),
            tags: vec!["head".to_string()],
            gameplay: PartGameplayMetadata {
                targetable: Some(true),
                health_pool: Some("core".to_string()),
                collision: vec![asset_pipeline::aseprite::CollisionVolume {
                    id: "head".to_string(),
                    role: asset_pipeline::aseprite::CollisionRole::Collider,
                    shape: asset_pipeline::aseprite::CollisionShape::Circle {
                        radius: 4.0,
                        offset: asset_pipeline::aseprite::Vec2Value::default(),
                    },
                    tags: vec![],
                }],
                ..Default::default()
            },
        });
        atlas.parts.push(PartInstance {
            id: "head".to_string(),
            definition_id: "head".to_string(),
            name: "Head".to_string(),
            parent_id: Some("body".to_string()),
            source_layer: Some("head".to_string()),
            draw_order: 1,
            pivot: Point::default(),
            tags: vec![],
            visible_by_default: true,
            gameplay: PartGameplayMetadata::default(),
        });
        atlas.sprites.push(AtlasSprite {
            id: "sprite_0001".to_string(),
            rect: Rect {
                x: 4,
                y: 0,
                w: 4,
                h: 4,
            },
        });
        atlas.animations[0].frames[0].parts.push(PartPose {
            part_id: "head".to_string(),
            sprite_id: "sprite_0001".to_string(),
            local_offset: Point::default(),
            flip_x: false,
            flip_y: false,
            visible: true,
            opacity: 255,
        });

        let cache = build_runtime_cache(&atlas).expect("blink atlas should validate");
        let bindings = ComposedAtlasBindings {
            atlas: Handle::default(),
            sprite_regions: HashMap::from([
                ("sprite_0000".to_string(), AtlasRegionId(0)),
                ("sprite_0001".to_string(), AtlasRegionId(1)),
            ]),
            sprite_rects: HashMap::from([
                (
                    "sprite_0000".to_string(),
                    AtlasRect {
                        x: 0,
                        y: 0,
                        w: 4,
                        h: 4,
                    },
                ),
                (
                    "sprite_0001".to_string(),
                    AtlasRect {
                        x: 4,
                        y: 0,
                        w: 4,
                        h: 4,
                    },
                ),
            ]),
        };
        let mut part_states = ComposedPartStates::from_cache(&cache);
        part_states.part_mut("head").unwrap().hit_blink = Some(PartHitBlinkState {
            phase_started_at_ms: 0,
            showing_invert: true,
            remaining_invert_cycles: COMPOSED_PART_HIT_BLINK_INVERT_CYCLES,
        });

        let (parts, _, _) = compose_frame(
            &cache.animations["idle_stand"].frames[0].poses,
            &cache,
            &bindings,
            &part_states,
            &Handle::default(),
            Entity::from_bits(1),
        )
        .expect("frame should compose");

        assert_eq!(parts.len(), 2);
        assert!(parts[0].filter.is_none(), "body should not blink");
        assert!(parts[1].filter.is_some(), "head should blink");
    }

    #[test]
    fn part_hit_blink_advances_and_clears() {
        let mut part_states = ComposedPartStates {
            parts: HashMap::from([(
                "head".to_string(),
                PartGameplayState {
                    current_durability: 0,
                    max_durability: 0,
                    breakable: false,
                    broken: false,
                    visible: true,
                    hit_blink: Some(PartHitBlinkState {
                        phase_started_at_ms: 0,
                        showing_invert: true,
                        remaining_invert_cycles: 1,
                    }),
                },
            )]),
        };

        advance_part_hit_blinks(&mut part_states, 0);
        assert!(
            part_states
                .part("head")
                .unwrap()
                .hit_blink
                .as_ref()
                .unwrap()
                .showing_invert
        );

        advance_part_hit_blinks(
            &mut part_states,
            COMPOSED_PART_HIT_BLINK_PHASE.as_millis() as u64,
        );
        assert!(
            !part_states
                .part("head")
                .unwrap()
                .hit_blink
                .as_ref()
                .unwrap()
                .showing_invert
        );

        advance_part_hit_blinks(
            &mut part_states,
            (COMPOSED_PART_HIT_BLINK_PHASE * 2).as_millis() as u64,
        );
        assert!(
            part_states
                .part("head")
                .unwrap()
                .hit_blink
                .as_ref()
                .unwrap()
                .showing_invert
        );

        advance_part_hit_blinks(
            &mut part_states,
            (COMPOSED_PART_HIT_BLINK_PHASE * 4).as_millis() as u64,
        );
        assert!(part_states.part("head").unwrap().hit_blink.is_none());
    }

    #[test]
    fn durability_absorbs_damage_before_core_health() {
        let mut atlas = minimal_atlas();
        atlas.part_definitions[0].gameplay.durability = Some(5);
        atlas.part_definitions[0].gameplay.breakable = Some(true);

        let cache = build_runtime_cache(&atlas).expect("durability atlas should validate");
        let mut health_pools = ComposedHealthPools::from_cache(&cache);
        let mut part_states = ComposedPartStates::from_cache(&cache);

        let first_hit = apply_part_damage(&cache, &mut health_pools, &mut part_states, "body", 3)
            .expect("durability should absorb the first hit");
        assert_eq!(
            first_hit,
            AppliedPartDamage {
                pool_id: Some("core".to_string()),
                remaining_health: None,
                remaining_durability: Some(2),
                broke_part: false,
            }
        );
        assert_eq!(health_pools.pools().get("core"), Some(&10));

        let second_hit = apply_part_damage(&cache, &mut health_pools, &mut part_states, "body", 4)
            .expect("overflow should reach the core pool once durability is exhausted");
        assert_eq!(
            second_hit,
            AppliedPartDamage {
                pool_id: Some("core".to_string()),
                remaining_health: Some(8),
                remaining_durability: Some(0),
                broke_part: true,
            }
        );
        assert_eq!(health_pools.pools().get("core"), Some(&8));
    }

    #[test]
    fn broken_parts_stop_emitting_runtime_collisions() {
        let mut atlas = minimal_atlas();
        atlas.part_definitions[0].gameplay.durability = Some(2);
        atlas.part_definitions[0].gameplay.breakable = Some(true);

        let cache = build_runtime_cache(&atlas).expect("breakable atlas should validate");
        let bindings = ComposedAtlasBindings {
            atlas: Handle::default(),
            sprite_regions: HashMap::from([("sprite_0000".to_string(), AtlasRegionId(0))]),
            sprite_rects: HashMap::from([(
                "sprite_0000".to_string(),
                AtlasRect {
                    x: 0,
                    y: 0,
                    w: 4,
                    h: 4,
                },
            )]),
        };
        let frame = &cache.animations["idle_stand"].frames[0];
        let part_states = ComposedPartStates::from_cache(&cache);
        let (_parts, _metrics, resolved_parts) = compose_frame(
            &frame.poses,
            &cache,
            &bindings,
            &part_states,
            &Handle::default(),
            Entity::from_bits(1),
        )
        .expect("frame should compose");
        let mut part_states = part_states;
        let mut health_pools = ComposedHealthPools::from_cache(&cache);

        let active = build_collision_state(&cache, &resolved_parts, &part_states, Vec2::ZERO);
        assert_eq!(
            active.len(),
            1,
            "targetable part should collide before breakage"
        );

        let damage = apply_part_damage(&cache, &mut health_pools, &mut part_states, "body", 2)
            .expect("damage should break the part");
        assert!(damage.broke_part);

        let after_break = build_collision_state(&cache, &resolved_parts, &part_states, Vec2::ZERO);
        assert!(
            after_break.is_empty(),
            "broken parts should stop contributing targetable collisions"
        );
    }

    #[test]
    fn apply_composed_part_damage_system_updates_shared_entity_pool() {
        let mut app = App::new();
        app.insert_resource(Assets::<CompositionAtlasAsset>::default())
            .add_message::<PartDamageMessage>()
            .add_systems(Update, apply_composed_part_damage);

        let mut atlas = load_exported_mosquiton();
        atlas.prepare_runtime();
        let cache = match atlas.runtime() {
            Ok(cache) => cache.clone(),
            Err(reason) => panic!("mosquiton atlas should prepare: {reason}"),
        };
        let atlas_handle = app
            .world_mut()
            .resource_mut::<Assets<CompositionAtlasAsset>>()
            .add(atlas);
        let entity = app
            .world_mut()
            .spawn((
                ComposedEnemyVisual {
                    atlas_manifest: atlas_handle,
                    sprite_atlas: Handle::default(),
                    track_states: Vec::new(),
                    last_error: None,
                },
                ComposedHealthPools::from_cache(&cache),
                ComposedPartStates::from_cache(&cache),
                Health(40),
            ))
            .id();

        app.world_mut()
            .write_message(PartDamageMessage::new(entity, "head".to_string(), 3));
        app.world_mut()
            .write_message(PartDamageMessage::new(entity, "body".to_string(), 5));
        app.update();

        let pools = app
            .world()
            .entity(entity)
            .get::<ComposedHealthPools>()
            .expect("composed pools should remain attached");
        assert_eq!(pools.pools().get("core"), Some(&37));
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<Health>()
                .expect("entity health should mirror the core pool")
                .0,
            37
        );
    }

    #[test]
    fn composed_health_override_replaces_entity_health_pool_on_setup() {
        let atlas = load_exported_mosquiton();
        let cache = build_runtime_cache(&atlas.atlas).expect("mosquiton atlas should validate");

        let pools = ComposedHealthPools::from_cache_with_entity_health_override(&cache, Some(150));

        assert_eq!(pools.pools().get("core"), Some(&150));
    }

    #[test]
    fn apply_composed_part_damage_system_keeps_mirrored_parts_semantic() {
        let mut app = App::new();
        app.insert_resource(Assets::<CompositionAtlasAsset>::default())
            .add_message::<PartDamageMessage>()
            .add_systems(Update, apply_composed_part_damage);

        let mut atlas = load_exported_mosquiton();
        atlas.prepare_runtime();
        let cache = match atlas.runtime() {
            Ok(cache) => cache.clone(),
            Err(reason) => panic!("mosquiton atlas should prepare: {reason}"),
        };
        let atlas_handle = app
            .world_mut()
            .resource_mut::<Assets<CompositionAtlasAsset>>()
            .add(atlas);
        let entity = app
            .world_mut()
            .spawn((
                ComposedEnemyVisual {
                    atlas_manifest: atlas_handle,
                    sprite_atlas: Handle::default(),
                    track_states: Vec::new(),
                    last_error: None,
                },
                ComposedHealthPools::from_cache(&cache),
                ComposedPartStates::from_cache(&cache),
                Health(40),
            ))
            .id();

        app.world_mut()
            .write_message(PartDamageMessage::new(entity, "arm_l".to_string(), 4));
        app.world_mut()
            .write_message(PartDamageMessage::new(entity, "arm_r".to_string(), 4));
        app.update();

        let pools = app
            .world()
            .entity(entity)
            .get::<ComposedHealthPools>()
            .expect("composed pools should remain attached");
        assert_eq!(pools.pools().get("core"), Some(&38));
        assert_eq!(
            app.world()
                .entity(entity)
                .get::<Health>()
                .expect("entity health should mirror the shared core pool")
                .0,
            38
        );
    }
}
