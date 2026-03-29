//! Parser-based Aseprite export pipeline.
//!
//! This module turns one `.aseprite` source file into a piece atlas package:
//! a packed `atlas.png`, an engine-consumable `atlas.px_atlas.ron`, and an
//! `atlas.json` manifest describing how a runtime can compose animation frames
//! from deduplicated part sprites.

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
const RUNTIME_ATLAS_IMAGE_NAME: &str = "atlas.runtime.png";
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
    /// Stable snake_case semantic identifier.
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
    /// Stable snake_case instance identifier.
    pub id: String,
    /// Referenced reusable semantic definition.
    pub definition_id: String,
    /// Human-readable label for tooling/debugging.
    pub name: String,
    /// Optional parent instance id for hierarchical composition.
    pub parent_id: Option<String>,
    /// Optional source Aseprite layer used to author this visual node.
    pub source_layer: Option<String>,
    /// Draw order for this part within a composed frame.
    ///
    /// This belongs to the visual layer only. Validation enforces uniqueness
    /// only for parts with a bound `source_layer`.
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
struct CompositionSource {
    #[serde(default)]
    part_definitions: Vec<PartDefinition>,
    #[serde(default)]
    parts: Vec<CompositionPartSource>,
    #[serde(default)]
    animation_events: Vec<CompositionAnimationEventSource>,
    #[serde(default)]
    gameplay: CompositionGameplay,
}

#[derive(Debug, Deserialize)]
struct CompositionPartSource {
    id: String,
    definition_id: String,
    name: Option<String>,
    parent_id: Option<String>,
    source_layer: Option<String>,
    draw_order: u32,
    pivot: Option<Point>,
    #[serde(default)]
    tags: Vec<String>,
    visible_by_default: Option<bool>,
    #[serde(default)]
    gameplay: PartGameplayMetadata,
}

#[derive(Debug, Deserialize)]
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

/// Exports one piece-atlas package for the requested entity/depth pair.
///
/// The exporter reads the source `.aseprite` file directly, selects part layers
/// from the configured top-level group, resolves the shared origin from the
/// configured slice, deduplicates repeated or mirrored cel images, packs them
/// into `atlas.png`, and writes a matching `atlas.json` manifest.
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
    let atlas = build_piece_atlas(sprite, &aseprite, &composition)?;

    let output_dir = request
        .output_root
        .join(sprite.target_dir_path()?)
        .join(format!("{}_{}", sprite.entity, sprite.depth));
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create output directory {}", output_dir.display()))?;

    let atlas_png = output_dir.join("atlas.png");
    let runtime_atlas_png = output_dir.join(RUNTIME_ATLAS_IMAGE_NAME);
    let atlas_json = output_dir.join("atlas.json");
    atlas
        .0
        .save(&atlas_png)
        .with_context(|| format!("Failed to save {}", atlas_png.display()))?;
    quantize_runtime_atlas(&atlas.0, &runtime_atlas_png)?;
    let metadata = CompositionAtlas {
        schema_version: CURRENT_SCHEMA_VERSION,
        atlas_image: atlas_png
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
        ..atlas.1
    };
    let runtime_atlas_asset_image_path = sprite
        .target_dir_path()?
        .join(format!("{}_{}", sprite.entity, sprite.depth))
        .join(RUNTIME_ATLAS_IMAGE_NAME);
    write_px_atlas_metadata(&output_dir, &metadata, runtime_atlas_asset_image_path)?;
    fs::write(
        &atlas_json,
        format!("{}\n", serde_json::to_string_pretty(&metadata)?),
    )
    .with_context(|| format!("Failed to write {}", atlas_json.display()))?;

    println!(
        "Exported piece atlas for {} depth {} to {}",
        sprite.entity,
        sprite.depth,
        output_dir.display()
    );

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
pub fn default_manifest_path() -> PathBuf {
    PathBuf::from("resources/sprites/data.toml")
}

/// Returns the default output root used by the exporter CLI.
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
) -> Result<(RgbaImage, CompositionAtlas)> {
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

        animations.push(Animation {
            tag: tag.name.clone(),
            direction: animation_direction_name(tag.direction),
            repeats: tag.repeat.map(u32::from),
            frames,
        });
    }

    let (atlas_image, atlas_sprites) = pack_sprites(&sprite_interner.sprites)?;
    let metadata = CompositionAtlas {
        schema_version: CURRENT_SCHEMA_VERSION,
        entity: sprite.entity.clone(),
        depth: sprite.depth,
        source: sprite.source.clone(),
        canvas: Size {
            w: u32::from(aseprite.size().0),
            h: u32::from(aseprite.size().1),
        },
        origin,
        atlas_image: String::new(),
        part_definitions: composition.part_definitions.clone(),
        parts,
        sprites: atlas_sprites,
        animations,
        gameplay: composition.gameplay.clone(),
    };

    validate_composition_atlas(&metadata)?;

    Ok((atlas_image, metadata))
}

#[derive(Serialize)]
struct PxSpriteAtlasDescriptor {
    image: PathBuf,
    regions: Vec<AtlasRegionDescriptor>,
    #[serde(default)]
    names: BTreeMap<String, u32>,
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
    image_asset_path: PathBuf,
) -> Result<()> {
    let descriptor = PxSpriteAtlasDescriptor {
        image: image_asset_path,
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
    };

    let atlas_path = output_dir.join("atlas.px_atlas.ron");
    let body = ron::ser::to_string_pretty(&descriptor, ron::ser::PrettyConfig::default())
        .context("Failed to serialize atlas metadata")?;
    fs::write(&atlas_path, body)
        .with_context(|| format!("Failed to write {}", atlas_path.display()))?;
    Ok(())
}

/// Writes the runtime-only atlas image in a palette-safe form for `seldom_pixel`.
///
/// The semantic `atlas.png` must preserve authored colors. The runtime atlas is
/// a separate quantized image because `PxSpriteAtlasLoader` requires every
/// opaque pixel to exist in the configured project palette.
fn quantize_runtime_atlas(source: &RgbaImage, destination: &Path) -> Result<()> {
    let palette = load_runtime_palette(Path::new(DEFAULT_RUNTIME_PALETTE_PATH))?;
    let mut runtime_image = source.clone();
    let grayscale_mapping = grayscale_ramp_mapping(source, &palette);

    for pixel in runtime_image.pixels_mut() {
        if pixel.0[3] == 0 {
            continue;
        }
        *pixel = grayscale_mapping
            .get(&pixel.0)
            .copied()
            .unwrap_or_else(|| nearest_palette_color(*pixel, &palette));
    }

    runtime_image
        .save(destination)
        .with_context(|| format!("Failed to save {}", destination.display()))?;
    Ok(())
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
        || source_colors.iter().any(|pixel| !is_grayscale(pixel))
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

fn is_grayscale(pixel: &Rgba<u8>) -> bool {
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
        .ok_or_else(|| anyhow!("Missing part group '{}'", part_group))?;
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
        "Part group '{}' does not contain any direct child layers",
        part_group
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
        .ok_or_else(|| anyhow!("Missing required origin slice '{}'", slice_name))?;
    let first_key = slice
        .slice_keys
        .first()
        .ok_or_else(|| anyhow!("Origin slice '{}' does not contain any keys", slice_name))?;
    let first_pivot = first_key
        .pivot
        .ok_or_else(|| anyhow!("Origin slice '{}' must define a pivot", slice_name))?;

    for key in slice.slice_keys.iter().skip(1) {
        ensure!(
            key_matches_origin(first_key, key),
            "Origin slice '{}' varies across frames; this exporter requires one shared origin",
            slice_name
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
    image: &RgbaImage,
) -> SpriteUsage {
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
            return SpriteUsage {
                sprite_id: sprite_id.clone(),
                flip_x: *flip_x,
                flip_y: *flip_y,
            };
        }
    }

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
        .ok_or_else(|| {
            anyhow!(
                "No sprite entry found for entity '{}' at depth {}",
                entity,
                depth
            )
        })
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
        if part.source_layer.is_some() {
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

/// Validates an exported composed-asset manifest at the load boundary.
///
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
            let mut frame_parts = HashSet::new();
            let mut frame_events = HashSet::new();
            for pose in &frame.parts {
                ensure!(
                    frame_parts.insert(pose.part_id.as_str()),
                    "animation '{}' frame {} defines part '{}' more than once",
                    animation.tag,
                    frame.source_frame,
                    pose.part_id
                );
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

                let mut parent_id = part_lookup
                    .get(pose.part_id.as_str())
                    .and_then(|part| part.parent_id.as_deref());
                while let Some(parent) = parent_id {
                    let parent_part = part_lookup.get(parent).expect("validated part graph");
                    if parent_part.source_layer.is_some() {
                        ensure!(
                            frame_parts.contains(parent),
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
        if let Some(source_layer) = part.source_layer.as_deref() {
            ensure!(
                visual_draw_orders.insert(part.draw_order),
                "duplicate visual draw_order {}",
                part.draw_order
            );
            ensure!(
                source_layers.insert(source_layer),
                "source layer '{}' is referenced by more than one part",
                source_layer
            );
        }
        validate_tags(&part.tags, &format!("part '{}'", part.id))?;
        let merged_gameplay = merged_part_gameplay(&definition.gameplay, &part.gameplay);
        validate_part_gameplay(&merged_gameplay, gameplay, &format!("part '{}'", part.id))?;
        if part.source_layer.is_none() {
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
            "entity_health_pool '{}' must reference a declared health pool",
            entity_health_pool
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
            "{} references missing health pool '{}'",
            context,
            health_pool
        );
    }
    if gameplay.targetable == Some(true) || !gameplay.collision.is_empty() {
        ensure!(
            gameplay.health_pool.is_some() || gameplay.durability.is_some(),
            "{} must define a health_pool or durability when it is targetable or owns collision volumes",
            context
        );
    }
    if gameplay.targetable == Some(true) {
        ensure!(
            !gameplay.collision.is_empty(),
            "{} must define at least one collision volume when it is targetable",
            context
        );
    }
    if !gameplay.collision.is_empty() {
        ensure!(
            gameplay.targetable == Some(true),
            "{} must set targetable = true when it owns collision volumes; non-targetable or role-specific collision routing is not yet supported",
            context
        );
    }
    if let Some(durability) = gameplay.durability {
        ensure!(durability > 0, "{} must use durability > 0", context);
        ensure!(
            gameplay.targetable == Some(true),
            "{} must set targetable = true when durability is defined",
            context
        );
        ensure!(
            !gameplay.collision.is_empty(),
            "{} must define collision volumes when durability is defined",
            context
        );
    }
    if gameplay.breakable == Some(true) {
        ensure!(
            gameplay.durability.is_some(),
            "{} must define durability when breakable = true",
            context
        );
    }

    let mut collision_ids = HashSet::new();
    for collision in &gameplay.collision {
        ensure!(
            !collision.id.trim().is_empty(),
            "{} defines a collision volume with an empty id",
            context
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
        ensure!(!tag.trim().is_empty(), "{} contains an empty tag", context);
        ensure!(
            seen.insert(tag.as_str()),
            "{} contains duplicate tag '{}'",
            context,
            tag
        );
    }

    Ok(())
}

fn validate_layer_bindings(
    parts: &[PartInstance],
    selected_layers: &[SelectedLayer<'_>],
) -> Result<()> {
    let bound_layers: HashSet<&str> = parts
        .iter()
        .filter_map(|part| part.source_layer.as_deref())
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
        let Some(layer_name) = part.source_layer.as_deref() else {
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
        "part hierarchy contains a cycle involving '{}'",
        part_id
    );

    let part = part_lookup
        .get(part_id)
        .ok_or_else(|| anyhow!("missing part '{}' while building hierarchy", part_id))?;
    if let Some(parent_id) = part.parent_id.as_deref() {
        visit_part(parent_id, part_lookup, visiting, visited, ordered)?;
    }

    visiting.remove(part_id);
    visited.insert(part_id.to_string());
    ordered.push(part_id.to_string());
    Ok(())
}

fn build_raw_placements(
    parts: &[PartInstance],
    layer_lookup: &HashMap<&str, &SelectedLayer<'_>>,
    aseprite: &AsepriteFile<'_>,
    frame: &aseprite_loader::loader::Frame,
    frame_index: usize,
    origin: Point,
    sprite_interner: &mut SpriteInterner,
) -> Result<HashMap<String, RawPlacement>> {
    let mut placements = HashMap::new();
    let raw_frame = aseprite.file.frames.get(frame_index).ok_or_else(|| {
        anyhow!(
            "Missing raw frame {} while building placements",
            frame_index
        )
    })?;

    for part in parts {
        let Some(layer_name) = part.source_layer.as_deref() else {
            continue;
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
        let Some((trimmed_image, trimmed_bounds)) = trim_transparent_bounds(&cel_image) else {
            continue;
        };
        let usage = sprite_interner.intern(&trimmed_image);
        let world_x = i32::from(frame_cel.origin.0) + trimmed_bounds.x as i32;
        let world_y = i32::from(frame_cel.origin.1) + trimmed_bounds.y as i32;

        placements.insert(
            part.id.clone(),
            RawPlacement {
                sprite_id: usage.sprite_id,
                top_left: Point {
                    x: world_x - origin.x,
                    y: world_y - origin.y,
                },
                flip_x: usage.flip_x,
                flip_y: usage.flip_y,
                opacity: combine_opacity(layer.opacity, raw_cel.opacity),
            },
        );
    }

    Ok(placements)
}

fn build_frame_poses(
    parts: &[PartInstance],
    raw_placements: &HashMap<String, RawPlacement>,
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
        let Some(raw) = raw_placements.get(part.id.as_str()) else {
            continue;
        };

        let pivot_world = Point {
            x: raw.top_left.x + part.pivot.x,
            y: raw.top_left.y + part.pivot.y,
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
        poses.push(PartPose {
            part_id: part.id.clone(),
            sprite_id: raw.sprite_id.clone(),
            local_offset,
            flip_x: raw.flip_x,
            flip_y: raw.flip_y,
            visible: true,
            opacity: raw.opacity,
        });
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
        if parent.source_layer.is_some() {
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
            draw_order: self.draw_order,
            pivot: self.pivot.unwrap_or_default(),
            tags: self.tags.clone(),
            visible_by_default: self.visible_by_default.unwrap_or(true),
            gameplay: self.gameplay.clone(),
        }
    }
}

impl SpriteInterner {
    fn intern(&mut self, image: &RgbaImage) -> SpriteUsage {
        intern_sprite(&mut self.sprites, &mut self.cache, image)
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

#[cfg(test)]
mod tests {
    use super::{
        AnimationEventKind, CollisionShape, CollisionVolume, CompositionAnimationEventSource,
        CompositionGameplay, CompositionPartSource, CompositionSource, HealthPool, ImageKey,
        Manifest, PartDefinition, PartGameplayMetadata, Point, Rect, SpriteSpec, Vec2Value,
        image_key, normalize_part_id, trim_transparent_bounds, validate_composition_source,
        validate_manifest,
    };
    use image::{ImageBuffer, Rgba};
    use std::path::PathBuf;

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
                draw_order: 0,
                pivot: Some(Point::default()),
                tags: vec![],
                visible_by_default: Some(true),
                gameplay: PartGameplayMetadata::default(),
            }],
            animation_events: vec![],
            gameplay: CompositionGameplay {
                entity_health_pool: Some("core".to_string()),
                health_pools: vec![HealthPool {
                    id: "core".to_string(),
                    max_health: 10,
                }],
            },
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

    #[test]
    fn rejects_composition_cycles() {
        let mut composition = minimal_composition();
        composition.parts.push(CompositionPartSource {
            id: "head".to_string(),
            definition_id: "body".to_string(),
            name: Some("Head".to_string()),
            parent_id: Some("body".to_string()),
            source_layer: Some("head".to_string()),
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
