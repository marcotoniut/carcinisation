//! Parser-based Aseprite export pipeline.
//!
//! This module turns one `.aseprite` source file into a piece atlas package:
//! a packed `atlas.pxi`, an engine-consumable `atlas.px_atlas.ron`, and a
//! `atlas.json` manifest describing how a runtime can compose animation frames
//! from deduplicated part sprites.

#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::unnecessary_wraps
)]

use anyhow::{Context, Result, anyhow, bail, ensure};
use aseprite_loader::{
    binary::{
        blend_mode::BlendMode,
        chunks::{
            layer::{LayerFlags, LayerType},
            slice::SliceKey,
            tags::AnimationDirection,
        },
    },
    loader::AsepriteFile,
};
use image::{ImageBuffer, Rgba, RgbaImage, imageops};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

const DEFAULT_PART_GROUP: &str = "base";
const DEFAULT_ORIGIN_SLICE: &str = "origin";
const CURRENT_SCHEMA_VERSION: u32 = 3;
const RUNTIME_ATLAS_PXI_NAME: &str = "atlas.pxi";
const RUNTIME_COMPOSED_RON_NAME: &str = "atlas.composed.ron";
const DEFAULT_RUNTIME_PALETTE_PATH: &str = "assets/palette/base.png";

/// Top-level manifest for Aseprite exports.
#[derive(Debug, Deserialize)]
pub struct Manifest {
    /// Source sprite entries keyed by the unique `(entity, depth)` pair.
    pub sprites: Vec<SpriteSpec>,
}

/// One exportable composed sprite source.
#[derive(Debug, Deserialize)]
pub struct SpriteSpec {
    /// Path to the `.aseprite` file, relative to the manifest file.
    pub source: String,
    /// Optional output directory relative to the exporter output root.
    ///
    /// When omitted, the exporter mirrors the source file's parent directory
    /// under `assets/sprites/...`.
    pub target_dir: Option<String>,
    /// Canonical composed asset name.
    pub entity: String,
    /// Depth variant of the composed asset.
    pub depth: u8,
    /// Optional palette label reserved for later pipeline stages.
    #[allow(dead_code)]
    pub palette: Option<String>,
    /// Top-level group containing the exportable part layers. Defaults to `"base"`.
    pub part_group: Option<String>,
    /// Slice whose pivot defines the shared composition origin. Defaults to `"origin"`.
    pub origin_slice: Option<String>,
    /// Optional structured semantic-composition metadata relative to the manifest file.
    ///
    /// When omitted, the exporter expects a sibling `*.composition.toml`
    /// sidecar next to the `.aseprite` source.
    pub composition: Option<String>,
}

/// Concrete export request resolved by the CLI wrapper.
pub struct ExportRequest {
    /// Path to the TOML manifest that declares available sprite sources.
    pub manifest_path: PathBuf,
    /// Canonical sprite/entity identifier to export.
    pub entity: String,
    /// Depth variant to export for `entity`.
    pub depth: u8,
    /// Root directory under which generated output will be written.
    pub output_root: PathBuf,
}

/// Final JSON metadata written next to the packed atlas image.
///
/// Despite the legacy `Atlas` name, this is the full composed-asset manifest:
/// visual sprite regions, semantic hierarchy, authored frame placements, and
/// gameplay metadata that the runtime validates before building caches.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompositionAtlas {
    /// Version of the exported JSON schema.
    pub schema_version: u32,
    /// Canonical composed asset identifier.
    pub entity: String,
    /// Depth variant of the composed asset.
    pub depth: u8,
    /// Source `.aseprite` path recorded relative to the manifest.
    pub source: String,
    /// Full source canvas size in pixels.
    pub canvas: Size,
    /// Shared world/root anchor authored from the Aseprite origin slice.
    pub origin: Point,
    /// How the entity position maps to the sprite.
    #[serde(default)]
    pub spawn_anchor: crate::composed_ron::SpawnAnchorMode,
    /// Authored ground contact offset (Y-down from origin). See [`CompactComposedAtlas`].
    #[serde(default)]
    pub ground_anchor_y: Option<i16>,
    /// Authored airborne pivot offset (Y-down from origin). See [`CompactComposedAtlas`].
    #[serde(default)]
    pub air_anchor_y: Option<i16>,
    /// Atlas image filename stored alongside this manifest.
    pub atlas_image: String,
    /// Reusable semantic part definitions.
    #[serde(default)]
    pub part_definitions: Vec<PartDefinition>,
    /// Concrete semantic part instances for this composed asset.
    pub parts: Vec<PartInstance>,
    /// Deduplicated sprite rectangles packed into the atlas image.
    pub sprites: Vec<AtlasSprite>,
    /// Animation tags and their per-frame composition data.
    pub animations: Vec<Animation>,
    /// Gameplay metadata kept separate from visual deduplication.
    #[serde(default)]
    pub gameplay: CompositionGameplay,
}

/// Reusable semantic part definition shared by multiple instances.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartDefinition {
    /// Stable `snake_case` semantic identifier.
    pub id: String,
    /// Broad semantic tags inherited by instances of this definition.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Definition-level gameplay metadata inherited by instances.
    #[serde(default)]
    pub gameplay: PartGameplayMetadata,
}

/// Concrete semantic part instance in the composed hierarchy.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartInstance {
    /// Stable `snake_case` instance identifier.
    pub id: String,
    /// Referenced reusable semantic definition.
    pub definition_id: String,
    /// Human-readable label for tooling/debugging.
    pub name: String,
    /// Optional parent instance id for hierarchical composition.
    pub parent_id: Option<String>,
    /// Optional source Aseprite layer used to author this visual node.
    /// Mutually exclusive with `source_region`.
    pub source_layer: Option<String>,
    /// Optional virtual sub-region of a source layer, resolved at export time.
    /// Mutually exclusive with `source_layer`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_region: Option<SourceRegion>,
    /// Visual-only render fragmentation mode. When set, the exporter produces
    /// multiple render fragments for one logical part. Present for provenance;
    /// the runtime does not act on it directly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split: Option<SplitMode>,
    /// Draw order for this part within a composed frame.
    ///
    /// This belongs to the visual layer only. Validation enforces uniqueness
    /// only for visual parts (those with `source_layer` or `source_region`).
    pub draw_order: u32,
    /// Sprite-local attachment origin authored for this visual part instance.
    #[serde(default)]
    pub pivot: Point,
    /// Instance-only semantic tags layered on top of the definition tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Reserved visibility default. The current runtime requires visibility to
    /// be authored explicitly per frame, so validation rejects `false`.
    #[serde(default = "default_true")]
    pub visible_by_default: bool,
    /// Instance-level gameplay metadata overrides.
    #[serde(default)]
    pub gameplay: PartGameplayMetadata,
}

/// One deduplicated sprite image packed into the atlas.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AtlasSprite {
    /// Stable sprite identifier referenced by [`PartPose`].
    pub id: String,
    /// Rectangle in atlas pixel coordinates.
    pub rect: Rect,
}

/// One Aseprite tag exported as a composed animation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Animation {
    /// Source Aseprite tag name.
    pub tag: String,
    /// Playback direction serialized from Aseprite tag metadata.
    pub direction: String,
    /// Optional repeat count. `None` means infinite or unspecified.
    pub repeats: Option<u32>,
    /// Ordered frames within this animation tag.
    pub frames: Vec<AnimationFrame>,
    /// Part-scoped overrides declared in composition metadata.
    #[serde(default)]
    pub part_overrides: Vec<AnimationOverride>,
}

/// A part-scoped animation override from composition metadata.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AnimationOverride {
    /// Source animation tag to pull pose data from.
    pub source_tag: String,
    /// Part tags to match.
    #[serde(default)]
    pub part_tags: Vec<String>,
    /// Part ids to match.
    #[serde(default)]
    pub part_ids: Vec<String>,
    /// When true, only sprite data is taken; position comes from the base.
    #[serde(default)]
    pub sprite_only: bool,
}

/// One composed frame in an animation tag.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AnimationFrame {
    /// Zero-based source frame index from the `.aseprite` file.
    pub source_frame: usize,
    /// Frame duration in milliseconds.
    pub duration_ms: u32,
    /// One-shot authored cues fired when playback enters this frame.
    #[serde(default)]
    pub events: Vec<AnimationEvent>,
    /// Per-part poses needed to compose this frame.
    pub parts: Vec<PartPose>,
}

/// One authored animation event emitted when playback enters a frame.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct AnimationEvent {
    /// Broad semantic category consumed by gameplay, VFX, or SFX systems.
    pub kind: AnimationEventKind,
    /// Stable consumer-defined event identifier within `kind`.
    pub id: String,
    /// Optional semantic part id used as the event origin or target.
    pub part_id: Option<String>,
    /// Pixel offset from the owning part pivot in local authored sprite space.
    #[serde(default)]
    pub local_offset: Point,
}

/// Supported authored composed-animation event kinds.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AnimationEventKind {
    ProjectileSpawn,
    HitboxOn,
    HitboxOff,
    EffectSpawn,
    SoundPlay,
    /// Synthesised at runtime when a finite animation exhausts its repeats.
    /// Never authored in composition metadata.
    AnimationComplete,
}

/// One per-part authored placement in a composed frame.
///
/// The legacy `Pose` name remains because it is part of the exported schema,
/// but this value is not a pure transform track: it also selects the sprite to
/// draw for the semantic part in this frame.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartPose {
    /// Part instance identifier matching [`PartInstance::id`].
    pub part_id: String,
    /// Sprite identifier matching [`AtlasSprite::id`].
    pub sprite_id: String,
    /// Pivot-to-pivot offset from the parent visual ancestor, or from
    /// [`CompositionAtlas::origin`] for root visual parts.
    pub local_offset: Point,
    /// Whether the runtime should mirror the sprite horizontally.
    pub flip_x: bool,
    /// Whether the runtime should mirror the sprite vertically.
    pub flip_y: bool,
    /// Whether this part is visible in the current frame.
    #[serde(default = "default_true")]
    pub visible: bool,
    /// Final opacity after combining layer opacity and cel opacity.
    pub opacity: u8,
    /// Render fragment index within a split part. Default 0, omitted for
    /// non-split parts. Used only to disambiguate multiple entries with the
    /// same `part_id` within a frame — not a logical/gameplay identifier.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub fragment: u32,
}

/// Top-level gameplay state kept separate from the visual atlas.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CompositionGameplay {
    /// Health pool mirrored onto the entity-level `Health` component.
    pub entity_health_pool: Option<String>,
    /// Shared health pools referenced by semantic parts.
    #[serde(default)]
    pub health_pools: Vec<HealthPool>,
}

/// Shared gameplay health pool.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HealthPool {
    pub id: String,
    pub max_health: u32,
}

/// Gameplay metadata attached to a semantic definition or instance.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PartGameplayMetadata {
    /// Whether the part can be targeted directly.
    pub targetable: Option<bool>,
    /// Shared health-pool id used when routing damage to this part.
    pub health_pool: Option<String>,
    /// Flat armour value retained as metadata for gameplay systems.
    #[serde(default)]
    pub armour: u32,
    /// Optional local durability that absorbs damage before health-pool routing.
    ///
    /// Durability is tracked per semantic part instance at runtime even when
    /// multiple parts share one definition or core health pool.
    pub durability: Option<u32>,
    /// Whether the part should become gameplay-inactive once durability reaches zero.
    ///
    /// Current runtime support disables future targeting/collision for the part.
    /// Visual state changes remain authored work and are not automatic yet.
    pub breakable: Option<bool>,
    /// Fraction of adjusted damage forwarded to the health pool each hit,
    /// regardless of durability absorption. `None` = pool only receives
    /// overflow after durability is depleted.
    #[serde(default)]
    pub pool_damage_ratio: Option<f32>,
    /// Collision volumes attached to the part.
    ///
    /// The current runtime only consumes targetable `Collider` volumes. Other
    /// roles are reserved for later gameplay work and are rejected at
    /// validation time so authoring does not silently no-op.
    #[serde(default)]
    pub collision: Vec<CollisionVolume>,
}

/// One collision-related volume attached to a semantic part.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CollisionVolume {
    /// Stable collision identifier scoped to the owning part.
    pub id: String,
    /// Semantic purpose of the volume.
    pub role: CollisionRole,
    /// Geometry resolved in part-local pivot space.
    pub shape: CollisionShape,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Semantic meaning for a collision volume.
///
/// Only `Collider` is currently supported by the runtime. The other variants are
/// kept in the schema as reserved concepts and must fail loudly until gameplay
/// systems consume them.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CollisionRole {
    Collider,
    Hurtbox,
    Hitbox,
}

/// Serializable collision shape description.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum CollisionShape {
    Circle {
        radius: f32,
        #[serde(default)]
        offset: Vec2Value,
    },
    Box {
        size: Vec2Value,
        #[serde(default)]
        offset: Vec2Value,
    },
}

/// Lightweight serializable 2D vector used in gameplay metadata.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Vec2Value {
    pub x: f32,
    pub y: f32,
}

/// Two-dimensional integer point in source canvas pixels.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct Point {
    /// Horizontal position.
    pub x: i32,
    /// Vertical position.
    pub y: i32,
}

/// Axis-aligned rectangle in pixel coordinates.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width in pixels.
    pub w: u32,
    /// Height in pixels.
    pub h: u32,
}

/// Two-dimensional size in pixels.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Size {
    /// Width in pixels.
    pub w: u32,
    /// Height in pixels.
    pub h: u32,
}

/// Source-of-truth semantic composition metadata loaded alongside the Aseprite file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompositionSource {
    #[serde(default)]
    part_definitions: Vec<PartDefinition>,
    #[serde(default)]
    parts: Vec<CompositionPartSource>,
    #[serde(default)]
    animation_events: Vec<CompositionAnimationEventSource>,
    #[serde(default)]
    animation_overrides: Vec<CompositionAnimationOverrideSource>,
    #[serde(default)]
    gameplay: CompositionGameplay,
    /// How the entity position maps to the sprite. Defaults to `BottomOrigin`.
    #[serde(default)]
    spawn_anchor: crate::composed_ron::SpawnAnchorMode,
    /// Authored ground contact offset (Y-down from origin, positive = below).
    #[serde(default)]
    ground_anchor_y: Option<i16>,
    /// Authored airborne pivot offset (Y-down from origin).
    #[serde(default)]
    air_anchor_y: Option<i16>,
}

/// Per-animation part override declared in composition metadata.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompositionAnimationOverrideSource {
    /// Which animation tags this override applies to.
    tags: Vec<String>,
    /// Source animation tag for override pose data.
    source_tag: String,
    /// Part tags to match.
    #[serde(default)]
    part_tags: Vec<String>,
    /// Part ids to match.
    #[serde(default)]
    part_ids: Vec<String>,
    /// Sprite-only merge mode.
    #[serde(default)]
    sprite_only: bool,
}

/// Virtual region of a source layer, resolved at export time.
///
/// This allows multiple semantic parts to reference different halves of a
/// single authored layer without splitting the source art. The exporter crops
/// each cel at the composition origin and applies centre-column exclusion for
/// odd-width frames so that the interner can detect horizontal flip matches.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourceRegion {
    /// The Aseprite layer name to read from.
    pub layer: String,
    /// Which half of the layer to extract.
    pub half: SplitHalf,
}

/// Which side of the symmetry axis to extract from a source layer.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SplitHalf {
    /// Left half: cel-local X in `[0, centre)`. Centre column excluded.
    Left,
    /// Right half: cel-local X in `[centre + 1, width)`. Centre column excluded.
    Right,
}

/// Visual fragmentation mode for atlas dedup. This is a render-level
/// optimisation that does not create new logical parts or gameplay identities.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SplitMode {
    /// Split at the composition origin X. Produces two render fragments per
    /// frame (left half + right half with centre-column exclusion). The
    /// interner detects horizontal flip matches between the halves.
    MirrorX,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompositionPartSource {
    id: String,
    definition_id: String,
    name: Option<String>,
    parent_id: Option<String>,
    /// Whole-layer source. Mutually exclusive with `source_region`.
    source_layer: Option<String>,
    /// Virtual sub-region of a layer. Mutually exclusive with `source_layer`.
    source_region: Option<SourceRegion>,
    /// Visual-only render fragmentation hint. When set, the exporter produces
    /// multiple render fragments from one logical part for atlas dedup.
    /// Mutually exclusive with `source_region`.
    split: Option<SplitMode>,
    draw_order: u32,
    pivot: Option<Point>,
    #[serde(default)]
    tags: Vec<String>,
    visible_by_default: Option<bool>,
    #[serde(default)]
    gameplay: PartGameplayMetadata,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompositionAnimationEventSource {
    tag: String,
    frame: usize,
    kind: AnimationEventKind,
    id: String,
    part_id: Option<String>,
    #[serde(default)]
    local_offset: Point,
}

#[derive(Debug, Clone, Copy)]
struct SelectedLayer<'a> {
    index: usize,
    name: &'a str,
    opacity: u8,
    visible: bool,
}

#[derive(Debug, Clone)]
struct RawPlacement {
    sprite_id: String,
    top_left: Point,
    flip_x: bool,
    flip_y: bool,
    opacity: u8,
}

#[derive(Debug, Clone)]
struct PreparedSprite {
    id: String,
    image: RgbaImage,
}

#[derive(Debug, Default)]
struct SpriteInterner {
    sprites: Vec<PreparedSprite>,
    cache: HashMap<ImageKey, String>,
    stats: InternerStats,
}

/// Tracks deduplication statistics during sprite interning.
#[derive(Clone, Debug, Default)]
pub struct InternerStats {
    /// Total `intern()` calls.
    pub total_interns: u32,
    /// Calls that produced a new unique sprite.
    pub new_sprites: u32,
    /// Calls that matched an existing sprite exactly (identity, no flip).
    pub exact_hits: u32,
    /// Calls that matched via horizontal flip.
    pub flip_x_hits: u32,
    /// Calls that matched via vertical flip.
    pub flip_y_hits: u32,
    /// Calls that matched via both horizontal and vertical flip.
    pub flip_xy_hits: u32,
}

#[derive(Debug, Clone)]
struct SpriteUsage {
    sprite_id: String,
    flip_x: bool,
    flip_y: bool,
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct ImageKey {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

fn default_true() -> bool {
    true
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde skip_serializing_if requires &T
fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

/// Exports one piece-atlas package for the requested entity/depth pair.
///
/// The exporter reads the source `.aseprite` file directly, selects part layers
/// from the configured top-level group, resolves the shared origin from the
/// configured slice, deduplicates repeated or mirrored cel images, packs them
/// into `atlas.pxi`, and writes a matching `atlas.json` manifest.
pub fn export_sprite(request: &ExportRequest) -> Result<()> {
    let manifest = load_manifest(&request.manifest_path)?;
    let sprite = find_sprite(&manifest, &request.entity, request.depth)?;
    let manifest_dir = request
        .manifest_path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let source_path = manifest_dir.join(&sprite.source);
    ensure!(
        source_path.exists(),
        "Aseprite source file does not exist: {}",
        source_path.display()
    );

    let source_bytes = fs::read(&source_path)
        .with_context(|| format!("Failed to read {}", source_path.display()))?;
    let aseprite = AsepriteFile::load(&source_bytes)
        .with_context(|| format!("Failed to parse {}", source_path.display()))?;
    let composition_path = manifest_dir.join(sprite.composition_path()?);
    let composition = load_composition_source(&composition_path)?;
    let (atlas_image, atlas_metadata, interner_stats) =
        build_piece_atlas(sprite, &aseprite, &composition)?;

    let output_dir = request
        .output_root
        .join(sprite.target_dir_path()?)
        .join(format!("{}_{}", sprite.entity, sprite.depth));
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create output directory {}", output_dir.display()))?;

    let source_png = output_dir.join("source.png");
    let runtime_atlas_pxi = output_dir.join(RUNTIME_ATLAS_PXI_NAME);
    let atlas_json = output_dir.join("atlas.json");
    atlas_image
        .save(&source_png)
        .with_context(|| format!("Failed to save {}", source_png.display()))?;

    let palette_indices = compute_palette_indices(&atlas_image)?;
    let pxi_bytes =
        crate::pxi::encode_compressed(atlas_image.width(), atlas_image.height(), &palette_indices)
            .context("Failed to encode PXI atlas")?;
    fs::write(&runtime_atlas_pxi, &pxi_bytes)
        .with_context(|| format!("Failed to write {}", runtime_atlas_pxi.display()))?;

    let metadata = CompositionAtlas {
        schema_version: CURRENT_SCHEMA_VERSION,
        atlas_image: source_png
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
        ..atlas_metadata
    };
    let entity_depth_dir = sprite
        .target_dir_path()?
        .join(format!("{}_{}", sprite.entity, sprite.depth));
    let runtime_atlas_asset_pxi_path = entity_depth_dir.join(RUNTIME_ATLAS_PXI_NAME);
    write_px_atlas_metadata(&output_dir, &metadata, runtime_atlas_asset_pxi_path)?;
    fs::write(
        &atlas_json,
        format!("{}\n", serde_json::to_string_pretty(&metadata)?),
    )
    .with_context(|| format!("Failed to write {}", atlas_json.display()))?;

    let compact =
        crate::composed_ron::encode(&metadata).context("Failed to encode compact composed RON")?;
    // Log per-animation ground anchor overrides.
    for anim in &compact.animations {
        if let Some(ground) = anim.ground_anchor_y {
            eprintln!("    {}: ground_anchor_y={ground} (override)", anim.tag);
        }
    }
    let composed_ron_path = output_dir.join(RUNTIME_COMPOSED_RON_NAME);
    let composed_ron_body = crate::composed_ron::to_ron(&compact)
        .context("Failed to serialize compact composed RON")?;
    fs::write(&composed_ron_path, &composed_ron_body)
        .with_context(|| format!("Failed to write {}", composed_ron_path.display()))?;

    println!(
        "Exported piece atlas for {} depth {} to {}",
        sprite.entity,
        sprite.depth,
        output_dir.display()
    );

    // Post-export analysis: dedup report and split candidate detection.
    let report = crate::analysis::build_report(
        &metadata,
        &interner_stats,
        atlas_image.width(),
        atlas_image.height(),
        Some(&atlas_image),
    );
    crate::analysis::print_report(&report);
    crate::analysis::write_json_report(&report, &output_dir)?;

    Ok(())
}

/// Loads and validates an Aseprite export manifest.
///
/// Validation currently enforces unique `(entity, depth)` pairs and requires
/// each sprite entry to define `source` and `entity`.
pub fn load_manifest(path: &Path) -> Result<Manifest> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read manifest {}", path.display()))?;
    let manifest = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse manifest {}", path.display()))?;
    validate_manifest(&manifest).with_context(|| format!("Invalid manifest {}", path.display()))?;
    Ok(manifest)
}

/// Returns the default manifest path used by the exporter CLI.
#[must_use]
pub fn default_manifest_path() -> PathBuf {
    PathBuf::from("resources/sprites/data.toml")
}

/// Returns the default output root used by the exporter CLI.
#[must_use]
pub fn default_output_root() -> PathBuf {
    PathBuf::from("tmp/aseprite-export")
}

fn load_composition_source(path: &Path) -> Result<CompositionSource> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read composition metadata {}", path.display()))?;
    let composition = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse composition metadata {}", path.display()))?;
    validate_composition_source(&composition)
        .with_context(|| format!("Invalid composition metadata {}", path.display()))?;
    Ok(composition)
}

fn build_piece_atlas(
    sprite: &SpriteSpec,
    aseprite: &AsepriteFile<'_>,
    composition: &CompositionSource,
) -> Result<(RgbaImage, CompositionAtlas, InternerStats)> {
    ensure!(
        !aseprite.tags().is_empty(),
        "Aseprite file '{}' does not define any animation tags",
        sprite.source
    );

    let selected_layers = select_part_layers(aseprite, sprite.part_group_name())?;
    let origin = resolve_origin(aseprite, sprite.origin_slice_name())?;
    let layer_lookup: HashMap<&str, &SelectedLayer<'_>> = selected_layers
        .iter()
        .map(|layer| (layer.name, layer))
        .collect();
    let parts = composition
        .parts
        .iter()
        .map(CompositionPartSource::to_instance)
        .collect::<Vec<_>>();
    validate_layer_bindings(&parts, &selected_layers)?;
    let authored_events = bind_animation_events(aseprite, &parts, &composition.animation_events)?;
    let topo_order = build_part_topology(&parts)?;
    let mut sprite_interner = SpriteInterner::default();
    let mut animations = Vec::new();

    for tag in aseprite.tags() {
        let mut frames = Vec::new();
        for source_frame in tag.range.clone() {
            let frame_index = usize::from(source_frame);
            let frame = aseprite.frames().get(frame_index).ok_or_else(|| {
                anyhow!(
                    "Tag '{}' references missing frame {}",
                    tag.name,
                    frame_index
                )
            })?;
            let raw_placements = build_raw_placements(
                &parts,
                &layer_lookup,
                aseprite,
                frame,
                frame_index,
                origin,
                &mut sprite_interner,
            )?;
            let placements =
                build_frame_poses(&parts, &raw_placements, &topo_order, frame_index, &tag.name)?;
            let authored_frame_events = authored_events
                .get(tag.name.as_str())
                .and_then(|events_by_frame| events_by_frame.get(&frames.len()))
                .cloned()
                .unwrap_or_default();
            frames.push(AnimationFrame {
                source_frame: frame_index,
                duration_ms: u32::from(frame.duration),
                events: authored_frame_events,
                parts: placements,
            });
        }

        // Collect metadata-declared overrides that target this animation tag.
        let part_overrides: Vec<AnimationOverride> = composition
            .animation_overrides
            .iter()
            .filter(|o| o.tags.contains(&tag.name))
            .map(|o| AnimationOverride {
                source_tag: o.source_tag.clone(),
                part_tags: o.part_tags.clone(),
                part_ids: o.part_ids.clone(),
                sprite_only: o.sprite_only,
            })
            .collect();

        animations.push(Animation {
            tag: tag.name.clone(),
            direction: animation_direction_name(tag.direction),
            repeats: tag.repeat.map(u32::from),
            frames,
            part_overrides,
        });
    }

    let (atlas_image, atlas_sprites) = pack_sprites(&sprite_interner.sprites)?;
    let mut metadata = CompositionAtlas {
        schema_version: CURRENT_SCHEMA_VERSION,
        entity: sprite.entity.clone(),
        depth: sprite.depth,
        source: sprite.source.clone(),
        canvas: Size {
            w: u32::from(aseprite.size().0),
            h: u32::from(aseprite.size().1),
        },
        origin,
        spawn_anchor: composition.spawn_anchor,
        ground_anchor_y: composition.ground_anchor_y,
        air_anchor_y: composition.air_anchor_y,
        atlas_image: String::new(),
        part_definitions: composition.part_definitions.clone(),
        parts,
        sprites: atlas_sprites,
        animations,
        gameplay: composition.gameplay.clone(),
    };

    let had_ground = metadata.ground_anchor_y.is_some();
    let had_air = metadata.air_anchor_y.is_some();
    derive_anchor_offsets(&mut metadata);
    // Log derived values so developers can verify without inspecting the RON.
    if !had_ground || !had_air {
        let ground_src = if had_ground { "authored" } else { "derived" };
        let air_src = if had_air { "authored" } else { "derived" };
        eprintln!(
            "  anchors: ground={} ({ground_src}), air={} ({air_src})",
            metadata.ground_anchor_y.unwrap_or(0),
            metadata.air_anchor_y.unwrap_or(0),
        );
    }
    validate_composition_atlas(&metadata)?;

    Ok((atlas_image, metadata, sprite_interner.stats))
}

#[derive(Serialize)]
struct PxSpriteAtlasDescriptor {
    /// Path to the compact indexed runtime image (.pxi).
    indexed_image: PathBuf,
    regions: Vec<AtlasRegionDescriptor>,
    #[serde(default)]
    names: BTreeMap<String, u32>,
    /// Per-region animation metadata derived from aseprite tags.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    animations: BTreeMap<String, RegionAnimationDescriptor>,
}

/// Animation metadata for one atlas region, derived from an aseprite tag.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionAnimationDescriptor {
    /// Total animation cycle duration in milliseconds.
    pub duration_ms: u64,
    /// Playback direction: "forward" or "backward".
    pub direction: String,
    /// Finish behavior: "loop", "mark", or "despawn".
    pub on_finish: String,
}

#[derive(Serialize)]
struct AtlasRegionDescriptor {
    frame_size: [u32; 2],
    frames: Vec<AtlasRectDescriptor>,
}

#[derive(Serialize)]
struct AtlasRectDescriptor {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

fn write_px_atlas_metadata(
    output_dir: &Path,
    metadata: &CompositionAtlas,
    pxi_asset_path: PathBuf,
) -> Result<()> {
    let descriptor = PxSpriteAtlasDescriptor {
        indexed_image: pxi_asset_path,
        regions: metadata
            .sprites
            .iter()
            .map(|sprite| AtlasRegionDescriptor {
                frame_size: [sprite.rect.w, sprite.rect.h],
                frames: vec![AtlasRectDescriptor {
                    x: sprite.rect.x,
                    y: sprite.rect.y,
                    w: sprite.rect.w,
                    h: sprite.rect.h,
                }],
            })
            .collect(),
        names: metadata
            .sprites
            .iter()
            .enumerate()
            .map(|(index, sprite)| (sprite.id.clone(), index as u32))
            .collect(),
        animations: BTreeMap::new(),
    };

    let atlas_path = output_dir.join("atlas.px_atlas.ron");
    let body = ron::ser::to_string_pretty(&descriptor, ron::ser::PrettyConfig::default())
        .context("Failed to serialize atlas metadata")?;
    fs::write(&atlas_path, body)
        .with_context(|| format!("Failed to write {}", atlas_path.display()))?;
    Ok(())
}

/// Maps source atlas pixels to palette indices for the `.pxi` runtime image.
///
/// Quantizes authored colours to the nearest runtime palette entry, then
/// returns one `u8` per pixel: 0 = transparent, 1–N = palette index.
/// Map every pixel in `source` to a 4-bit palette index (0 = transparent).
pub fn compute_palette_indices(source: &RgbaImage) -> Result<Vec<u8>> {
    let palette = load_runtime_palette(Path::new(DEFAULT_RUNTIME_PALETTE_PATH))?;
    let grayscale_mapping = grayscale_ramp_mapping(source, &palette);

    // Build palette index lookup: RGB → 1-based index (0 = transparent).
    // Keyed on RGB (not RGBA) to match the runtime Palette which ignores alpha.
    let palette_index: HashMap<[u8; 3], u8> = palette
        .iter()
        .enumerate()
        .map(|(i, color)| ([color.0[0], color.0[1], color.0[2]], (i + 1) as u8))
        .collect();

    ensure!(
        palette_index.len() <= 15,
        "Runtime palette has {} colours, but PXI format supports at most 15 \
         (indices 1–15, with 0 reserved for transparent)",
        palette_index.len(),
    );

    let indices: Vec<u8> = source
        .pixels()
        .map(|pixel| {
            if pixel.0[3] == 0 {
                return 0;
            }
            // Quantize to palette colour first, then look up its index.
            let quantized = grayscale_mapping
                .get(&pixel.0)
                .copied()
                .unwrap_or_else(|| nearest_palette_color(*pixel, &palette));
            let rgb = [quantized.0[0], quantized.0[1], quantized.0[2]];
            *palette_index.get(&rgb).unwrap_or(&0)
        })
        .collect();

    Ok(indices)
}

fn load_runtime_palette(path: &Path) -> Result<Vec<Rgba<u8>>> {
    let palette_image = image::open(path)
        .with_context(|| format!("Failed to load runtime palette {}", path.display()))?
        .to_rgba8();
    let mut palette = Vec::new();

    for pixel in palette_image.pixels() {
        if pixel.0[3] == 0 || palette.iter().any(|entry: &Rgba<u8>| entry.0 == pixel.0) {
            continue;
        }
        palette.push(*pixel);
    }

    ensure!(
        !palette.is_empty(),
        "Runtime palette '{}' does not contain any opaque colors",
        path.display()
    );

    Ok(palette)
}

fn nearest_palette_color(pixel: Rgba<u8>, palette: &[Rgba<u8>]) -> Rgba<u8> {
    let [r, g, b, a] = pixel.0;

    palette
        .iter()
        .copied()
        .min_by_key(|candidate| {
            let [cr, cg, cb, ca] = candidate.0;
            let dr = i32::from(r) - i32::from(cr);
            let dg = i32::from(g) - i32::from(cg);
            let db = i32::from(b) - i32::from(cb);
            let da = i32::from(a) - i32::from(ca);
            dr * dr + dg * dg + db * db + da * da
        })
        .expect("palette is guaranteed non-empty by load_runtime_palette")
}

/// Preserves authored grayscale ramps by mapping tones by luminance rank rather
/// than raw RGB distance.
///
/// The project palette is not itself grayscale. Naive nearest-color mapping can
/// collapse multiple authored tones into the same palette entry, which is
/// visually incorrect for sprites intentionally authored as ordered ramps.
fn grayscale_ramp_mapping(source: &RgbaImage, palette: &[Rgba<u8>]) -> HashMap<[u8; 4], Rgba<u8>> {
    let mut source_colors = Vec::<Rgba<u8>>::new();

    for pixel in source.pixels() {
        if pixel.0[3] == 0 {
            continue;
        }
        if source_colors.iter().any(|existing| existing.0 == pixel.0) {
            continue;
        }
        source_colors.push(*pixel);
    }

    if source_colors.is_empty()
        || source_colors.len() > palette.len()
        || source_colors.iter().any(|pixel| !is_grayscale(*pixel))
    {
        return HashMap::new();
    }

    source_colors.sort_by_key(|pixel| luminance_key(*pixel));

    let mut palette_by_luminance = palette.to_vec();
    palette_by_luminance.sort_by_key(|pixel| luminance_key(*pixel));

    source_colors
        .into_iter()
        .zip(palette_by_luminance)
        .map(|(source, target)| (source.0, target))
        .collect()
}

fn is_grayscale(pixel: Rgba<u8>) -> bool {
    let [r, g, b, _] = pixel.0;
    r == g && g == b
}

fn luminance_key(pixel: Rgba<u8>) -> u32 {
    let [r, g, b, _] = pixel.0;
    2126_u32 * u32::from(r) + 7152_u32 * u32::from(g) + 722_u32 * u32::from(b)
}

fn select_part_layers<'a>(
    aseprite: &'a AsepriteFile<'a>,
    part_group: &str,
) -> Result<Vec<SelectedLayer<'a>>> {
    let group_index = aseprite
        .file
        .layers
        .iter()
        .position(|layer| layer.layer_type == LayerType::Group && layer.name == part_group)
        .ok_or_else(|| anyhow!("Missing part group '{part_group}'"))?;
    let group = &aseprite.file.layers[group_index];
    let group_level = group.child_level;
    let child_level = group_level + 1;
    let mut selected = Vec::new();

    for (index, layer) in aseprite
        .file
        .layers
        .iter()
        .enumerate()
        .skip(group_index + 1)
    {
        if layer.child_level <= group_level {
            break;
        }

        if layer.child_level != child_level {
            if matches!(layer.layer_type, LayerType::Normal | LayerType::Group) {
                bail!(
                    "Nested layers inside part group '{}' are not supported yet: '{}'",
                    part_group,
                    layer.name
                );
            }
            continue;
        }

        match layer.layer_type {
            LayerType::Normal => {
                ensure!(
                    layer.blend_mode == BlendMode::Normal,
                    "Layer '{}' uses unsupported blend mode {:?}",
                    layer.name,
                    layer.blend_mode
                );
                selected.push(SelectedLayer {
                    index,
                    name: layer.name,
                    opacity: layer.opacity,
                    visible: layer.flags.contains(LayerFlags::VISIBLE),
                });
            }
            LayerType::Group => {
                bail!(
                    "Nested groups inside part group '{}' are not supported yet: '{}'",
                    part_group,
                    layer.name
                );
            }
            LayerType::Tilemap | LayerType::Unknown(_) => {
                bail!(
                    "Unsupported layer type inside part group '{}': '{}'",
                    part_group,
                    layer.name
                );
            }
        }
    }

    ensure!(
        !selected.is_empty(),
        "Part group '{part_group}' does not contain any direct child layers"
    );
    Ok(selected)
}

fn load_cel_image(
    aseprite: &AsepriteFile<'_>,
    cel: &aseprite_loader::loader::FrameCel,
) -> Result<RgbaImage> {
    let width = usize::from(cel.size.0);
    let height = usize::from(cel.size.1);
    let mut pixels = vec![0; width * height * 4];
    aseprite
        .load_image(cel.image_index, &mut pixels)
        .context("Failed to decode cel image")?;
    RgbaImage::from_raw(cel.size.0.into(), cel.size.1.into(), pixels)
        .ok_or_else(|| anyhow!("Failed to construct RGBA image for cel {}", cel.image_index))
}

fn resolve_origin(aseprite: &AsepriteFile<'_>, slice_name: &str) -> Result<Point> {
    let slice = aseprite
        .slices()
        .iter()
        .find(|slice| slice.name == slice_name)
        .ok_or_else(|| anyhow!("Missing required origin slice '{slice_name}'"))?;
    let first_key = slice
        .slice_keys
        .first()
        .ok_or_else(|| anyhow!("Origin slice '{slice_name}' does not contain any keys"))?;
    let first_pivot = first_key
        .pivot
        .ok_or_else(|| anyhow!("Origin slice '{slice_name}' must define a pivot"))?;

    for key in slice.slice_keys.iter().skip(1) {
        ensure!(
            key_matches_origin(first_key, key),
            "Origin slice '{slice_name}' varies across frames; this exporter requires one shared origin"
        );
    }

    Ok(Point {
        x: first_key.x + first_pivot.x,
        y: first_key.y + first_pivot.y,
    })
}

fn key_matches_origin(reference: &SliceKey, candidate: &SliceKey) -> bool {
    reference.x == candidate.x
        && reference.y == candidate.y
        && reference.width == candidate.width
        && reference.height == candidate.height
        && match (reference.pivot, candidate.pivot) {
            (Some(a), Some(b)) => a.x == b.x && a.y == b.y,
            (None, None) => true,
            _ => false,
        }
}

fn animation_direction_name(direction: AnimationDirection) -> String {
    match direction {
        AnimationDirection::Forward => String::from("forward"),
        AnimationDirection::Reverse => String::from("reverse"),
        AnimationDirection::PingPong => String::from("ping_pong"),
        AnimationDirection::PingPongReverse => String::from("ping_pong_reverse"),
        AnimationDirection::Unknown(value) => format!("unknown_{value}"),
    }
}

fn combine_opacity(layer_opacity: u8, cel_opacity: u8) -> u8 {
    let combined = (u32::from(layer_opacity) * u32::from(cel_opacity) + 127) / 255;
    combined.min(255) as u8
}

fn normalize_part_id(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    let mut last_was_sep = false;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            last_was_sep = false;
        } else if !last_was_sep {
            normalized.push('_');
            last_was_sep = true;
        }
    }

    normalized = normalized.trim_matches('_').to_string();
    if normalized.is_empty() {
        String::from("part")
    } else {
        normalized
    }
}

fn pack_sprites(sprites: &[PreparedSprite]) -> Result<(RgbaImage, Vec<AtlasSprite>)> {
    ensure!(
        !sprites.is_empty(),
        "No non-empty sprites were extracted from Aseprite"
    );

    let area: u64 = sprites
        .iter()
        .map(|sprite| u64::from(sprite.image.width()) * u64::from(sprite.image.height()))
        .sum();
    let widest = sprites
        .iter()
        .map(|sprite| sprite.image.width())
        .max()
        .unwrap_or(1);
    let target_width = next_power_of_two(u32::max(widest, (area as f64).sqrt().ceil() as u32));

    let mut placements = Vec::with_capacity(sprites.len());
    let mut cursor_x = 0;
    let mut cursor_y = 0;
    let mut shelf_height = 0;
    let mut atlas_height = 0;

    for sprite in sprites {
        let width = sprite.image.width();
        let height = sprite.image.height();
        if cursor_x > 0 && cursor_x + width > target_width {
            cursor_x = 0;
            cursor_y += shelf_height;
            shelf_height = 0;
        }

        placements.push((
            sprite.id.clone(),
            Rect {
                x: cursor_x,
                y: cursor_y,
                w: width,
                h: height,
            },
        ));
        cursor_x += width;
        shelf_height = shelf_height.max(height);
        atlas_height = atlas_height.max(cursor_y + height);
    }

    let mut atlas = ImageBuffer::from_pixel(target_width, atlas_height.max(1), Rgba([0, 0, 0, 0]));
    let mut atlas_sprites = Vec::with_capacity(placements.len());

    for (sprite, (id, rect)) in sprites.iter().zip(placements.into_iter()) {
        imageops::overlay(
            &mut atlas,
            &sprite.image,
            i64::from(rect.x),
            i64::from(rect.y),
        );
        atlas_sprites.push(AtlasSprite { id, rect });
    }

    Ok((atlas, atlas_sprites))
}

fn next_power_of_two(value: u32) -> u32 {
    if value <= 1 {
        1
    } else {
        value.next_power_of_two()
    }
}

fn intern_sprite(
    sprites: &mut Vec<PreparedSprite>,
    cache: &mut HashMap<ImageKey, String>,
    stats: &mut InternerStats,
    image: &RgbaImage,
) -> SpriteUsage {
    stats.total_interns += 1;

    let variants = [
        (image.clone(), false, false),
        (imageops::flip_horizontal(image), true, false),
        (imageops::flip_vertical(image), false, true),
        (
            imageops::flip_vertical(&imageops::flip_horizontal(image)),
            true,
            true,
        ),
    ];

    for (variant_image, flip_x, flip_y) in &variants {
        let key = image_key(variant_image);
        if let Some(sprite_id) = cache.get(&key) {
            match (flip_x, flip_y) {
                (false, false) => stats.exact_hits += 1,
                (true, false) => stats.flip_x_hits += 1,
                (false, true) => stats.flip_y_hits += 1,
                (true, true) => stats.flip_xy_hits += 1,
            }
            return SpriteUsage {
                sprite_id: sprite_id.clone(),
                flip_x: *flip_x,
                flip_y: *flip_y,
            };
        }
    }

    stats.new_sprites += 1;
    let sprite_id = format!("sprite_{:04}", sprites.len());
    let key = image_key(image);
    cache.insert(key, sprite_id.clone());
    sprites.push(PreparedSprite {
        id: sprite_id.clone(),
        image: image.clone(),
    });
    SpriteUsage {
        sprite_id,
        flip_x: false,
        flip_y: false,
    }
}

fn image_key(image: &RgbaImage) -> ImageKey {
    ImageKey {
        width: image.width(),
        height: image.height(),
        pixels: image.as_raw().clone(),
    }
}

fn trim_transparent_bounds(image: &RgbaImage) -> Option<(RgbaImage, Rect)> {
    let mut min_x = image.width();
    let mut min_y = image.height();
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;

    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel.0[3] == 0 {
            continue;
        }
        found = true;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }

    if !found {
        return None;
    }

    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;
    Some((
        imageops::crop_imm(image, min_x, min_y, width, height).to_image(),
        Rect {
            x: min_x,
            y: min_y,
            w: width,
            h: height,
        },
    ))
}

fn find_sprite<'a>(manifest: &'a Manifest, entity: &str, depth: u8) -> Result<&'a SpriteSpec> {
    manifest
        .sprites
        .iter()
        .find(|sprite| sprite.entity == entity && sprite.depth == depth)
        .ok_or_else(|| anyhow!("No sprite entry found for entity '{entity}' at depth {depth}"))
}

fn validate_manifest(manifest: &Manifest) -> Result<()> {
    let mut sprite_keys = HashMap::new();

    for (index, sprite) in manifest.sprites.iter().enumerate() {
        if let Some(previous_index) =
            sprite_keys.insert((sprite.entity.as_str(), sprite.depth), index)
        {
            bail!(
                "Duplicate sprite entry for entity '{}' at depth {} (entries {} and {})",
                sprite.entity,
                sprite.depth,
                previous_index + 1,
                index + 1
            );
        }

        ensure!(
            !sprite.source.trim().is_empty(),
            "Sprite entry {} is missing a source path",
            index + 1
        );
        if let Some(target_dir) = &sprite.target_dir {
            ensure!(
                !target_dir.trim().is_empty(),
                "Sprite entry {} has an empty target_dir override",
                index + 1
            );
        }
        ensure!(
            !sprite.entity.trim().is_empty(),
            "Sprite entry {} is missing an entity name",
            index + 1
        );
        if let Some(composition) = &sprite.composition {
            ensure!(
                !composition.trim().is_empty(),
                "Sprite entry {} has an empty composition override",
                index + 1
            );
        }
    }

    Ok(())
}

fn validate_composition_source(source: &CompositionSource) -> Result<()> {
    ensure!(
        !source.part_definitions.is_empty(),
        "composition metadata must define at least one part definition"
    );
    ensure!(
        !source.parts.is_empty(),
        "composition metadata must define at least one part instance"
    );
    validate_composition_source_contracts(source)?;

    let parts = source
        .parts
        .iter()
        .map(CompositionPartSource::to_instance)
        .collect::<Vec<_>>();
    validate_part_graph(&source.part_definitions, &parts, &source.gameplay)
}

fn validate_composition_source_contracts(source: &CompositionSource) -> Result<()> {
    for part in &source.parts {
        ensure!(
            part.source_layer.is_none() || part.source_region.is_none(),
            "part '{}' must specify source_layer OR source_region, not both",
            part.id
        );
        if part.split.is_some() {
            ensure!(
                part.source_region.is_none(),
                "part '{}' has both `split` and `source_region`; these are mutually exclusive",
                part.id
            );
        }
        if part.is_visual() {
            ensure!(
                part.pivot.is_some(),
                "visual part '{}' must define an explicit pivot in composition metadata",
                part.id
            );
        }
        ensure!(
            part.visible_by_default.unwrap_or(true),
            "part '{}' cannot set visible_by_default = false; visibility must be authored per frame",
            part.id
        );
    }

    for event in &source.animation_events {
        ensure!(
            !event.tag.trim().is_empty(),
            "animation events must define a non-empty tag"
        );
        ensure!(
            !event.id.trim().is_empty(),
            "animation event on tag '{}' frame {} must define a non-empty id",
            event.tag,
            event.frame
        );
        if let Some(part_id) = event.part_id.as_deref() {
            ensure!(
                part_id == normalize_part_id(part_id),
                "animation event '{}' on tag '{}' must reference a snake_case part id",
                event.id,
                event.tag
            );
        }
    }

    Ok(())
}

/// Derive missing anchor offsets from animation frame data.
///
/// When `ground_anchor_y` is `None`, scans all visible poses across every
/// animation frame and takes the maximum `(offset_y + sprite_height)` — the
/// lowest rendered pixel in canvas Y-down space, relative to the composition
/// origin.  This is where the feet sit.
///
/// When `air_anchor_y` is `None`, finds the first part whose definition is
/// tagged both `"core"` and `"torso"` (the body part), then computes its
/// sprite vertical centre from the first visible pose.
///
/// Authored values in the composition TOML always take precedence; this
/// function only fills in `None` fields.
fn derive_anchor_offsets(atlas: &mut CompositionAtlas) {
    use crate::composed_ron::SpawnAnchorMode;

    if atlas.spawn_anchor != SpawnAnchorMode::Origin {
        return; // BottomOrigin entities don't need anchor offsets.
    }

    // Build sprite-id → height lookup.
    let sprite_height: HashMap<&str, i32> = atlas
        .sprites
        .iter()
        .map(|s| (s.id.as_str(), s.rect.h as i32))
        .collect();

    // --- Ground anchor: lowest visible pixel across all frames ---
    if atlas.ground_anchor_y.is_none() {
        let mut max_bottom: Option<i32> = None;
        for anim in &atlas.animations {
            for frame in &anim.frames {
                for pose in &frame.parts {
                    if !pose.visible {
                        continue;
                    }
                    if let Some(&h) = sprite_height.get(pose.sprite_id.as_str()) {
                        let bottom = pose.local_offset.y + h;
                        max_bottom = Some(max_bottom.map_or(bottom, |prev| prev.max(bottom)));
                    }
                }
            }
        }
        if let Some(bottom) = max_bottom {
            atlas.ground_anchor_y = i16::try_from(bottom).ok();
        }
    }

    // --- Air anchor: body part visual centre ---
    if atlas.air_anchor_y.is_none() {
        // Find the body part id: definition tagged both "core" and "torso".
        let body_def_id: Option<&str> = atlas.part_definitions.iter().find_map(|def| {
            let has_core = def.tags.iter().any(|t| t == "core");
            let has_torso = def.tags.iter().any(|t| t == "torso");
            if has_core && has_torso {
                Some(def.id.as_str())
            } else {
                None
            }
        });

        if let Some(body_def_id) = body_def_id {
            // Find part instances that match this definition.
            let body_part_ids: Vec<&str> = atlas
                .parts
                .iter()
                .filter(|p| p.definition_id == body_def_id)
                .map(|p| p.id.as_str())
                .collect();

            // Find the first visible pose for any body part instance.
            'outer: for anim in &atlas.animations {
                for frame in &anim.frames {
                    for pose in &frame.parts {
                        if !pose.visible || !body_part_ids.contains(&pose.part_id.as_str()) {
                            continue;
                        }
                        if let Some(&h) = sprite_height.get(pose.sprite_id.as_str()) {
                            let centre = pose.local_offset.y + h / 2;
                            atlas.air_anchor_y = i16::try_from(centre).ok();
                            break 'outer;
                        }
                    }
                }
            }
        } else {
            eprintln!(
                "warning: {} depth {}: no part definition tagged [\"core\", \"torso\"] — \
                 air_anchor_y will default to 0 (at origin)",
                atlas.entity, atlas.depth,
            );
        }
    }
}

/// This function is intentionally strict: if a schema concept is not yet
/// supported by the runtime, validation should reject it rather than allow a
/// silent partial load.
pub fn validate_composition_atlas(atlas: &CompositionAtlas) -> Result<()> {
    validate_part_graph(&atlas.part_definitions, &atlas.parts, &atlas.gameplay)?;

    let mut sprite_ids = HashSet::new();
    for sprite in &atlas.sprites {
        ensure!(
            sprite_ids.insert(sprite.id.as_str()),
            "duplicate sprite id '{}'",
            sprite.id
        );
        ensure!(
            sprite.rect.w > 0 && sprite.rect.h > 0,
            "sprite '{}' must have a non-zero rectangle",
            sprite.id
        );
    }

    let part_lookup: HashMap<&str, &PartInstance> = atlas
        .parts
        .iter()
        .map(|part| (part.id.as_str(), part))
        .collect();
    let sprite_lookup: HashSet<&str> = atlas
        .sprites
        .iter()
        .map(|sprite| sprite.id.as_str())
        .collect();
    let mut animation_tags = HashSet::new();

    for animation in &atlas.animations {
        ensure!(
            animation_tags.insert(animation.tag.as_str()),
            "duplicate animation tag '{}'",
            animation.tag
        );
        ensure!(
            !animation.frames.is_empty(),
            "animation '{}' must contain at least one frame",
            animation.tag
        );

        for frame in &animation.frames {
            let mut frame_part_fragments: HashSet<(&str, u32)> = HashSet::new();
            let mut fragments_by_part: HashMap<&str, Vec<u32>> = HashMap::new();
            let mut frame_events = HashSet::new();
            for pose in &frame.parts {
                ensure!(
                    frame_part_fragments.insert((pose.part_id.as_str(), pose.fragment)),
                    "animation '{}' frame {} defines part '{}' fragment {} more than once",
                    animation.tag,
                    frame.source_frame,
                    pose.part_id,
                    pose.fragment
                );
                // Fragment indices are allowed when produced by authored `split`
                // or by the exporter's auto-symmetry canonicalisation. The
                // `(part_id, fragment)` uniqueness check above is the real
                // invariant; this note documents why non-zero fragments can
                // appear on parts without an explicit `split` field.
                ensure!(
                    part_lookup.contains_key(pose.part_id.as_str()),
                    "animation '{}' frame {} references missing part '{}'",
                    animation.tag,
                    frame.source_frame,
                    pose.part_id
                );
                ensure!(
                    sprite_lookup.contains(pose.sprite_id.as_str()),
                    "animation '{}' frame {} references missing sprite '{}'",
                    animation.tag,
                    frame.source_frame,
                    pose.sprite_id
                );
                ensure!(
                    !(pose.visible && pose.opacity == 0),
                    "animation '{}' frame {} marks part '{}' visible with zero opacity",
                    animation.tag,
                    frame.source_frame,
                    pose.part_id
                );
                fragments_by_part
                    .entry(pose.part_id.as_str())
                    .or_default()
                    .push(pose.fragment);

                let mut parent_id = part_lookup
                    .get(pose.part_id.as_str())
                    .and_then(|part| part.parent_id.as_deref());
                while let Some(parent) = parent_id {
                    let parent_part = part_lookup.get(parent).expect("validated part graph");
                    if parent_part.source_layer.is_some() || parent_part.source_region.is_some() {
                        ensure!(
                            frame_part_fragments.iter().any(|(id, _)| *id == parent),
                            "animation '{}' frame {} renders child '{}' without visible parent '{}'",
                            animation.tag,
                            frame.source_frame,
                            pose.part_id,
                            parent
                        );
                    }
                    parent_id = parent_part.parent_id.as_deref();
                }
            }

            for (part_id, fragments) in &mut fragments_by_part {
                fragments.sort_unstable();
                ensure!(
                    fragments.first().copied() == Some(0),
                    "animation '{}' frame {} part '{}' is missing primary fragment 0",
                    animation.tag,
                    frame.source_frame,
                    part_id
                );
                for (expected, actual) in fragments.iter().enumerate() {
                    ensure!(
                        *actual == expected as u32,
                        "animation '{}' frame {} part '{}' has non-contiguous fragments: expected {} but found {}",
                        animation.tag,
                        frame.source_frame,
                        part_id,
                        expected,
                        actual
                    );
                }
            }

            for event in &frame.events {
                ensure!(
                    !event.id.trim().is_empty(),
                    "animation '{}' frame {} defines an event with an empty id",
                    animation.tag,
                    frame.source_frame
                );
                if let Some(part_id) = event.part_id.as_deref() {
                    ensure!(
                        part_lookup.contains_key(part_id),
                        "animation '{}' frame {} references missing event part '{}'",
                        animation.tag,
                        frame.source_frame,
                        part_id
                    );
                }
                ensure!(
                    frame_events.insert((event.kind, event.id.as_str(), event.part_id.as_deref())),
                    "animation '{}' frame {} defines event '{:?}:{}' more than once",
                    animation.tag,
                    frame.source_frame,
                    event.kind,
                    event.id
                );
            }
        }
    }

    Ok(())
}

fn bind_animation_events(
    aseprite: &AsepriteFile<'_>,
    parts: &[PartInstance],
    authored_events: &[CompositionAnimationEventSource],
) -> Result<HashMap<String, HashMap<usize, Vec<AnimationEvent>>>> {
    let frame_counts_by_tag: HashMap<&str, usize> = aseprite
        .tags()
        .iter()
        .map(|tag| (tag.name.as_str(), tag.range.len()))
        .collect();
    let part_ids: HashSet<&str> = parts.iter().map(|part| part.id.as_str()).collect();
    bind_animation_events_from_maps(&frame_counts_by_tag, &part_ids, authored_events)
}

fn bind_animation_events_from_maps<'a>(
    frame_counts_by_tag: &HashMap<&'a str, usize>,
    part_ids: &HashSet<&'a str>,
    authored_events: &[CompositionAnimationEventSource],
) -> Result<HashMap<String, HashMap<usize, Vec<AnimationEvent>>>> {
    let mut events_by_tag: HashMap<String, HashMap<usize, Vec<AnimationEvent>>> = HashMap::new();

    for event in authored_events {
        let Some(frame_count) = frame_counts_by_tag.get(event.tag.as_str()).copied() else {
            bail!(
                "animation event '{}' references missing tag '{}'",
                event.id,
                event.tag
            );
        };
        ensure!(
            event.frame < frame_count,
            "animation event '{}' references out-of-range frame {} for tag '{}' ({} frames)",
            event.id,
            event.frame,
            event.tag,
            frame_count
        );
        if let Some(part_id) = event.part_id.as_deref() {
            ensure!(
                part_ids.contains(part_id),
                "animation event '{}' references missing part '{}'",
                event.id,
                part_id
            );
        }

        let tag_events = events_by_tag.entry(event.tag.clone()).or_default();
        let frame_events = tag_events.entry(event.frame).or_default();
        ensure!(
            !frame_events.iter().any(|existing| {
                existing.kind == event.kind
                    && existing.id == event.id
                    && existing.part_id.as_deref() == event.part_id.as_deref()
            }),
            "animation event '{}' is duplicated on tag '{}' frame {}",
            event.id,
            event.tag,
            event.frame
        );
        frame_events.push(AnimationEvent {
            kind: event.kind,
            id: event.id.clone(),
            part_id: event.part_id.clone(),
            local_offset: event.local_offset,
        });
    }

    Ok(events_by_tag)
}

fn validate_part_graph(
    part_definitions: &[PartDefinition],
    parts: &[PartInstance],
    gameplay: &CompositionGameplay,
) -> Result<()> {
    let mut definition_ids = HashSet::new();
    for definition in part_definitions {
        ensure!(
            !definition.id.trim().is_empty(),
            "part definitions must not use empty ids"
        );
        ensure!(
            definition.id == normalize_part_id(&definition.id),
            "part definition id '{}' must already be snake_case",
            definition.id
        );
        ensure!(
            definition_ids.insert(definition.id.as_str()),
            "duplicate part definition id '{}'",
            definition.id
        );
        validate_tags(
            &definition.tags,
            &format!("part definition '{}'", definition.id),
        )?;
        validate_part_gameplay(
            &definition.gameplay,
            gameplay,
            &format!("part definition '{}'", definition.id),
        )?;
    }

    let mut part_ids = HashSet::new();
    let mut visual_draw_orders = HashSet::new();
    let mut source_layers = HashSet::new();
    let definition_lookup: HashMap<&str, &PartDefinition> = part_definitions
        .iter()
        .map(|part| (part.id.as_str(), part))
        .collect();

    for part in parts {
        ensure!(
            !part.id.trim().is_empty(),
            "part instances must not use empty ids"
        );
        ensure!(
            part.id == normalize_part_id(&part.id),
            "part instance id '{}' must already be snake_case",
            part.id
        );
        ensure!(
            part_ids.insert(part.id.as_str()),
            "duplicate part id '{}'",
            part.id
        );
        ensure!(
            definition_lookup.contains_key(part.definition_id.as_str()),
            "part '{}' references missing definition '{}'",
            part.id,
            part.definition_id
        );
        let definition = definition_lookup
            .get(part.definition_id.as_str())
            .expect("definition existence validated above");
        // split constraints: requires source_layer, incompatible with source_region.
        if part.split.is_some() {
            ensure!(
                part.source_layer.is_some(),
                "part '{}' has `split` without `source_layer`; split requires a whole-layer source",
                part.id
            );
            ensure!(
                part.source_region.is_none(),
                "part '{}' has both `split` and `source_region`; these are mutually exclusive",
                part.id
            );
        }
        let is_visual = part.source_layer.is_some() || part.source_region.is_some();
        if is_visual {
            ensure!(
                visual_draw_orders.insert(part.draw_order),
                "duplicate visual draw_order {}",
                part.draw_order
            );
        }
        // Whole-layer sources must be unique (1:1 mapping). Region-based
        // sources may share the same layer (e.g. left/right halves).
        if let Some(source_layer) = part.source_layer.as_deref() {
            ensure!(
                source_layers.insert(source_layer),
                "source layer '{source_layer}' is referenced by more than one part"
            );
        }
        validate_tags(&part.tags, &format!("part '{}'", part.id))?;
        let merged_gameplay = merged_part_gameplay(&definition.gameplay, &part.gameplay);
        validate_part_gameplay(&merged_gameplay, gameplay, &format!("part '{}'", part.id))?;
        if !is_visual {
            ensure!(
                part_gameplay_is_empty(&merged_gameplay),
                "non-visual part '{}' cannot carry gameplay metadata until transform-only semantic nodes are supported",
                part.id
            );
        }
    }

    let part_lookup: HashMap<&str, &PartInstance> =
        parts.iter().map(|part| (part.id.as_str(), part)).collect();
    for part in parts {
        if let Some(parent_id) = part.parent_id.as_deref() {
            let parent = part_lookup.get(parent_id).ok_or_else(|| {
                anyhow!(
                    "part '{}' references missing parent '{}'",
                    part.id,
                    parent_id
                )
            })?;
            ensure!(
                parent.id != part.id,
                "part '{}' cannot parent itself",
                part.id
            );
        }
    }

    build_part_topology(parts)?;
    let root_count = parts.iter().filter(|part| part.parent_id.is_none()).count();
    ensure!(
        root_count == 1,
        "composition must define exactly one root part"
    );
    validate_health_pools(gameplay)?;
    Ok(())
}

fn validate_health_pools(gameplay: &CompositionGameplay) -> Result<()> {
    let mut health_pools = HashSet::new();
    for pool in &gameplay.health_pools {
        ensure!(
            !pool.id.trim().is_empty(),
            "health pools must not use empty ids"
        );
        ensure!(
            pool.id == normalize_part_id(&pool.id),
            "health pool id '{}' must already be snake_case",
            pool.id
        );
        ensure!(
            health_pools.insert(pool.id.as_str()),
            "duplicate health pool id '{}'",
            pool.id
        );
        ensure!(
            pool.max_health > 0,
            "health pool '{}' must have max_health > 0",
            pool.id
        );
    }

    if let Some(entity_health_pool) = gameplay.entity_health_pool.as_deref() {
        ensure!(
            health_pools.contains(entity_health_pool),
            "entity_health_pool '{entity_health_pool}' must reference a declared health pool"
        );
    }

    Ok(())
}

fn validate_part_gameplay(
    gameplay: &PartGameplayMetadata,
    composition_gameplay: &CompositionGameplay,
    context: &str,
) -> Result<()> {
    if let Some(health_pool) = gameplay.health_pool.as_deref() {
        ensure!(
            composition_gameplay
                .health_pools
                .iter()
                .any(|pool| pool.id == health_pool),
            "{context} references missing health pool '{health_pool}'"
        );
    }
    if gameplay.targetable == Some(true) || !gameplay.collision.is_empty() {
        ensure!(
            gameplay.health_pool.is_some() || gameplay.durability.is_some(),
            "{context} must define a health_pool or durability when it is targetable or owns collision volumes"
        );
    }
    if gameplay.targetable == Some(true) {
        ensure!(
            !gameplay.collision.is_empty(),
            "{context} must define at least one collision volume when it is targetable"
        );
    }
    if !gameplay.collision.is_empty() {
        ensure!(
            gameplay.targetable == Some(true),
            "{context} must set targetable = true when it owns collision volumes; non-targetable or role-specific collision routing is not yet supported"
        );
    }
    if let Some(durability) = gameplay.durability {
        ensure!(durability > 0, "{context} must use durability > 0");
        ensure!(
            gameplay.targetable == Some(true),
            "{context} must set targetable = true when durability is defined"
        );
        ensure!(
            !gameplay.collision.is_empty(),
            "{context} must define collision volumes when durability is defined"
        );
    }
    if gameplay.breakable == Some(true) {
        ensure!(
            gameplay.durability.is_some(),
            "{context} must define durability when breakable = true"
        );
    }

    let mut collision_ids = HashSet::new();
    for collision in &gameplay.collision {
        ensure!(
            !collision.id.trim().is_empty(),
            "{context} defines a collision volume with an empty id"
        );
        ensure!(
            collision_ids.insert(collision.id.as_str()),
            "{} defines duplicate collision id '{}'",
            context,
            collision.id
        );
        validate_tags(
            &collision.tags,
            &format!("{context} collision '{}'", collision.id),
        )?;
        ensure!(
            collision.role == CollisionRole::Collider,
            "{} collision '{}' uses unsupported role '{:?}'; composed runtime currently supports only collider volumes",
            context,
            collision.id,
            collision.role
        );
        match &collision.shape {
            CollisionShape::Circle { radius, .. } => {
                ensure!(
                    *radius > 0.0,
                    "{context} collision '{}' must use radius > 0",
                    collision.id
                );
            }
            CollisionShape::Box { size, .. } => {
                ensure!(
                    size.x > 0.0 && size.y > 0.0,
                    "{context} collision '{}' must use a positive box size",
                    collision.id
                );
            }
        }
    }

    Ok(())
}

fn merged_part_gameplay(
    definition: &PartGameplayMetadata,
    instance: &PartGameplayMetadata,
) -> PartGameplayMetadata {
    PartGameplayMetadata {
        targetable: instance.targetable.or(definition.targetable),
        health_pool: instance
            .health_pool
            .clone()
            .or_else(|| definition.health_pool.clone()),
        armour: definition.armour.saturating_add(instance.armour),
        durability: instance.durability.or(definition.durability),
        breakable: instance.breakable.or(definition.breakable),
        pool_damage_ratio: instance.pool_damage_ratio.or(definition.pool_damage_ratio),
        collision: definition
            .collision
            .iter()
            .chain(instance.collision.iter())
            .cloned()
            .collect(),
    }
}

fn part_gameplay_is_empty(gameplay: &PartGameplayMetadata) -> bool {
    gameplay.targetable != Some(true)
        && gameplay.health_pool.is_none()
        && gameplay.armour == 0
        && gameplay.durability.is_none()
        && gameplay.breakable != Some(true)
        && gameplay.collision.is_empty()
}

fn validate_tags(tags: &[String], context: &str) -> Result<()> {
    let mut seen = HashSet::new();
    for tag in tags {
        ensure!(!tag.trim().is_empty(), "{context} contains an empty tag");
        ensure!(
            seen.insert(tag.as_str()),
            "{context} contains duplicate tag '{tag}'"
        );
    }

    Ok(())
}

fn validate_layer_bindings(
    parts: &[PartInstance],
    selected_layers: &[SelectedLayer<'_>],
) -> Result<()> {
    // Collect all referenced layer names (from source_layer and source_region).
    let bound_layers: HashSet<&str> = parts
        .iter()
        .filter_map(|part| {
            part.source_layer
                .as_deref()
                .or_else(|| part.source_region.as_ref().map(|r| r.layer.as_str()))
        })
        .collect();

    for layer in selected_layers {
        if !layer.visible {
            continue;
        }
        ensure!(
            bound_layers.contains(layer.name),
            "visible source layer '{}' is not referenced by composition metadata",
            layer.name
        );
    }

    for part in parts {
        let layer_name = part
            .source_layer
            .as_deref()
            .or_else(|| part.source_region.as_ref().map(|r| r.layer.as_str()));
        let Some(layer_name) = layer_name else {
            continue;
        };
        let layer = selected_layers
            .iter()
            .find(|candidate| candidate.name == layer_name)
            .ok_or_else(|| {
                anyhow!(
                    "part '{}' references missing source layer '{}'",
                    part.id,
                    layer_name
                )
            })?;
        ensure!(
            layer.visible,
            "part '{}' references hidden source layer '{}'",
            part.id,
            layer_name
        );
    }

    Ok(())
}

fn build_part_topology(parts: &[PartInstance]) -> Result<Vec<String>> {
    let part_lookup: HashMap<&str, &PartInstance> =
        parts.iter().map(|part| (part.id.as_str(), part)).collect();
    let mut ordered = Vec::with_capacity(parts.len());
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();

    for part in parts {
        visit_part(
            part.id.as_str(),
            &part_lookup,
            &mut visiting,
            &mut visited,
            &mut ordered,
        )?;
    }

    Ok(ordered)
}

fn visit_part(
    part_id: &str,
    part_lookup: &HashMap<&str, &PartInstance>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    ordered: &mut Vec<String>,
) -> Result<()> {
    if visited.contains(part_id) {
        return Ok(());
    }
    ensure!(
        visiting.insert(part_id.to_string()),
        "part hierarchy contains a cycle involving '{part_id}'"
    );

    let part = part_lookup
        .get(part_id)
        .ok_or_else(|| anyhow!("missing part '{part_id}' while building hierarchy"))?;
    if let Some(parent_id) = part.parent_id.as_deref() {
        visit_part(parent_id, part_lookup, visiting, visited, ordered)?;
    }

    visiting.remove(part_id);
    visited.insert(part_id.to_string());
    ordered.push(part_id.to_string());
    Ok(())
}

/// Crops a cel image to one half of the symmetry axis for virtual part splitting.
///
/// The centre column (at `canvas_origin_x`) is excluded from both halves,
/// guaranteeing equal dimensions so the interner can detect flip mirrors.
///
/// Returns `None` if the requested half has zero width (cel does not span it).
fn crop_to_split_half(
    cel_image: &RgbaImage,
    cel_origin_x: i16,
    canvas_origin_x: i32,
    half: SplitHalf,
) -> Option<(RgbaImage, i32)> {
    let center_local = canvas_origin_x - i32::from(cel_origin_x);
    let w = cel_image.width() as i32;
    let h = cel_image.height();

    let (start_x, region_w) = match half {
        SplitHalf::Left => {
            let end = center_local.min(w).max(0);
            (0i32, end)
        }
        SplitHalf::Right => {
            let start = (center_local + 1).max(0).min(w);
            (start, w - start)
        }
    };

    if region_w <= 0 || h == 0 {
        return None;
    }

    let cropped = imageops::crop_imm(cel_image, start_x as u32, 0, region_w as u32, h).to_image();
    let canvas_x = i32::from(cel_origin_x) + start_x;
    Some((cropped, canvas_x))
}

/// Tests whether a trimmed sprite is self-symmetric under horizontal flip.
///
/// Splits the image at the horizontal centre, excludes the centre column for
/// odd widths, and compares left half vs flipped right half pixel-by-pixel.
/// Returns `true` only on exact equality — no fuzzy matching.
fn is_self_symmetric_h(image: &RgbaImage) -> bool {
    let w = image.width() as i32;
    let h = image.height();
    if w < 2 || h == 0 {
        return false;
    }
    let cx = w / 2;
    let right_start = if w % 2 == 1 { cx + 1 } else { cx };
    let half_w = cx as u32;
    let right_w = (w - right_start) as u32;
    if half_w != right_w || half_w == 0 {
        return false;
    }
    // Compare left[x, y] == right[half_w - 1 - x, y] for all pixels.
    for y in 0..h {
        for x in 0..half_w {
            let left_px = image.get_pixel(x, y);
            let right_px = image.get_pixel(right_start as u32 + half_w - 1 - x, y);
            if left_px != right_px {
                return false;
            }
        }
    }
    true
}

fn centre_column_has_opaque(image: &RgbaImage, centre_x: u32) -> bool {
    image
        .enumerate_pixels()
        .any(|(x, _, pixel)| x == centre_x && pixel.0[3] > 0)
}

#[allow(clippy::too_many_arguments)]
fn emit_self_symmetric_fragments(
    placements: &mut HashMap<PlacementKey, RawPlacement>,
    part_id: &str,
    trimmed_image: &RgbaImage,
    world_x: i32,
    world_y: i32,
    origin: Point,
    sprite_interner: &mut SpriteInterner,
    opacity: u8,
) {
    let trimmed_w = trimmed_image.width() as i32;
    let trimmed_h = trimmed_image.height();
    let cx = trimmed_w / 2;
    let odd = trimmed_w % 2 == 1;

    let left = imageops::crop_imm(trimmed_image, 0, 0, cx as u32, trimmed_h).to_image();
    let half_usage = sprite_interner.intern(&left);

    let mut next_fragment = 0u32;
    placements.insert(
        (part_id.to_string(), next_fragment),
        RawPlacement {
            sprite_id: half_usage.sprite_id.clone(),
            top_left: Point {
                x: world_x - origin.x,
                y: world_y - origin.y,
            },
            flip_x: half_usage.flip_x,
            flip_y: half_usage.flip_y,
            opacity,
        },
    );
    next_fragment += 1;

    if odd && centre_column_has_opaque(trimmed_image, cx as u32) {
        let centre_col = imageops::crop_imm(trimmed_image, cx as u32, 0, 1, trimmed_h).to_image();
        let centre_usage = sprite_interner.intern(&centre_col);
        placements.insert(
            (part_id.to_string(), next_fragment),
            RawPlacement {
                sprite_id: centre_usage.sprite_id,
                top_left: Point {
                    x: (world_x + cx) - origin.x,
                    y: world_y - origin.y,
                },
                flip_x: centre_usage.flip_x,
                flip_y: centre_usage.flip_y,
                opacity,
            },
        );
        next_fragment += 1;
    }

    let right_offset_x = cx + i32::from(odd);
    placements.insert(
        (part_id.to_string(), next_fragment),
        RawPlacement {
            sprite_id: half_usage.sprite_id,
            top_left: Point {
                x: (world_x + right_offset_x) - origin.x,
                y: world_y - origin.y,
            },
            flip_x: !half_usage.flip_x,
            flip_y: half_usage.flip_y,
            opacity,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_authored_mirror_x_split_fragments(
    placements: &mut HashMap<PlacementKey, RawPlacement>,
    part: &PartInstance,
    cel_image: &RgbaImage,
    cel_origin_x: i16,
    cel_origin_y: i16,
    origin: Point,
    canvas_origin_x: i32,
    sprite_interner: &mut SpriteInterner,
    opacity: u8,
    _frame_index: usize,
    _layer_name: &str,
) -> Result<()> {
    let left_crop = crop_to_split_half(cel_image, cel_origin_x, canvas_origin_x, SplitHalf::Left);
    let right_crop = crop_to_split_half(cel_image, cel_origin_x, canvas_origin_x, SplitHalf::Right);
    let centre_local_x = canvas_origin_x - i32::from(cel_origin_x);
    let centre_column = if (0..cel_image.width() as i32).contains(&centre_local_x) {
        let centre = imageops::crop_imm(cel_image, centre_local_x as u32, 0, 1, cel_image.height())
            .to_image();
        trim_transparent_bounds(&centre)
    } else {
        None
    };
    if left_crop.is_none() && right_crop.is_none() && centre_column.is_none() {
        return Ok(());
    }

    let left_trimmed = left_crop.and_then(|(cropped, canvas_x)| {
        trim_transparent_bounds(&cropped).map(|(trimmed, bounds)| {
            let usage = sprite_interner.intern(&trimmed);
            let world_x = canvas_x + bounds.x as i32;
            let world_y = i32::from(cel_origin_y) + bounds.y as i32;
            RawPlacement {
                sprite_id: usage.sprite_id,
                top_left: Point {
                    x: world_x - origin.x,
                    y: world_y - origin.y,
                },
                flip_x: usage.flip_x,
                flip_y: usage.flip_y,
                opacity,
            }
        })
    });
    let right_trimmed = right_crop.and_then(|(cropped, canvas_x)| {
        trim_transparent_bounds(&cropped).map(|(trimmed, bounds)| {
            let usage = sprite_interner.intern(&trimmed);
            let world_x = canvas_x + bounds.x as i32;
            let world_y = i32::from(cel_origin_y) + bounds.y as i32;
            RawPlacement {
                sprite_id: usage.sprite_id,
                top_left: Point {
                    x: world_x - origin.x,
                    y: world_y - origin.y,
                },
                flip_x: usage.flip_x,
                flip_y: usage.flip_y,
                opacity,
            }
        })
    });
    let centre_trimmed = centre_column.map(|(trimmed_centre, centre_bounds)| {
        let usage = sprite_interner.intern(&trimmed_centre);
        let world_x = canvas_origin_x;
        let world_y = i32::from(cel_origin_y) + centre_bounds.y as i32;
        RawPlacement {
            sprite_id: usage.sprite_id,
            top_left: Point {
                x: world_x - origin.x,
                y: world_y - origin.y,
            },
            flip_x: usage.flip_x,
            flip_y: usage.flip_y,
            opacity,
        }
    });

    let mut next_fragment = 0u32;
    if let Some(left) = left_trimmed {
        placements.insert((part.id.clone(), next_fragment), left);
        next_fragment += 1;
    }
    if let Some(centre) = centre_trimmed {
        placements.insert((part.id.clone(), next_fragment), centre);
        next_fragment += 1;
    }
    if let Some(right) = right_trimmed {
        placements.insert((part.id.clone(), next_fragment), right);
    }

    Ok(())
}

/// Key for raw placements: `(part_id, fragment_index)`.
type PlacementKey = (String, u32);

fn build_raw_placements(
    parts: &[PartInstance],
    layer_lookup: &HashMap<&str, &SelectedLayer<'_>>,
    aseprite: &AsepriteFile<'_>,
    frame: &aseprite_loader::loader::Frame,
    frame_index: usize,
    origin: Point,
    sprite_interner: &mut SpriteInterner,
) -> Result<HashMap<PlacementKey, RawPlacement>> {
    let mut placements = HashMap::new();
    let raw_frame = aseprite
        .file
        .frames
        .get(frame_index)
        .ok_or_else(|| anyhow!("Missing raw frame {frame_index} while building placements"))?;

    for part in parts {
        // Resolve the layer name: either from source_layer or source_region.
        let (layer_name, split_half) = if let Some(ref name) = part.source_layer {
            (name.as_str(), None)
        } else if let Some(ref region) = part.source_region {
            (region.layer.as_str(), Some(region.half))
        } else {
            continue; // Non-visual marker part.
        };

        let layer = layer_lookup.get(layer_name).ok_or_else(|| {
            anyhow!(
                "part '{}' references missing source layer '{}'",
                part.id,
                layer_name
            )
        })?;
        let Some(frame_cel) = frame.cels.iter().find(|cel| cel.layer_index == layer.index) else {
            continue;
        };
        let raw_cel = raw_frame
            .cels
            .get(layer.index)
            .and_then(|cel| cel.as_ref())
            .ok_or_else(|| {
                anyhow!(
                    "Resolved frame cel missing matching raw cel at frame {} layer '{}'",
                    frame_index,
                    layer.name
                )
            })?;
        ensure!(
            raw_cel.z_index == 0,
            "Layer '{}' uses cel z-index {} in frame {}; dynamic z-order is not supported yet",
            layer.name,
            raw_cel.z_index,
            frame_index
        );

        let cel_image = load_cel_image(aseprite, frame_cel)?;
        let opacity = combine_opacity(layer.opacity, raw_cel.opacity);

        // Split parts are a visual-only decomposition around the composition
        // origin. The exporter preserves the authored left half, optional
        // centre strip, and authored right half exactly; it does not require
        // the layer itself to be self-symmetric.
        if part.split.is_some() {
            emit_authored_mirror_x_split_fragments(
                &mut placements,
                part,
                &cel_image,
                frame_cel.origin.0,
                frame_cel.origin.1,
                origin,
                origin.x,
                sprite_interner,
                opacity,
                frame_index,
                layer_name,
            )?;
            continue;
        }

        // For source_region parts, crop to the requested half before trimming.
        let (working_image, base_x) = if let Some(half) = split_half {
            let Some((cropped, canvas_x)) =
                crop_to_split_half(&cel_image, frame_cel.origin.0, origin.x, half)
            else {
                continue; // Cel does not span this half.
            };
            (cropped, canvas_x)
        } else {
            (cel_image, i32::from(frame_cel.origin.0))
        };

        let Some((trimmed_image, trimmed_bounds)) = trim_transparent_bounds(&working_image) else {
            continue;
        };
        let world_x = base_x + trimmed_bounds.x as i32;
        let world_y = i32::from(frame_cel.origin.1) + trimmed_bounds.y as i32;

        // Auto-canonicalisation: if the trimmed sprite is self-symmetric under
        // H-flip, store only the left half and reconstruct via mirror fragments.
        //
        // For odd-width sprites with a non-empty centre column, a 3rd fragment
        // preserves the centre strip that would otherwise be lost.
        //
        // Fragment layout:
        //   even width:              [left half] [mirrored right half]
        //   odd width, empty centre: [left half] [mirrored right half]
        //   odd width, filled centre: [left half] [centre strip] [mirrored right half]
        if is_self_symmetric_h(&trimmed_image) {
            emit_self_symmetric_fragments(
                &mut placements,
                &part.id,
                &trimmed_image,
                world_x,
                world_y,
                origin,
                sprite_interner,
                opacity,
            );
            continue;
        }

        let usage = sprite_interner.intern(&trimmed_image);

        placements.insert(
            (part.id.clone(), 0),
            RawPlacement {
                sprite_id: usage.sprite_id,
                top_left: Point {
                    x: world_x - origin.x,
                    y: world_y - origin.y,
                },
                flip_x: usage.flip_x,
                flip_y: usage.flip_y,
                opacity,
            },
        );
    }

    Ok(placements)
}

fn build_frame_poses(
    parts: &[PartInstance],
    raw_placements: &HashMap<PlacementKey, RawPlacement>,
    topo_order: &[String],
    frame_index: usize,
    animation_tag: &str,
) -> Result<Vec<PartPose>> {
    let part_lookup: HashMap<&str, &PartInstance> =
        parts.iter().map(|part| (part.id.as_str(), part)).collect();
    let mut absolute_pivots = HashMap::<String, Point>::new();
    let mut poses = Vec::new();

    for part_id in topo_order {
        let Some(part) = part_lookup.get(part_id.as_str()) else {
            continue;
        };

        // Collect all fragment indices for this part from the raw placements.
        let mut fragment_indices: Vec<u32> = raw_placements
            .keys()
            .filter(|(id, _)| id == &part.id)
            .map(|(_, frag)| *frag)
            .collect();
        fragment_indices.sort_unstable();

        if fragment_indices.is_empty() {
            continue;
        }

        ensure!(
            fragment_indices.first().copied() == Some(0),
            "animation '{}' frame {} part '{}' is missing primary fragment 0",
            animation_tag,
            frame_index,
            part.id
        );
        for (expected, actual) in fragment_indices.iter().enumerate() {
            ensure!(
                *actual == expected as u32,
                "animation '{}' frame {} part '{}' has non-contiguous fragments: expected {} but found {}",
                animation_tag,
                frame_index,
                part.id,
                expected,
                actual
            );
        }

        // Fragment 0 is the primary fragment; indices are contiguous from there.
        let primary_raw = raw_placements
            .get(&(part.id.clone(), fragment_indices[0]))
            .expect("fragment index collected from keys");

        let pivot_world = Point {
            x: primary_raw.top_left.x + part.pivot.x,
            y: primary_raw.top_left.y + part.pivot.y,
        };
        let local_offset = if part.parent_id.is_some() {
            let parent_pivot = resolve_parent_pivot_anchor(&part_lookup, part, &absolute_pivots)
                .with_context(|| {
                    format!(
                        "animation '{}' frame {} renders '{}' without visible parent chain",
                        animation_tag, frame_index, part.id
                    )
                })?;
            Point {
                x: pivot_world.x - parent_pivot.x,
                y: pivot_world.y - parent_pivot.y,
            }
        } else {
            pivot_world
        };

        absolute_pivots.insert(part.id.clone(), pivot_world);

        // Emit one pose per fragment.
        for &fragment in &fragment_indices {
            let raw = raw_placements
                .get(&(part.id.clone(), fragment))
                .expect("fragment index collected from keys");

            // For non-primary fragments, compute offset relative to primary.
            let frag_offset = if fragment == fragment_indices[0] {
                local_offset
            } else {
                let frag_pivot = Point {
                    x: raw.top_left.x + part.pivot.x,
                    y: raw.top_left.y + part.pivot.y,
                };
                if part.parent_id.is_some() {
                    let parent_pivot =
                        resolve_parent_pivot_anchor(&part_lookup, part, &absolute_pivots)
                            .with_context(|| {
                                format!(
                                    "animation '{}' frame {} renders '{}' fragment {} without visible parent chain",
                                    animation_tag, frame_index, part.id, fragment
                                )
                            })?;
                    Point {
                        x: frag_pivot.x - parent_pivot.x,
                        y: frag_pivot.y - parent_pivot.y,
                    }
                } else {
                    frag_pivot
                }
            };

            poses.push(PartPose {
                part_id: part.id.clone(),
                sprite_id: raw.sprite_id.clone(),
                local_offset: frag_offset,
                flip_x: raw.flip_x,
                flip_y: raw.flip_y,
                visible: true,
                opacity: raw.opacity,
                fragment,
            });
        }
    }

    poses.sort_by_key(|pose| {
        part_lookup
            .get(pose.part_id.as_str())
            .map_or(u32::MAX, |part| part.draw_order)
    });

    Ok(poses)
}

fn resolve_parent_pivot_anchor(
    part_lookup: &HashMap<&str, &PartInstance>,
    part: &PartInstance,
    absolute_pivots: &HashMap<String, Point>,
) -> Result<Point> {
    let mut parent_id = part.parent_id.as_deref();
    while let Some(current_parent_id) = parent_id {
        let parent = part_lookup
            .get(current_parent_id)
            .expect("validated hierarchy parent");
        if parent.source_layer.is_some() || parent.source_region.is_some() {
            return absolute_pivots
                .get(current_parent_id)
                .copied()
                .ok_or_else(|| {
                    anyhow!("missing resolved pivot for visual parent '{current_parent_id}'")
                });
        }
        parent_id = parent.parent_id.as_deref();
    }

    Ok(Point::default())
}

impl CompositionPartSource {
    fn to_instance(&self) -> PartInstance {
        PartInstance {
            id: self.id.clone(),
            definition_id: self.definition_id.clone(),
            name: self.name.clone().unwrap_or_else(|| self.id.clone()),
            parent_id: self.parent_id.clone(),
            source_layer: self.source_layer.clone(),
            source_region: self.source_region.clone(),
            split: self.split,
            draw_order: self.draw_order,
            pivot: self.pivot.unwrap_or_default(),
            tags: self.tags.clone(),
            visible_by_default: self.visible_by_default.unwrap_or(true),
            gameplay: self.gameplay.clone(),
        }
    }

    /// Whether this part is visual (has a source layer or region).
    fn is_visual(&self) -> bool {
        self.source_layer.is_some() || self.source_region.is_some()
    }
}

impl SpriteInterner {
    fn intern(&mut self, image: &RgbaImage) -> SpriteUsage {
        intern_sprite(&mut self.sprites, &mut self.cache, &mut self.stats, image)
    }
}

impl SpriteSpec {
    fn part_group_name(&self) -> &str {
        self.part_group
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(DEFAULT_PART_GROUP)
    }

    fn origin_slice_name(&self) -> &str {
        self.origin_slice
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(DEFAULT_ORIGIN_SLICE)
    }

    fn target_dir_path(&self) -> Result<PathBuf> {
        if let Some(target_dir) = self
            .target_dir
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(PathBuf::from(target_dir));
        }

        let source_path = Path::new(&self.source);
        let mut target_dir = PathBuf::from("sprites");
        if let Some(parent) = source_path.parent()
            && !parent.as_os_str().is_empty()
        {
            target_dir.push(parent);
        }
        Ok(target_dir)
    }

    fn composition_path(&self) -> Result<PathBuf> {
        if let Some(composition) = self
            .composition
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(PathBuf::from(composition));
        }

        let source_path = Path::new(&self.source);
        let stem = source_path
            .file_stem()
            .ok_or_else(|| anyhow!("Sprite source '{}' is missing a file stem", self.source))?;
        let mut composition_path = source_path
            .parent()
            .map_or_else(PathBuf::new, Path::to_path_buf);
        composition_path.push(format!("{}.composition.toml", stem.to_string_lossy()));
        Ok(composition_path)
    }
}

// ── Simple tagged-frame atlas export ──────────────────────────────────────

/// Request to build a simple tagged-frame sprite atlas from an aseprite file.
pub struct SimpleAtlasRequest {
    pub aseprite_path: PathBuf,
    pub output_dir: PathBuf,
    /// Asset-relative path for the PXI reference in the RON descriptor.
    pub pxi_asset_path: PathBuf,
}

/// Manifest entry for batch simple-atlas export.
#[derive(Clone, Debug, Deserialize)]
pub struct SimpleAtlasEntry {
    /// Aseprite source file, relative to the manifest directory.
    pub source: String,
    /// Asset-relative output directory (also used for PXI asset path).
    pub output: String,
}

/// Manifest for batch simple-atlas export.
#[derive(Debug, Deserialize)]
pub struct SimpleAtlasManifest {
    pub atlases: Vec<SimpleAtlasEntry>,
}

/// Export all simple atlases listed in a manifest file.
pub fn export_simple_atlas_manifest(manifest_path: &Path, assets_root: &Path) -> Result<()> {
    let body = fs::read_to_string(manifest_path)
        .with_context(|| format!("Failed to read {}", manifest_path.display()))?;
    let manifest: SimpleAtlasManifest = toml::from_str(&body)
        .with_context(|| format!("Failed to parse {}", manifest_path.display()))?;
    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));

    for entry in &manifest.atlases {
        let request = SimpleAtlasRequest {
            aseprite_path: manifest_dir.join(&entry.source),
            output_dir: assets_root.join(&entry.output),
            pxi_asset_path: PathBuf::from(format!("{}/atlas.pxi", entry.output)),
        };
        export_simple_atlas(&request)
            .with_context(|| format!("Failed to export '{}'", entry.source))?;
    }
    Ok(())
}

/// Build a simple sprite atlas: one region per tag, frames packed horizontally.
/// Frames are trimmed to their opaque bounding box (shared per tag).
pub fn export_simple_atlas(request: &SimpleAtlasRequest) -> Result<()> {
    let source_bytes = fs::read(&request.aseprite_path)
        .with_context(|| format!("Failed to read {}", request.aseprite_path.display()))?;
    let ase = AsepriteFile::load(&source_bytes)
        .with_context(|| format!("Failed to parse {}", request.aseprite_path.display()))?;

    let (w, h) = (u32::from(ase.size().0), u32::from(ase.size().1));
    let num_frames = ase.frames().len();

    // Flatten each frame (merge all visible layers).
    let mut flat_frames: Vec<RgbaImage> = Vec::new();
    for frame_idx in 0..num_frames {
        let frame = &ase.frames()[frame_idx];
        let mut merged = RgbaImage::new(w, h);
        for cel in &frame.cels {
            let layer = &ase.file.layers[cel.layer_index];
            if !layer.flags.contains(LayerFlags::VISIBLE) {
                continue;
            }
            if layer.layer_type != LayerType::Normal {
                continue;
            }
            let img = load_cel_image(&ase, cel)?;
            let (cx, cy) = (i32::from(cel.origin.0), i32::from(cel.origin.1));
            for py in 0..img.height() {
                for px in 0..img.width() {
                    let src = img.get_pixel(px, py);
                    if src.0[3] == 0 {
                        continue;
                    }
                    let dx = cx + px as i32;
                    let dy = cy + py as i32;
                    if dx >= 0 && dy >= 0 && (dx as u32) < w && (dy as u32) < h {
                        merged.put_pixel(dx as u32, dy as u32, *src);
                    }
                }
            }
        }
        flat_frames.push(merged);
    }

    // Group frames by tags, capturing animation metadata.
    #[allow(clippy::items_after_statements)]
    struct TagRegion {
        name: String,
        frames: Vec<usize>,
        direction: AnimationDirection,
        repeat: Option<u16>,
    }
    let mut tag_regions: Vec<TagRegion> = Vec::new();
    for tag in ase.tags() {
        let frames: Vec<usize> = tag.range.clone().map(usize::from).collect();
        tag_regions.push(TagRegion {
            name: tag.name.clone(),
            frames,
            direction: tag.direction,
            repeat: tag.repeat,
        });
    }
    if tag_regions.is_empty() {
        tag_regions.push(TagRegion {
            name: "default".to_string(),
            frames: (0..flat_frames.len()).collect(),
            direction: AnimationDirection::Forward,
            repeat: None,
        });
    }

    // Trim each tag's frames to the tightest shared bounding box.
    #[allow(clippy::items_after_statements)]
    struct PackedRegion {
        name: String,
        frame_size: (u32, u32),
        rects: Vec<(u32, u32, u32, u32)>,
    }
    let mut packed_regions: Vec<PackedRegion> = Vec::new();
    let mut atlas_strips: Vec<RgbaImage> = Vec::new();

    for tag in &tag_regions {
        let mut min_x = w;
        let mut min_y = h;
        let mut max_x = 0u32;
        let mut max_y = 0u32;
        for &fi in &tag.frames {
            for (px, py, pixel) in flat_frames[fi].enumerate_pixels() {
                if pixel.0[3] > 0 {
                    min_x = min_x.min(px);
                    min_y = min_y.min(py);
                    max_x = max_x.max(px + 1);
                    max_y = max_y.max(py + 1);
                }
            }
        }
        if max_x <= min_x || max_y <= min_y {
            continue;
        }
        let tw = max_x - min_x;
        let th = max_y - min_y;
        let num = tag.frames.len() as u32;

        let mut strip = RgbaImage::new(tw * num, th);
        for (i, &fi) in tag.frames.iter().enumerate() {
            let src = &flat_frames[fi];
            for y in 0..th {
                for x in 0..tw {
                    strip.put_pixel(i as u32 * tw + x, y, *src.get_pixel(min_x + x, min_y + y));
                }
            }
        }
        atlas_strips.push(strip);
        packed_regions.push(PackedRegion {
            name: tag.name.clone(),
            frame_size: (tw, th),
            rects: Vec::new(),
        });
    }

    let atlas_w = atlas_strips
        .iter()
        .map(image::ImageBuffer::width)
        .max()
        .unwrap_or(1);
    let atlas_h: u32 = atlas_strips.iter().map(image::ImageBuffer::height).sum();
    let mut atlas = RgbaImage::new(atlas_w, atlas_h);
    let mut y_cursor = 0u32;
    for (i, strip) in atlas_strips.iter().enumerate() {
        imageops::overlay(&mut atlas, strip, 0, i64::from(y_cursor));
        let region = &mut packed_regions[i];
        let (tw, _) = region.frame_size;
        let num = strip.width() / tw;
        for f in 0..num {
            region
                .rects
                .push((f * tw, y_cursor, region.frame_size.0, region.frame_size.1));
        }
        y_cursor += strip.height();
    }

    fs::create_dir_all(&request.output_dir)?;
    atlas
        .save(request.output_dir.join("atlas.png"))
        .context("Failed to save atlas PNG")?;

    let indices = compute_palette_indices(&atlas)?;
    let pxi_bytes = crate::pxi::encode_compressed(atlas.width(), atlas.height(), &indices)
        .context("Failed to encode PXI")?;
    fs::write(request.output_dir.join("atlas.pxi"), &pxi_bytes).context("Failed to write PXI")?;

    // Build animation metadata from tag timing.
    let mut animations = BTreeMap::new();
    for tag in &tag_regions {
        let total_ms: u64 = tag
            .frames
            .iter()
            .map(|&fi| u64::from(ase.frames()[fi].duration))
            .sum();
        let direction = match tag.direction {
            AnimationDirection::Reverse => "backward",
            _ => "forward",
        };
        let on_finish = match tag.repeat {
            None => "loop",
            Some(_) => "mark",
        };
        animations.insert(
            tag.name.clone(),
            RegionAnimationDescriptor {
                duration_ms: total_ms,
                direction: direction.to_string(),
                on_finish: on_finish.to_string(),
            },
        );
    }

    let descriptor = PxSpriteAtlasDescriptor {
        indexed_image: request.pxi_asset_path.clone(),
        regions: packed_regions
            .iter()
            .map(|r| AtlasRegionDescriptor {
                frame_size: [r.frame_size.0, r.frame_size.1],
                frames: r
                    .rects
                    .iter()
                    .map(|&(x, y, rw, rh)| AtlasRectDescriptor { x, y, w: rw, h: rh })
                    .collect(),
            })
            .collect(),
        names: packed_regions
            .iter()
            .enumerate()
            .map(|(i, r)| (r.name.clone(), i as u32))
            .collect(),
        animations,
    };
    let ron_path = request.output_dir.join("atlas.px_atlas.ron");
    let body = ron::ser::to_string_pretty(&descriptor, ron::ser::PrettyConfig::default())
        .context("Failed to serialize atlas RON")?;
    fs::write(&ron_path, format!("{body}\n")).context("Failed to write RON")?;

    println!(
        "Simple atlas: {} regions, {}x{} px",
        packed_regions.len(),
        atlas.width(),
        atlas.height(),
    );
    for r in &packed_regions {
        println!(
            "  {:16} {}x{} x{} frames",
            r.name,
            r.frame_size.0,
            r.frame_size.1,
            r.rects.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        Animation, AnimationEventKind, AnimationFrame, AtlasSprite, CollisionShape,
        CollisionVolume, CompositionAnimationEventSource, CompositionAtlas, CompositionGameplay,
        CompositionPartSource, CompositionSource, HealthPool, ImageKey, Manifest, PartDefinition,
        PartGameplayMetadata, PartInstance, PartPose, Point, Rect, Size, SplitMode, SpriteInterner,
        SpriteSpec, Vec2Value, emit_authored_mirror_x_split_fragments, image_key,
        normalize_part_id, trim_transparent_bounds, validate_composition_atlas,
        validate_composition_source, validate_manifest,
    };
    use crate::composed_ron::SpawnAnchorMode;
    use image::{ImageBuffer, Rgba, RgbaImage};
    use std::{collections::HashMap, path::PathBuf};

    fn sprite(entity: &str, depth: u8) -> SpriteSpec {
        SpriteSpec {
            source: format!("{entity}_{depth}.aseprite"),
            target_dir: None,
            entity: entity.to_string(),
            depth,
            palette: None,
            part_group: Some(String::from("base")),
            origin_slice: Some(String::from("origin")),
            composition: None,
        }
    }

    #[test]
    fn derives_target_dir_from_source_parent() {
        let sprite = SpriteSpec {
            source: "enemies/mosquiton_3.aseprite".to_string(),
            target_dir: None,
            entity: "mosquiton".to_string(),
            depth: 3,
            palette: None,
            part_group: None,
            origin_slice: None,
            composition: None,
        };

        assert_eq!(
            sprite.target_dir_path().unwrap(),
            PathBuf::from("sprites/enemies")
        );
    }

    #[test]
    fn derives_composition_sidecar_from_source_basename() {
        let sprite = SpriteSpec {
            source: "enemies/mosquiton_3.aseprite".to_string(),
            target_dir: None,
            entity: "mosquiton".to_string(),
            depth: 3,
            palette: None,
            part_group: None,
            origin_slice: None,
            composition: None,
        };

        assert_eq!(
            sprite.composition_path().unwrap(),
            PathBuf::from("enemies/mosquiton_3.composition.toml")
        );
    }

    fn minimal_composition() -> CompositionSource {
        CompositionSource {
            part_definitions: vec![PartDefinition {
                id: "body".to_string(),
                tags: vec!["core".to_string()],
                gameplay: PartGameplayMetadata {
                    targetable: Some(true),
                    health_pool: Some("core".to_string()),
                    collision: vec![CollisionVolume {
                        id: "body_hurtbox".to_string(),
                        role: super::CollisionRole::Collider,
                        shape: CollisionShape::Circle {
                            radius: 4.0,
                            offset: Vec2Value::default(),
                        },
                        tags: vec!["body".to_string()],
                    }],
                    ..Default::default()
                },
            }],
            parts: vec![CompositionPartSource {
                id: "body".to_string(),
                definition_id: "body".to_string(),
                name: Some("Body".to_string()),
                parent_id: None,
                source_layer: Some("body".to_string()),
                source_region: None,
                split: None,
                draw_order: 0,
                pivot: Some(Point::default()),
                tags: vec![],
                visible_by_default: Some(true),
                gameplay: PartGameplayMetadata::default(),
            }],
            animation_events: vec![],
            animation_overrides: vec![],
            gameplay: CompositionGameplay {
                entity_health_pool: Some("core".to_string()),
                health_pools: vec![HealthPool {
                    id: "core".to_string(),
                    max_health: 10,
                }],
            },
            spawn_anchor: SpawnAnchorMode::default(),
            ground_anchor_y: None,
            air_anchor_y: None,
        }
    }

    #[test]
    fn rejects_duplicate_entity_depth_entries() {
        let manifest = Manifest {
            sprites: vec![sprite("mosquito", 3), sprite("mosquito", 3)],
        };

        let error = validate_manifest(&manifest).unwrap_err().to_string();
        assert!(error.contains("Duplicate sprite entry"));
    }

    #[test]
    fn normalizes_part_names_to_snake_case() {
        assert_eq!(normalize_part_id("Wing R"), "wing_r");
        assert_eq!(normalize_part_id("Arm-L"), "arm_l");
        assert_eq!(normalize_part_id("__Head__"), "head");
    }

    #[test]
    fn trims_transparent_bounds() {
        let mut image = ImageBuffer::from_pixel(5, 4, Rgba([0, 0, 0, 0]));
        image.put_pixel(2, 1, Rgba([255, 255, 255, 255]));
        image.put_pixel(3, 2, Rgba([255, 255, 255, 255]));

        let (trimmed, bounds) = trim_transparent_bounds(&image).unwrap();
        assert_eq!(trimmed.width(), 2);
        assert_eq!(trimmed.height(), 2);
        assert_eq!(
            bounds,
            Rect {
                x: 2,
                y: 1,
                w: 2,
                h: 2
            }
        );
    }

    #[test]
    fn image_key_tracks_dimensions_and_pixels() {
        let image = ImageBuffer::from_pixel(2, 2, Rgba([1, 2, 3, 4]));
        let key = image_key(&image);
        assert_eq!(
            key,
            ImageKey {
                width: 2,
                height: 2,
                pixels: image.as_raw().clone(),
            }
        );
    }

    /// Verifies that centre-column exclusion during a symmetric split produces
    /// equal-width halves that the interner deduplicates via horizontal flip.
    ///
    /// Given a 5-wide horizontally symmetric image:
    ///   [A B | C | B A]      (C is the centre column)
    ///
    /// With centre exclusion:
    ///   left  = [A B]   (2 px)
    ///   right = [B A]   (2 px)
    ///   right == H-flip of left → interner returns same `sprite_id` with `flip_x`
    ///
    /// Without centre exclusion:
    ///   left  = [A B]   (2 px)
    ///   right = [C B A] (3 px)
    ///   Different widths → interner cannot match.
    #[test]
    fn centre_column_exclusion_enables_flip_dedup_for_symmetric_sprites() {
        let mut interner = SpriteInterner::default();

        // Build a 5x2 symmetric image: columns mirror around the centre (col 2).
        //   col:  0   1   2   3   4
        //         R   G   W   G   R    (row 0)
        //         B   Y   W   Y   B    (row 1)
        let red = Rgba([255, 0, 0, 255]);
        let green = Rgba([0, 255, 0, 255]);
        let white = Rgba([255, 255, 255, 255]);
        let blue = Rgba([0, 0, 255, 255]);
        let yellow = Rgba([255, 255, 0, 255]);

        let mut full = ImageBuffer::from_pixel(5, 2, Rgba([0, 0, 0, 0]));
        full.put_pixel(0, 0, red);
        full.put_pixel(1, 0, green);
        full.put_pixel(2, 0, white);
        full.put_pixel(3, 0, green);
        full.put_pixel(4, 0, red);
        full.put_pixel(0, 1, blue);
        full.put_pixel(1, 1, yellow);
        full.put_pixel(2, 1, white);
        full.put_pixel(3, 1, yellow);
        full.put_pixel(4, 1, blue);

        let mid = 2; // floor(5 / 2)

        // Split WITH centre-column exclusion: left = [0,2), right = [3,5).
        let left: RgbaImage = ImageBuffer::from_fn(mid as u32, 2, |x, y| *full.get_pixel(x, y));
        let right: RgbaImage =
            ImageBuffer::from_fn(mid as u32, 2, |x, y| *full.get_pixel(x + mid as u32 + 1, y));

        assert_eq!(left.width(), right.width(), "halves must have equal width");

        let usage_l = interner.intern(&left);
        let usage_r = interner.intern(&right);

        assert_eq!(
            usage_l.sprite_id, usage_r.sprite_id,
            "symmetric halves should share a sprite_id after centre exclusion"
        );
        assert!(!usage_l.flip_x, "left half should be canonical (no flip)");
        assert!(
            usage_r.flip_x,
            "right half should be detected as H-flip of left"
        );
        assert_eq!(
            interner.stats.new_sprites, 1,
            "only one unique sprite should be created"
        );
        assert_eq!(
            interner.stats.flip_x_hits, 1,
            "right half should register as a flip_x cache hit"
        );
    }

    /// Confirms that WITHOUT centre-column exclusion, equal-content halves
    /// of an odd-width sprite have different dimensions and cannot match.
    #[test]
    fn odd_width_split_without_exclusion_prevents_dedup() {
        let mut interner = SpriteInterner::default();

        // Same 5x2 symmetric image.
        let red = Rgba([255, 0, 0, 255]);
        let green = Rgba([0, 255, 0, 255]);
        let white = Rgba([255, 255, 255, 255]);

        let mut full = ImageBuffer::from_pixel(5, 1, Rgba([0, 0, 0, 0]));
        full.put_pixel(0, 0, red);
        full.put_pixel(1, 0, green);
        full.put_pixel(2, 0, white);
        full.put_pixel(3, 0, green);
        full.put_pixel(4, 0, red);

        let mid = 2;

        // Split WITHOUT exclusion: left = [0,2), right = [2,5) — includes centre.
        let left: RgbaImage = ImageBuffer::from_fn(mid as u32, 1, |x, y| *full.get_pixel(x, y));
        let right_with_centre: RgbaImage =
            ImageBuffer::from_fn(5 - mid as u32, 1, |x, y| *full.get_pixel(x + mid as u32, y));

        assert_ne!(
            left.width(),
            right_with_centre.width(),
            "without exclusion, halves have different widths"
        );

        let usage_l = interner.intern(&left);
        let usage_r = interner.intern(&right_with_centre);

        assert_ne!(
            usage_l.sprite_id, usage_r.sprite_id,
            "different-width halves cannot deduplicate"
        );
        assert_eq!(interner.stats.new_sprites, 2);
        assert_eq!(interner.stats.flip_x_hits, 0);
    }

    #[test]
    fn rejects_composition_cycles() {
        let mut composition = minimal_composition();
        composition.parts.push(CompositionPartSource {
            id: "head".to_string(),
            definition_id: "body".to_string(),
            name: Some("Head".to_string()),
            parent_id: Some("body".to_string()),
            source_layer: Some("head".to_string()),
            source_region: None,
            split: None,
            draw_order: 1,
            pivot: Some(Point::default()),
            tags: vec![],
            visible_by_default: Some(true),
            gameplay: PartGameplayMetadata::default(),
        });
        composition.parts[0].parent_id = Some("head".to_string());

        let error = validate_composition_source(&composition)
            .expect_err("cyclic hierarchies should be rejected")
            .to_string();
        assert!(error.contains("cycle"));
    }

    #[test]
    fn rejects_missing_health_pool_references() {
        let mut composition = minimal_composition();
        composition.part_definitions[0].gameplay.health_pool = Some("missing".to_string());

        let error = validate_composition_source(&composition)
            .expect_err("missing health pools should fail validation")
            .to_string();
        assert!(error.contains("missing health pool"));
    }

    #[test]
    fn rejects_non_visual_parts_with_gameplay_metadata() {
        let mut composition = minimal_composition();
        composition.part_definitions.push(PartDefinition {
            id: "marker".to_string(),
            tags: vec!["marker".to_string()],
            gameplay: PartGameplayMetadata {
                targetable: Some(true),
                health_pool: Some("core".to_string()),
                collision: vec![CollisionVolume {
                    id: "marker".to_string(),
                    role: super::CollisionRole::Collider,
                    shape: CollisionShape::Circle {
                        radius: 2.0,
                        offset: Vec2Value::default(),
                    },
                    tags: vec![],
                }],
                ..Default::default()
            },
        });
        composition.parts.push(CompositionPartSource {
            id: "marker".to_string(),
            definition_id: "marker".to_string(),
            name: Some("Marker".to_string()),
            parent_id: Some("body".to_string()),
            source_layer: None,
            source_region: None,
            split: None,
            draw_order: 1,
            pivot: None,
            tags: vec![],
            visible_by_default: Some(true),
            gameplay: PartGameplayMetadata::default(),
        });

        let error = validate_composition_source(&composition)
            .expect_err("non-visual gameplay-bearing nodes should be rejected")
            .to_string();
        assert!(error.contains("non-visual part"));
    }

    #[test]
    fn rejects_non_targetable_collision_volumes() {
        let mut composition = minimal_composition();
        composition.part_definitions[0].gameplay.targetable = Some(false);

        let error = validate_composition_source(&composition)
            .expect_err("collision volumes without targetable routing should fail")
            .to_string();
        assert!(error.contains("targetable = true"));
    }

    #[test]
    fn rejects_unsupported_collision_roles() {
        let mut composition = minimal_composition();
        composition.part_definitions[0].gameplay.collision[0].role = super::CollisionRole::Hurtbox;

        let error = validate_composition_source(&composition)
            .expect_err("unsupported collision roles should fail loudly")
            .to_string();
        assert!(error.contains("supports only collider volumes"));
    }

    #[test]
    fn rejects_breakable_parts_without_durability() {
        let mut composition = minimal_composition();
        composition.part_definitions[0].gameplay.breakable = Some(true);

        let error = validate_composition_source(&composition)
            .expect_err("breakable parts must define durability")
            .to_string();
        assert!(error.contains("breakable = true"));
    }

    #[test]
    fn rejects_durability_without_targetable_collision_contract() {
        let mut composition = minimal_composition();
        composition.part_definitions[0].gameplay.targetable = Some(false);
        composition.part_definitions[0].gameplay.durability = Some(5);

        let error = validate_composition_source(&composition)
            .expect_err("durability should require targetable collision routing")
            .to_string();
        assert!(error.contains("targetable = true"));
    }

    #[test]
    fn rejects_visual_parts_without_explicit_pivots() {
        let mut composition = minimal_composition();
        composition.parts[0].pivot = None;

        let error = validate_composition_source(&composition)
            .expect_err("visual parts must author pivots explicitly")
            .to_string();
        assert!(error.contains("explicit pivot"));
    }

    #[test]
    fn rejects_visible_by_default_false_until_runtime_supports_it() {
        let mut composition = minimal_composition();
        composition.parts[0].visible_by_default = Some(false);

        let error = validate_composition_source(&composition)
            .expect_err("visible_by_default false is currently unsupported")
            .to_string();
        assert!(error.contains("visible_by_default = false"));
    }

    #[test]
    fn rejects_animation_events_that_reference_missing_tags() {
        let composition = minimal_composition();
        let parts = composition
            .parts
            .iter()
            .map(CompositionPartSource::to_instance)
            .collect::<Vec<_>>();
        let source = vec![CompositionAnimationEventSource {
            tag: "missing".to_string(),
            frame: 0,
            kind: AnimationEventKind::ProjectileSpawn,
            id: "blood_shot".to_string(),
            part_id: Some("body".to_string()),
            local_offset: Point::default(),
        }];

        let error = bind_animation_events_for_tests([("idle", 1)], &parts, &source)
            .expect_err("missing tag should fail")
            .to_string();
        assert!(error.contains("missing tag"));
    }

    #[test]
    fn rejects_animation_events_with_out_of_range_frames() {
        let composition = minimal_composition();
        let parts = composition
            .parts
            .iter()
            .map(CompositionPartSource::to_instance)
            .collect::<Vec<_>>();
        let source = vec![CompositionAnimationEventSource {
            tag: "idle".to_string(),
            frame: 2,
            kind: AnimationEventKind::ProjectileSpawn,
            id: "blood_shot".to_string(),
            part_id: Some("body".to_string()),
            local_offset: Point::default(),
        }];

        let error = bind_animation_events_for_tests([("idle", 1)], &parts, &source)
            .expect_err("out of range frame should fail")
            .to_string();
        assert!(error.contains("out-of-range frame"));
    }

    #[test]
    fn rejects_animation_events_with_missing_parts() {
        let composition = minimal_composition();
        let parts = composition
            .parts
            .iter()
            .map(CompositionPartSource::to_instance)
            .collect::<Vec<_>>();
        let source = vec![CompositionAnimationEventSource {
            tag: "idle".to_string(),
            frame: 0,
            kind: AnimationEventKind::ProjectileSpawn,
            id: "blood_shot".to_string(),
            part_id: Some("missing".to_string()),
            local_offset: Point::default(),
        }];

        let error = bind_animation_events_for_tests([("idle", 1)], &parts, &source)
            .expect_err("missing part should fail")
            .to_string();
        assert!(error.contains("missing part"));
    }

    // -- T-6: Validation rejects split + source_region --

    #[test]
    fn rejects_split_with_source_region() {
        let mut comp = minimal_composition();
        // Clear source_layer so the source_layer-OR-source_region contract
        // doesn't fire first, then set both split and source_region.
        comp.parts[0].source_layer = None;
        comp.parts[0].split = Some(super::SplitMode::MirrorX);
        comp.parts[0].source_region = Some(super::SourceRegion {
            layer: "body".to_string(),
            half: super::SplitHalf::Left,
        });
        let error = validate_composition_source(&comp).unwrap_err().to_string();
        assert!(
            error.contains("split") && error.contains("source_region"),
            "expected split+source_region rejection, got: {error}"
        );
    }

    #[test]
    fn rejects_split_without_source_layer() {
        let mut comp = minimal_composition();
        comp.parts[0].source_layer = None;
        comp.parts[0].split = Some(super::SplitMode::MirrorX);
        let error = validate_composition_source(&comp).unwrap_err().to_string();
        assert!(
            error.contains("split") && error.contains("source_layer"),
            "expected split-without-source_layer rejection, got: {error}"
        );
    }

    // -- Self-symmetry detection tests --

    fn make_symmetric_image(w: u32, h: u32, fill_centre: bool) -> RgbaImage {
        let red = Rgba([255, 0, 0, 255]);
        let blue = Rgba([0, 0, 255, 255]);
        let green = Rgba([0, 255, 0, 255]);
        let transparent = Rgba([0, 0, 0, 0]);
        let cx = w / 2;
        let odd = w % 2 == 1;
        ImageBuffer::from_fn(w, h, |x, y| {
            if odd && x == cx {
                return if fill_centre { green } else { transparent };
            }
            // Map right-side pixels to their left-side mirror coordinate.
            let lx = if x < cx {
                x
            } else {
                // For odd width: right side starts at cx+1, mirror of cx+1 is cx-1, etc.
                // For even width: right side starts at cx, mirror of cx is cx-1, etc.
                let right_start = if odd { cx + 1 } else { cx };
                let dist_from_right_start = x - right_start;
                cx - 1 - dist_from_right_start
            };
            if (lx + y) % 2 == 0 { red } else { blue }
        })
    }

    fn make_split_part_instance() -> PartInstance {
        PartInstance {
            id: "legs_visual".to_string(),
            definition_id: "legs".to_string(),
            name: "Legs Visual".to_string(),
            parent_id: None,
            source_layer: Some("legs".to_string()),
            source_region: None,
            split: Some(SplitMode::MirrorX),
            draw_order: 0,
            pivot: Point::default(),
            tags: vec![],
            visible_by_default: true,
            gameplay: PartGameplayMetadata::default(),
        }
    }

    fn reconstruct_fragment_image(
        placements: &HashMap<(String, u32), super::RawPlacement>,
        interner: &SpriteInterner,
        expected_top_left: Point,
        width: u32,
        height: u32,
    ) -> RgbaImage {
        let mut reconstructed = RgbaImage::new(width, height);
        let mut fragments: Vec<_> = placements.iter().collect();
        fragments.sort_unstable_by_key(|((_, fragment), _)| *fragment);

        for ((_part_id, _fragment), placement) in fragments {
            let sprite = interner
                .sprites
                .iter()
                .find(|sprite| sprite.id == placement.sprite_id)
                .expect("placement sprite should exist in interner");
            let mut image = sprite.image.clone();
            if placement.flip_x {
                image = image::imageops::flip_horizontal(&image);
            }
            if placement.flip_y {
                image = image::imageops::flip_vertical(&image);
            }

            let draw_x = (placement.top_left.x - expected_top_left.x) as u32;
            let draw_y = (placement.top_left.y - expected_top_left.y) as u32;
            for y in 0..image.height() {
                for x in 0..image.width() {
                    let pixel = image.get_pixel(x, y);
                    if pixel.0[3] > 0 {
                        reconstructed.put_pixel(draw_x + x, draw_y + y, *pixel);
                    }
                }
            }
        }

        reconstructed
    }

    fn assert_authored_mirror_x_split_lossless(
        original: &RgbaImage,
        label: &str,
        expected_fragments: Option<Vec<u32>>,
    ) {
        let pad_left = 2u32;
        let pad_top = 1u32;
        let pad_right = 3u32;
        let pad_bottom = 2u32;
        let origin = Point { x: 20, y: 7 };
        let frame_origin_x = (origin.x - (pad_left as i32 + (original.width() / 2) as i32)) as i16;
        let frame_origin_y = 29i16;

        let mut cel = RgbaImage::new(
            original.width() + pad_left + pad_right,
            original.height() + pad_top + pad_bottom,
        );
        for y in 0..original.height() {
            for x in 0..original.width() {
                cel.put_pixel(x + pad_left, y + pad_top, *original.get_pixel(x, y));
            }
        }

        let (trimmed_image, _trimmed_bounds) =
            trim_transparent_bounds(&cel).expect("padded cel should trim back to authored image");
        assert_eq!(
            trimmed_image, *original,
            "{label}: trim path should preserve authored pixels before split emission"
        );

        let expected_top_left = Point {
            x: i32::from(frame_origin_x) + pad_left as i32 - origin.x,
            y: i32::from(frame_origin_y) + pad_top as i32 - origin.y,
        };
        let mut placements = HashMap::new();
        let mut interner = SpriteInterner::default();
        let part = make_split_part_instance();

        emit_authored_mirror_x_split_fragments(
            &mut placements,
            &part,
            &cel,
            frame_origin_x,
            frame_origin_y,
            origin,
            origin.x,
            &mut interner,
            255,
            0,
            "legs",
        )
        .expect("authored mirror_x split should emit fragments");

        let mut fragment_indices: Vec<_> = placements
            .keys()
            .map(|(_part_id, fragment)| *fragment)
            .collect();
        fragment_indices.sort_unstable();
        if let Some(expected_fragments) = expected_fragments {
            assert_eq!(
                fragment_indices, expected_fragments,
                "{label}: emitted fragment indices should be contiguous and start at 0"
            );
        }

        let reconstructed = reconstruct_fragment_image(
            &placements,
            &interner,
            expected_top_left,
            original.width(),
            original.height(),
        );

        for y in 0..original.height() {
            for x in 0..original.width() {
                assert_eq!(
                    original.get_pixel(x, y),
                    reconstructed.get_pixel(x, y),
                    "{label}: pixel mismatch at ({x},{y})"
                );
            }
        }
    }

    fn make_fragment_contract_atlas(frame_parts: Vec<PartPose>) -> CompositionAtlas {
        CompositionAtlas {
            schema_version: 3,
            entity: "fragment_contract".to_string(),
            depth: 1,
            source: "fragment_contract.aseprite".to_string(),
            canvas: Size { w: 16, h: 16 },
            origin: Point::default(),
            spawn_anchor: SpawnAnchorMode::default(),
            ground_anchor_y: None,
            air_anchor_y: None,
            atlas_image: "source.png".to_string(),
            part_definitions: vec![PartDefinition {
                id: "legs".to_string(),
                tags: vec![],
                gameplay: PartGameplayMetadata::default(),
            }],
            parts: vec![PartInstance {
                id: "legs_visual".to_string(),
                definition_id: "legs".to_string(),
                name: "Legs Visual".to_string(),
                parent_id: None,
                source_layer: Some("legs".to_string()),
                source_region: None,
                split: Some(SplitMode::MirrorX),
                draw_order: 0,
                pivot: Point::default(),
                tags: vec![],
                visible_by_default: true,
                gameplay: PartGameplayMetadata::default(),
            }],
            sprites: vec![
                AtlasSprite {
                    id: "legs_left".to_string(),
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 3,
                        h: 4,
                    },
                },
                AtlasSprite {
                    id: "legs_right".to_string(),
                    rect: Rect {
                        x: 3,
                        y: 0,
                        w: 3,
                        h: 4,
                    },
                },
            ],
            animations: vec![Animation {
                tag: "idle".to_string(),
                direction: "forward".to_string(),
                repeats: None,
                frames: vec![AnimationFrame {
                    source_frame: 0,
                    duration_ms: 100,
                    events: vec![],
                    parts: frame_parts,
                }],
                part_overrides: vec![],
            }],
            gameplay: CompositionGameplay {
                entity_health_pool: None,
                health_pools: vec![],
            },
        }
    }

    #[test]
    fn self_symmetry_detects_even_width() {
        let img = make_symmetric_image(8, 8, false);
        assert!(super::is_self_symmetric_h(&img));
    }

    #[test]
    fn self_symmetry_detects_odd_width_empty_centre() {
        let img = make_symmetric_image(9, 9, false);
        assert!(super::is_self_symmetric_h(&img));
    }

    #[test]
    fn self_symmetry_detects_odd_width_filled_centre() {
        let img = make_symmetric_image(9, 9, true);
        assert!(super::is_self_symmetric_h(&img));
    }

    #[test]
    fn self_symmetry_rejects_asymmetric() {
        let mut img = make_symmetric_image(8, 8, false);
        // Break symmetry by changing one pixel on the right half only.
        img.put_pixel(6, 3, Rgba([0, 255, 0, 255]));
        assert!(!super::is_self_symmetric_h(&img));
    }

    #[test]
    fn self_symmetry_rejects_near_miss() {
        let mut img = make_symmetric_image(10, 10, false);
        // 1 pixel different on the right side.
        img.put_pixel(8, 4, Rgba([128, 128, 128, 255]));
        assert!(!super::is_self_symmetric_h(&img));
    }

    #[test]
    fn self_symmetry_rejects_too_narrow() {
        let img: RgbaImage = ImageBuffer::from_fn(1, 5, |_, _| Rgba([255, 0, 0, 255]));
        assert!(!super::is_self_symmetric_h(&img));
    }

    /// Simulate the actual auto-canonicalisation pipeline path for a given
    /// symmetric image: detect symmetry, crop/intern fragments, then
    /// reconstruct from fragments and verify pixel-exact equivalence.
    fn assert_auto_canon_lossless(original: &RgbaImage, label: &str) {
        let w = original.width();
        let h = original.height();
        assert!(
            super::is_self_symmetric_h(original),
            "{label}: precondition — image must be self-symmetric"
        );

        let cx = w / 2;
        let odd = w % 2 == 1;
        let mut interner = SpriteInterner::default();

        // --- replicate the exact pipeline path in build_raw_placements ---
        // Left half
        let left = image::imageops::crop_imm(original, 0, 0, cx, h).to_image();
        let half_usage = interner.intern(&left);

        // Collect fragments as (sprite_image, render_x, flip_x)
        let mut fragments: Vec<(RgbaImage, u32, bool)> = Vec::new();

        // Fragment 0: left half
        let half_sprite = interner.sprites[0].image.clone();
        let f0_img = if half_usage.flip_x {
            image::imageops::flip_horizontal(&half_sprite)
        } else {
            half_sprite.clone()
        };
        fragments.push((f0_img, 0, false));

        // Centre strip (odd-width, if opaque pixels present)
        if odd {
            let centre_col = image::imageops::crop_imm(original, cx, 0, 1, h).to_image();
            let has_opaque = centre_col.pixels().any(|p| p.0[3] > 0);
            if has_opaque {
                let centre_usage = interner.intern(&centre_col);
                let idx = interner
                    .sprites
                    .iter()
                    .position(|s| s.id == centre_usage.sprite_id)
                    .expect("centre sprite must exist");
                let mut c_img = interner.sprites[idx].image.clone();
                if centre_usage.flip_x {
                    c_img = image::imageops::flip_horizontal(&c_img);
                }
                if centre_usage.flip_y {
                    c_img = image::imageops::flip_vertical(&c_img);
                }
                fragments.push((c_img, cx, false));
            }
        }

        // Mirrored right half
        let right_x = cx + u32::from(odd);
        let right_img = if half_usage.flip_x {
            half_sprite.clone()
        } else {
            image::imageops::flip_horizontal(&half_sprite)
        };
        fragments.push((right_img, right_x, true));

        // --- reconstruct from fragments ---
        let mut reconstructed = RgbaImage::new(w, h);
        for (frag_img, rx, _) in &fragments {
            for y in 0..frag_img.height() {
                for x in 0..frag_img.width() {
                    let px = frag_img.get_pixel(x, y);
                    if px.0[3] > 0 {
                        reconstructed.put_pixel(rx + x, y, *px);
                    }
                }
            }
        }

        // --- verify pixel-exact equivalence ---
        for y in 0..h {
            for x in 0..w {
                assert_eq!(
                    original.get_pixel(x, y),
                    reconstructed.get_pixel(x, y),
                    "{label}: pixel mismatch at ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn pixel_equivalence_even_width_auto_canon() {
        let img = make_symmetric_image(8, 6, false);
        assert_auto_canon_lossless(&img, "8x6 even");
    }

    #[test]
    fn pixel_equivalence_odd_width_empty_centre_auto_canon() {
        let img = make_symmetric_image(9, 7, false);
        assert_auto_canon_lossless(&img, "9x7 empty centre");
    }

    #[test]
    fn pixel_equivalence_odd_width_filled_centre_auto_canon() {
        let img = make_symmetric_image(9, 7, true);
        assert_auto_canon_lossless(&img, "9x7 filled centre");
    }

    #[test]
    fn pixel_equivalence_large_odd_filled_centre_auto_canon() {
        // Matches mosquiton head dimensions: 13px wide, odd, filled centre.
        let img = make_symmetric_image(13, 30, true);
        assert_auto_canon_lossless(&img, "13x30 filled centre");
    }

    #[test]
    fn pixel_equivalence_body_dimensions_auto_canon() {
        // Matches mosquiton body dimensions: 23px wide, odd, filled centre.
        let img = make_symmetric_image(23, 25, true);
        assert_auto_canon_lossless(&img, "23x25 filled centre");
    }

    #[test]
    fn pixel_equivalence_even_width_authored_mirror_x_split() {
        let img = make_symmetric_image(8, 6, false);
        assert_authored_mirror_x_split_lossless(&img, "8x6 even authored split", None);
    }

    #[test]
    fn pixel_equivalence_odd_width_empty_centre_authored_mirror_x_split() {
        let img = make_symmetric_image(9, 7, false);
        assert_authored_mirror_x_split_lossless(
            &img,
            "9x7 empty centre authored split",
            Some(vec![0, 1]),
        );
    }

    #[test]
    fn pixel_equivalence_odd_width_filled_centre_authored_mirror_x_split() {
        let img = make_symmetric_image(9, 7, true);
        assert_authored_mirror_x_split_lossless(
            &img,
            "9x7 filled centre authored split",
            Some(vec![0, 1, 2]),
        );
    }

    #[test]
    fn authored_mirror_x_split_preserves_asymmetric_cels_losslessly() {
        let mut img = make_symmetric_image(9, 7, true);
        img.put_pixel(7, 3, Rgba([255, 255, 255, 255]));

        assert_authored_mirror_x_split_lossless(&img, "9x7 asymmetric authored split", None);
    }

    #[test]
    fn validate_composition_atlas_rejects_missing_primary_fragment() {
        let atlas = make_fragment_contract_atlas(vec![PartPose {
            part_id: "legs_visual".to_string(),
            sprite_id: "legs_right".to_string(),
            local_offset: Point::default(),
            flip_x: true,
            flip_y: false,
            visible: true,
            opacity: 255,
            fragment: 1,
        }]);

        let error = validate_composition_atlas(&atlas)
            .expect_err("missing fragment 0 should be rejected")
            .to_string();
        assert!(error.contains("missing primary fragment 0"));
    }

    #[test]
    fn validate_composition_atlas_rejects_non_contiguous_fragment_indices() {
        let atlas = make_fragment_contract_atlas(vec![
            PartPose {
                part_id: "legs_visual".to_string(),
                sprite_id: "legs_left".to_string(),
                local_offset: Point { x: -3, y: 0 },
                flip_x: false,
                flip_y: false,
                visible: true,
                opacity: 255,
                fragment: 0,
            },
            PartPose {
                part_id: "legs_visual".to_string(),
                sprite_id: "legs_right".to_string(),
                local_offset: Point { x: 3, y: 0 },
                flip_x: true,
                flip_y: false,
                visible: true,
                opacity: 255,
                fragment: 2,
            },
        ]);

        let error = validate_composition_atlas(&atlas)
            .expect_err("fragment gaps should be rejected")
            .to_string();
        assert!(error.contains("non-contiguous fragments"));
    }

    #[test]
    fn validate_composition_atlas_rejects_duplicate_fragment_indices() {
        let atlas = make_fragment_contract_atlas(vec![
            PartPose {
                part_id: "legs_visual".to_string(),
                sprite_id: "legs_left".to_string(),
                local_offset: Point { x: -3, y: 0 },
                flip_x: false,
                flip_y: false,
                visible: true,
                opacity: 255,
                fragment: 0,
            },
            PartPose {
                part_id: "legs_visual".to_string(),
                sprite_id: "legs_right".to_string(),
                local_offset: Point { x: 3, y: 0 },
                flip_x: true,
                flip_y: false,
                visible: true,
                opacity: 255,
                fragment: 0,
            },
        ]);

        let error = validate_composition_atlas(&atlas)
            .expect_err("duplicate fragment indices should be rejected")
            .to_string();
        assert!(error.contains("fragment 0 more than once"));
    }

    #[test]
    fn symmetric_half_interns_as_single_sprite() {
        // A symmetric 8x4 image should intern its left half and detect flip.
        let img = make_symmetric_image(8, 4, false);
        let left = image::imageops::crop_imm(&img, 0, 0, 4, 4).to_image();
        let right = image::imageops::crop_imm(&img, 4, 0, 4, 4).to_image();

        let mut interner = SpriteInterner::default();
        let usage_left = interner.intern(&left);
        let usage_right = interner.intern(&right);

        assert_eq!(
            usage_left.sprite_id, usage_right.sprite_id,
            "symmetric halves should dedup to same sprite"
        );
        assert!(usage_right.flip_x, "right half should match via H-flip");
    }

    fn bind_animation_events_for_tests<'a>(
        tags: impl IntoIterator<Item = (&'a str, usize)>,
        parts: &[super::PartInstance],
        authored_events: &[CompositionAnimationEventSource],
    ) -> anyhow::Result<
        std::collections::HashMap<
            String,
            std::collections::HashMap<usize, Vec<super::AnimationEvent>>,
        >,
    > {
        let frame_counts_by_tag = tags
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let part_ids = parts
            .iter()
            .map(|part| part.id.as_str())
            .collect::<std::collections::HashSet<_>>();
        super::bind_animation_events_from_maps(&frame_counts_by_tag, &part_ids, authored_events)
    }
}
