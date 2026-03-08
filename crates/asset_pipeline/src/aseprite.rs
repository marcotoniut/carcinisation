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
    collections::{BTreeMap, HashMap},
    fs,
    path::{Path, PathBuf},
};

const DEFAULT_PART_GROUP: &str = "base";
const DEFAULT_ORIGIN_SLICE: &str = "origin";
const CURRENT_SCHEMA_VERSION: u32 = 1;
const BASE_PALETTE_PATH: &str = "assets/palette/base.png";

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
    /// Output directory relative to the exporter output root.
    pub target_dir: String,
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
    /// Shared composition origin used as the offset reference for all parts.
    pub origin: Point,
    /// Atlas image filename stored alongside this manifest.
    pub atlas_image: String,
    /// Exported parts in draw order.
    pub parts: Vec<PartDefinition>,
    /// Deduplicated sprite rectangles packed into the atlas image.
    pub sprites: Vec<AtlasSprite>,
    /// Animation tags and their per-frame composition data.
    pub animations: Vec<Animation>,
}

/// One canonical part exported from the source file.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartDefinition {
    /// Stable snake_case part identifier derived from the layer name.
    pub id: String,
    /// Original Aseprite layer name.
    pub name: String,
    /// Draw order for this part within a composed frame.
    pub draw_order: u32,
}

/// One deduplicated sprite image packed into the atlas.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AtlasSprite {
    /// Stable sprite identifier referenced by [`PartPlacement`].
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
    /// Per-part placements needed to compose this frame.
    pub parts: Vec<PartPlacement>,
}

/// One part placement in a composed frame.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartPlacement {
    /// Part identifier matching [`PartDefinition::id`].
    pub part_id: String,
    /// Sprite identifier matching [`AtlasSprite::id`].
    pub sprite_id: String,
    /// Offset from [`CompositionAtlas::origin`] in source canvas pixels.
    pub offset: Point,
    /// Whether the runtime should mirror the sprite horizontally.
    pub flip_x: bool,
    /// Whether the runtime should mirror the sprite vertically.
    pub flip_y: bool,
    /// Final opacity after combining layer opacity and cel opacity.
    pub opacity: u8,
}

/// Two-dimensional integer point in source canvas pixels.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy)]
struct SelectedLayer<'a> {
    index: usize,
    name: &'a str,
    opacity: u8,
    visible: bool,
}

#[derive(Debug, Clone)]
struct PreparedSprite {
    id: String,
    image: RgbaImage,
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
    let atlas = build_piece_atlas(sprite, &aseprite)?;

    let output_dir = request
        .output_root
        .join(&sprite.target_dir)
        .join(format!("{}_{}", sprite.entity, sprite.depth));
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create output directory {}", output_dir.display()))?;

    let atlas_png = output_dir.join("atlas.png");
    let atlas_json = output_dir.join("atlas.json");
    let palette_image = load_base_palette()?;
    let atlas_image = reduce_to_palette(&palette_image, &atlas.0);
    atlas_image
        .save(&atlas_png)
        .with_context(|| format!("Failed to save {}", atlas_png.display()))?;
    let metadata = CompositionAtlas {
        schema_version: CURRENT_SCHEMA_VERSION,
        atlas_image: atlas_png
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
        ..atlas.1
    };
    let atlas_asset_image_path = PathBuf::from(&sprite.target_dir)
        .join(format!("{}_{}", sprite.entity, sprite.depth))
        .join(&metadata.atlas_image);
    write_px_atlas_metadata(&output_dir, &metadata, atlas_asset_image_path)?;
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
/// each sprite entry to define `source`, `target_dir`, and `entity`.
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

fn build_piece_atlas(
    sprite: &SpriteSpec,
    aseprite: &AsepriteFile<'_>,
) -> Result<(RgbaImage, CompositionAtlas)> {
    ensure!(
        !aseprite.tags().is_empty(),
        "Aseprite file '{}' does not define any animation tags",
        sprite.source
    );

    let selected_layers = select_part_layers(aseprite, sprite.part_group_name())?;
    let origin = resolve_origin(aseprite, sprite.origin_slice_name())?;
    let mut part_id_map = HashMap::new();
    let mut draw_order_map = HashMap::new();
    let mut parts = Vec::new();

    for layer in &selected_layers {
        // Invisible direct children of the part group do not participate in the
        // exported composition.
        if !layer.visible {
            continue;
        }
        let part_id = normalize_part_id(layer.name);
        if let Some(previous) = part_id_map.insert(part_id.clone(), layer.name.to_string()) {
            bail!(
                "Layer names '{}' and '{}' both normalize to part id '{}'",
                previous,
                layer.name,
                part_id
            );
        }
        if let Some(previous) = draw_order_map.insert(layer.index as u32, layer.name.to_string()) {
            bail!(
                "Layers '{}' and '{}' both use draw_order {}",
                previous,
                layer.name,
                layer.index
            );
        }
        parts.push(PartDefinition {
            id: part_id,
            name: layer.name.to_string(),
            draw_order: layer.index as u32,
        });
    }
    ensure!(
        !parts.is_empty(),
        "Aseprite file '{}' has no visible parts in group '{}'",
        sprite.source,
        sprite.part_group_name()
    );

    let part_lookup: HashMap<&str, &PartDefinition> = parts
        .iter()
        .map(|part| (part.name.as_str(), part))
        .collect();
    let mut sprite_cache: HashMap<ImageKey, String> = HashMap::new();
    let mut prepared_sprites: Vec<PreparedSprite> = Vec::new();
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
            let raw_frame = aseprite.file.frames.get(frame_index).ok_or_else(|| {
                anyhow!("Missing raw frame {} for tag '{}'", frame_index, tag.name)
            })?;
            let mut placements = Vec::new();

            for layer in &selected_layers {
                // Invisible layers are excluded at export time instead of
                // emitting permanently hidden parts.
                if !layer.visible {
                    continue;
                }
                let part = part_lookup.get(layer.name).ok_or_else(|| {
                    anyhow!("Visible layer '{}' is missing from part lookup", layer.name)
                })?;
                // Missing cels are intentional: the part is invisible for this
                // authored frame.
                let Some(frame_cel) = frame.cels.iter().find(|cel| cel.layer_index == layer.index)
                else {
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
                // Fully transparent cels do not produce atlas sprites.
                let Some((trimmed_image, trimmed_bounds)) = trim_transparent_bounds(&cel_image)
                else {
                    continue;
                };
                let usage = intern_sprite(&mut prepared_sprites, &mut sprite_cache, &trimmed_image);
                let world_x = i32::from(frame_cel.origin.0) + trimmed_bounds.x as i32;
                let world_y = i32::from(frame_cel.origin.1) + trimmed_bounds.y as i32;
                placements.push(PartPlacement {
                    part_id: part.id.clone(),
                    sprite_id: usage.sprite_id,
                    offset: Point {
                        x: world_x - origin.x,
                        y: world_y - origin.y,
                    },
                    flip_x: usage.flip_x,
                    flip_y: usage.flip_y,
                    opacity: combine_opacity(layer.opacity, raw_cel.opacity),
                });
            }

            placements.sort_by_key(|placement| {
                parts
                    .iter()
                    .find(|part| part.id == placement.part_id)
                    .map_or(u32::MAX, |part| part.draw_order)
            });
            frames.push(AnimationFrame {
                source_frame: frame_index,
                duration_ms: u32::from(frame.duration),
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

    let (atlas_image, atlas_sprites) = pack_sprites(&prepared_sprites)?;
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
        parts,
        sprites: atlas_sprites,
        animations,
    };

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

fn load_base_palette() -> Result<RgbaImage> {
    let image = image::open(BASE_PALETTE_PATH)
        .with_context(|| format!("Failed to open base palette at {BASE_PALETTE_PATH}"))?
        .to_rgba8();
    Ok(image)
}

fn reduce_to_palette(
    palette_image: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    image: &RgbaImage,
) -> RgbaImage {
    let palette: Vec<[u8; 3]> = palette_image
        .pixels()
        .filter(|pixel| pixel[3] != 0)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    assert!(
        !palette.is_empty(),
        "base palette must contain at least one opaque color"
    );

    let mut output = image.clone();
    for pixel in output.pixels_mut() {
        if pixel[3] == 0 {
            continue;
        }
        let closest = find_closest_palette_color(&palette, [pixel[0], pixel[1], pixel[2]]);
        *pixel = Rgba([closest[0], closest[1], closest[2], pixel[3]]);
    }

    output
}

fn find_closest_palette_color(palette: &[[u8; 3]], color: [u8; 3]) -> &[u8; 3] {
    palette
        .iter()
        .min_by_key(|candidate| color_distance_sq(**candidate, color))
        .expect("palette must contain at least one color")
}

fn color_distance_sq(a: [u8; 3], b: [u8; 3]) -> u32 {
    let dr = i32::from(a[0]) - i32::from(b[0]);
    let dg = i32::from(a[1]) - i32::from(b[1]);
    let db = i32::from(a[2]) - i32::from(b[2]);
    (dr * dr + dg * dg + db * db) as u32
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
        ensure!(
            !sprite.target_dir.trim().is_empty(),
            "Sprite entry {} is missing a target_dir",
            index + 1
        );
        ensure!(
            !sprite.entity.trim().is_empty(),
            "Sprite entry {} is missing an entity name",
            index + 1
        );
    }

    Ok(())
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
}

#[cfg(test)]
mod tests {
    use super::{
        ImageKey, Manifest, Rect, SpriteSpec, image_key, normalize_part_id,
        trim_transparent_bounds, validate_manifest,
    };
    use image::{ImageBuffer, Rgba};

    fn sprite(entity: &str, depth: u8) -> SpriteSpec {
        SpriteSpec {
            source: format!("{entity}_{depth}.aseprite"),
            target_dir: String::from("sprites/enemies"),
            entity: entity.to_string(),
            depth,
            palette: None,
            part_group: Some(String::from("base")),
            origin_slice: Some(String::from("origin")),
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
}
