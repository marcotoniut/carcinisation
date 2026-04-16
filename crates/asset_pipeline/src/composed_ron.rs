//! Compact runtime composed-atlas manifest (RON).
//!
//! Replaces the verbose JSON manifest at runtime. JSON is kept for debug/tooling.
//!
//! # What changed vs JSON
//!
//! - String IDs → u8 indices (parts, sprites), with string tables for lookup.
//! - `opacity` dropped — must be 255 for every pose; encoder rejects otherwise.
//! - `visible` dropped — must be true for every pose; encoder rejects otherwise.
//! - Offsets narrowed to i8 (−128..127), durations to u16, indices to u8.
//!   All narrowing is bounds-checked at encode time; overflow fails the export.
//! - Direction strings → enum. Enums for closed sets (collision role, event kind).
//!
//! # Why RON, not binary
//!
//! RON deserialises directly into typed Rust structs via serde. Same schema can
//! be re-encoded to a compact binary format later by swapping only the codec.
//! Runtime validation rejects malformed compact tables explicitly rather than
//! relying on unchecked indexing or silent truncation.
//!
//! # Future evolution
//!
//! If `opacity` or `visible` need per-pose values, add them back to [`CompactPose`]
//! and bump the format (add a version field or a new file extension).

use anyhow::{Result, anyhow, bail, ensure};
use serde::{Deserialize, Serialize};

use crate::aseprite::{AnimationEventKind, CollisionRole, CollisionShape, CompositionAtlas};

// ── Compact types ──────────────────────────────────────────────────────────

/// Top-level compact composed-atlas manifest.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactComposedAtlas {
    /// Entity identifier (e.g. "mosquiton") — kept for error logging.
    pub entity: String,
    /// Depth variant — kept for error logging.
    pub depth: u8,
    /// Canvas size in pixels `(w, h)`.
    pub canvas: (u16, u16),
    /// World anchor `(x, y)`.
    pub origin: (i16, i16),
    /// How the entity position maps to the sprite. Defaults to `BottomOrigin`
    /// (feet/ground contact) for backwards compatibility.
    #[serde(default, skip_serializing_if = "SpawnAnchorMode::is_default")]
    pub spawn_anchor: SpawnAnchorMode,
    /// Y offset from composition origin to ground contact point, in canvas
    /// Y-down pixels (positive = below origin).  When `None`, the runtime
    /// falls back to the legacy proxy: `canvas_height − origin_y`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground_anchor_y: Option<i16>,
    /// Y offset from composition origin to airborne pivot, in canvas Y-down
    /// pixels.  `None` defaults to `0` (at origin / body centre).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub air_anchor_y: Option<i16>,
    /// Part string table. Index into this with `u8` part indices.
    pub part_names: Vec<String>,
    /// Sprite string table. Index into this with `u8` sprite indices.
    pub sprite_names: Vec<String>,
    /// Sprite sizes `(w, h)` in atlas order (index = sprite id).
    pub sprite_sizes: Vec<(u16, u16)>,
    /// Merged part definitions + instances.
    pub parts: Vec<CompactPart>,
    /// Animation sequences.
    pub animations: Vec<CompactAnimation>,
    /// Gameplay metadata.
    pub gameplay: CompactGameplay,
}

/// Merged part definition + instance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactPart {
    /// Index into `part_names`.
    pub id: u8,
    /// Parent part index, or `None` for root.
    pub parent: Option<u8>,
    /// Whether this part has visual sprites.
    pub visual: bool,
    /// Draw order (lower = behind).
    pub draw_order: u8,
    /// Local pivot `(x, y)`.
    pub pivot: (i16, i16),
    /// Merged semantic tags from definition + instance.
    pub tags: Vec<String>,
    /// Gameplay metadata.
    pub gameplay: CompactPartGameplay,
}

/// Per-part gameplay metadata (merged definition + instance).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompactPartGameplay {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub targetable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_pool: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub armour: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub durability: Option<u8>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub breakable: bool,
    /// Fraction of adjusted damage forwarded to the health pool each hit,
    /// regardless of durability absorption. `None` = pool only receives
    /// overflow after durability is depleted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_damage_ratio: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collisions: Vec<CompactCollision>,
}

/// Compact collision volume.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactCollision {
    pub role: CollisionRole,
    pub shape: CollisionShape,
}

/// One animation sequence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactAnimation {
    /// Animation tag name (e.g. "idle_stand", "shoot_fly").
    pub tag: String,
    /// Playback direction.
    pub direction: CompactDirection,
    /// Repeat count (`None` = loop forever).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeats: Option<u32>,
    /// Frames in playback order.
    pub frames: Vec<CompactFrame>,
    /// Per-part overrides declared in metadata. At runtime these are merged
    /// between code-side overrides (highest priority) and the base animation
    /// (lowest priority).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub part_overrides: Vec<CompactAnimationOverride>,
    /// Per-animation ground anchor override.  When present, the runtime uses
    /// this instead of the entity-level `ground_anchor_y` while this animation
    /// is active.  Emitted by the export pipeline when the animation's lowest
    /// visible pixel differs from the entity-level default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground_anchor_y: Option<i16>,
}

/// A part-scoped animation override declared in the atlas metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactAnimationOverride {
    /// Source animation tag to pull pose data from.
    pub source_tag: String,
    /// Part selector (by tags or ids).
    pub selector: CompactPartSelector,
    /// When true, only sprite data is taken; position comes from the base.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sprite_only: bool,
}

/// Selector targeting a subset of parts by tag or id.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompactPartSelector {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub part_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub part_tags: Vec<String>,
}

/// How the entity's world position relates to the composed sprite.
///
/// - `BottomOrigin` — entity position sits at the lowest visible pixel,
///   X centred on the authored origin. Correct for ground-contact enemies.
/// - `Origin` — entity position sits at the full authored origin `(x, y)`.
///   Correct for flying enemies where the origin marks body centre.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum SpawnAnchorMode {
    /// Entity position = bottom of bounding box, X from origin.
    #[default]
    BottomOrigin,
    /// Entity position = authored origin (both X and Y).
    Origin,
}

/// Playback direction enum (replaces string).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompactDirection {
    Forward,
    Reverse,
    PingPong,
    PingPongReverse,
}

/// One composed frame.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactFrame {
    /// Duration in milliseconds.
    pub duration_ms: u16,
    /// Animation events (rare — usually empty).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<CompactEvent>,
    /// Per-part poses for this frame.
    pub poses: Vec<CompactPose>,
}

/// One animation event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactEvent {
    pub kind: AnimationEventKind,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<u8>,
    #[serde(default, skip_serializing_if = "is_zero_offset")]
    pub offset: (i8, i8),
}

/// One per-part pose in a composed frame.
///
/// # Invariants enforced at encode time
///
/// - `opacity` must be 255 — omitted from compact format entirely.
/// - `visible` must be true — omitted from compact format entirely.
///
/// If either invariant is violated the encoder fails with a diagnostic error.
/// If future authoring needs per-pose opacity or visibility, add the fields
/// back here and bump the format.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactPose {
    /// Part index into `part_names`.
    pub p: u8,
    /// Sprite index (atlas order).
    pub s: u8,
    /// Pivot-to-pivot offset `(x, y)`.
    pub o: (i8, i8),
    /// Horizontal flip.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fx: bool,
    /// Vertical flip.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fy: bool,
    /// Fragment index for split parts (omitted when 0).
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub frag: u8,
}

/// Top-level gameplay metadata.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CompactGameplay {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_health_pool: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub health_pools: Vec<CompactHealthPool>,
}

/// Shared health pool.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactHealthPool {
    pub id: String,
    pub max_health: u16,
}

// ── Helpers ────────────────────────────────────────────────────────────────

impl SpawnAnchorMode {
    /// Serde helper: returns true when the value equals the default variant.
    pub fn is_default(&self) -> bool {
        *self == Self::BottomOrigin
    }
}

fn is_zero_u8(v: &u8) -> bool {
    *v == 0
}

fn is_zero_offset(v: &(i8, i8)) -> bool {
    v.0 == 0 && v.1 == 0
}

// ── Encoder ────────────────────────────────────────────────────────────────

/// Capacity warning thresholds (80% of each limit).
const U8_WARN: usize = (u8::MAX as f64 * 0.8) as usize; // 204
const I8_POS_WARN: i32 = (i8::MAX as f64 * 0.8) as i32; // 101
const U16_WARN: u32 = (u16::MAX as f64 * 0.8) as u32; // 52428

/// Convert a full `CompositionAtlas` into the compact runtime manifest.
///
/// All numeric narrowing is bounds-checked. Dropped-field invariants
/// (`opacity == 255`, `visible == true`) are enforced with diagnostic errors.
/// Capacity warnings are logged to stderr via `eprintln!`.
pub fn encode(atlas: &CompositionAtlas) -> Result<CompactComposedAtlas> {
    let (result, warnings) = encode_with_diagnostics(atlas)?;
    for w in &warnings {
        eprintln!("[WARN] composed_ron::encode: {w}");
    }
    Ok(result)
}

/// Like [`encode`], but returns capacity warnings as a vec instead of
/// printing them. Useful for testing.
pub fn encode_with_diagnostics(
    atlas: &CompositionAtlas,
) -> Result<(CompactComposedAtlas, Vec<String>)> {
    let mut warnings: Vec<String> = Vec::new();
    // ── Validate top-level numeric bounds ───────────────────────────────
    ensure!(
        u16::try_from(atlas.canvas.w).is_ok() && u16::try_from(atlas.canvas.h).is_ok(),
        "canvas {}×{} exceeds u16 range",
        atlas.canvas.w,
        atlas.canvas.h,
    );
    ensure!(
        i16::try_from(atlas.origin.x).is_ok() && i16::try_from(atlas.origin.y).is_ok(),
        "origin ({}, {}) exceeds i16 range",
        atlas.origin.x,
        atlas.origin.y,
    );

    // ── Build part name → index lookup ─────────────────────────────────
    let mut part_names: Vec<String> = Vec::new();
    let mut part_index = std::collections::HashMap::<String, u8>::new();
    for part in &atlas.parts {
        let idx = part_names.len();
        ensure!(idx <= u8::MAX as usize, "too many parts (>255)");
        let idx = u8::try_from(idx)
            .map_err(|_| anyhow!("part index {} for '{}' exceeds u8 range", idx, part.id))?;
        part_index.insert(part.id.clone(), idx);
        part_names.push(part.id.clone());
    }
    if part_names.len() >= U8_WARN {
        warnings.push(format!(
            "part count {} is approaching u8 limit (255)",
            part_names.len(),
        ));
    }

    // ── Build sprite id → index lookup ─────────────────────────────────
    let mut sprite_index = std::collections::HashMap::<String, u8>::new();
    let mut sprite_names = Vec::with_capacity(atlas.sprites.len());
    let mut sprite_sizes = Vec::with_capacity(atlas.sprites.len());
    for (i, sprite) in atlas.sprites.iter().enumerate() {
        ensure!(i <= u8::MAX as usize, "too many sprites (>255)");
        let sprite_idx = u8::try_from(i)
            .map_err(|_| anyhow!("sprite index {} for '{}' exceeds u8 range", i, sprite.id))?;
        let sprite_width = u16::try_from(sprite.rect.w).map_err(|_| {
            anyhow!(
                "sprite '{}' width {} exceeds u16 range",
                sprite.id,
                sprite.rect.w
            )
        })?;
        let sprite_height = u16::try_from(sprite.rect.h).map_err(|_| {
            anyhow!(
                "sprite '{}' height {} exceeds u16 range",
                sprite.id,
                sprite.rect.h
            )
        })?;
        sprite_index.insert(sprite.id.clone(), sprite_idx);
        sprite_names.push(sprite.id.clone());
        sprite_sizes.push((sprite_width, sprite_height));
    }
    if sprite_names.len() >= U8_WARN {
        warnings.push(format!(
            "sprite count {} is approaching u8 limit (255)",
            sprite_names.len(),
        ));
    }

    // ── Build definition lookup for merging ─────────────────────────────
    let def_lookup: std::collections::HashMap<&str, &crate::aseprite::PartDefinition> = atlas
        .part_definitions
        .iter()
        .map(|d| (d.id.as_str(), d))
        .collect();

    // ── Build compact parts ─────────────────────────────────────────────
    let mut parts = Vec::with_capacity(atlas.parts.len());
    for inst in &atlas.parts {
        let draw_order = u8::try_from(inst.draw_order).map_err(|_| {
            anyhow!(
                "part '{}' draw_order {} exceeds u8 range",
                inst.id,
                inst.draw_order,
            )
        })?;
        if draw_order as usize >= U8_WARN {
            warnings.push(format!(
                "part '{}' draw_order {} is approaching u8 limit (255)",
                inst.id, draw_order,
            ));
        }
        let pivot_x = i16::try_from(inst.pivot.x).map_err(|_| {
            anyhow!(
                "part '{}' pivot.x {} exceeds i16 range",
                inst.id,
                inst.pivot.x,
            )
        })?;
        let pivot_y = i16::try_from(inst.pivot.y).map_err(|_| {
            anyhow!(
                "part '{}' pivot.y {} exceeds i16 range",
                inst.id,
                inst.pivot.y,
            )
        })?;

        let def = def_lookup.get(inst.definition_id.as_str());
        let def_tags = def.map_or(&[] as &[String], |d| &d.tags);
        let def_gameplay = def.map(|d| &d.gameplay);

        let mut tags: Vec<String> = def_tags.to_vec();
        for tag in &inst.tags {
            if !tags.contains(tag) {
                tags.push(tag.clone());
            }
        }

        let gameplay = merge_gameplay(def_gameplay, &inst.gameplay, &inst.id)?;

        let parent = inst
            .parent_id
            .as_ref()
            .and_then(|pid| part_index.get(pid.as_str()).copied());

        parts.push(CompactPart {
            id: *part_index.get(inst.id.as_str()).unwrap(),
            parent,
            visual: inst.source_layer.is_some() || inst.source_region.is_some(),
            draw_order,
            pivot: (pivot_x, pivot_y),
            tags,
            gameplay,
        });
    }

    // ── Build compact animations ────────────────────────────────────────
    let animations: Vec<CompactAnimation> = atlas
        .animations
        .iter()
        .map(|anim| {
            let direction = match anim.direction.as_str() {
                "forward" => CompactDirection::Forward,
                "reverse" => CompactDirection::Reverse,
                "ping_pong" => CompactDirection::PingPong,
                "ping_pong_reverse" => CompactDirection::PingPongReverse,
                other => bail!(
                    "unknown animation direction '{}' in tag '{}'",
                    other,
                    anim.tag,
                ),
            };

            let mut frames = Vec::with_capacity(anim.frames.len());
            for (frame_idx, frame) in anim.frames.iter().enumerate() {
                ensure!(
                    u16::try_from(frame.duration_ms).is_ok(),
                    "animation '{}' frame {} duration_ms {} exceeds u16 range",
                    anim.tag,
                    frame_idx,
                    frame.duration_ms,
                );

                let mut poses = Vec::with_capacity(frame.parts.len());
                for pose in &frame.parts {
                    // ── Dropped-field invariants ────────────────────────
                    ensure!(
                        pose.opacity == 255,
                        "animation '{}' frame {} part '{}': opacity {} is not 255; \
                         compact format requires fully opaque poses",
                        anim.tag,
                        frame_idx,
                        pose.part_id,
                        pose.opacity,
                    );
                    ensure!(
                        pose.visible,
                        "animation '{}' frame {} part '{}': visible is false; \
                         compact format requires all poses to be visible",
                        anim.tag,
                        frame_idx,
                        pose.part_id,
                    );

                    // ── Numeric narrowing ──────────────────────────────
                    let p = *part_index.get(pose.part_id.as_str()).ok_or_else(|| {
                        anyhow::anyhow!(
                            "pose references unknown part '{}' in animation '{}'",
                            pose.part_id,
                            anim.tag,
                        )
                    })?;
                    let s = *sprite_index.get(pose.sprite_id.as_str()).ok_or_else(|| {
                        anyhow::anyhow!(
                            "pose references unknown sprite '{}' in animation '{}'",
                            pose.sprite_id,
                            anim.tag,
                        )
                    })?;
                    let offset_x = i8::try_from(pose.local_offset.x).map_err(|_| {
                        anyhow!(
                            "animation '{}' frame {} part '{}': offset.x {} exceeds i8 range",
                            anim.tag,
                            frame_idx,
                            pose.part_id,
                            pose.local_offset.x,
                        )
                    })?;
                    let offset_y = i8::try_from(pose.local_offset.y).map_err(|_| {
                        anyhow!(
                            "animation '{}' frame {} part '{}': offset.y {} exceeds i8 range",
                            anim.tag,
                            frame_idx,
                            pose.part_id,
                            pose.local_offset.y,
                        )
                    })?;
                    if pose.local_offset.x.abs() >= I8_POS_WARN
                        || pose.local_offset.y.abs() >= I8_POS_WARN
                    {
                        warnings.push(format!(
                            "animation '{}' frame {} part '{}': offset ({}, {}) is approaching i8 limits",
                            anim.tag, frame_idx, pose.part_id, pose.local_offset.x, pose.local_offset.y,
                        ));
                    }
                    let fragment = u8::try_from(pose.fragment).map_err(|_| {
                        anyhow!(
                            "animation '{}' frame {} part '{}': fragment {} exceeds u8 range",
                            anim.tag,
                            frame_idx,
                            pose.part_id,
                            pose.fragment,
                        )
                    })?;

                    poses.push(CompactPose {
                        p,
                        s,
                        o: (offset_x, offset_y),
                        fx: pose.flip_x,
                        fy: pose.flip_y,
                        frag: fragment,
                    });
                }

                let mut events = Vec::with_capacity(frame.events.len());
                for event in &frame.events {
                    let offset_x = i8::try_from(event.local_offset.x).map_err(|_| {
                        anyhow!(
                            "animation '{}' frame {} event '{}': offset.x {} exceeds i8 range",
                            anim.tag,
                            frame_idx,
                            event.id,
                            event.local_offset.x,
                        )
                    })?;
                    let offset_y = i8::try_from(event.local_offset.y).map_err(|_| {
                        anyhow!(
                            "animation '{}' frame {} event '{}': offset.y {} exceeds i8 range",
                            anim.tag,
                            frame_idx,
                            event.id,
                            event.local_offset.y,
                        )
                    })?;
                    let part = event
                        .part_id
                        .as_ref()
                        .map(|pid| {
                            part_index.get(pid.as_str()).copied().ok_or_else(|| {
                                anyhow::anyhow!(
                                    "event references unknown part '{}' in animation '{}'",
                                    pid,
                                    anim.tag,
                                )
                            })
                        })
                        .transpose()?;
                    events.push(CompactEvent {
                        kind: event.kind,
                        id: event.id.clone(),
                        part,
                        offset: (offset_x, offset_y),
                    });
                }

                let duration_ms = u16::try_from(frame.duration_ms).map_err(|_| {
                    anyhow!(
                        "animation '{}' frame {} duration_ms {} exceeds u16 range",
                        anim.tag,
                        frame_idx,
                        frame.duration_ms,
                    )
                })?;
                if duration_ms as u32 >= U16_WARN {
                    warnings.push(format!(
                        "animation '{}' frame {} duration_ms {} is approaching u16 limit (65535)",
                        anim.tag, frame_idx, duration_ms,
                    ));
                }

                frames.push(CompactFrame {
                    duration_ms,
                    events,
                    poses,
                });
            }

            let part_overrides = anim
                .part_overrides
                .iter()
                .map(|o| CompactAnimationOverride {
                    source_tag: o.source_tag.clone(),
                    selector: CompactPartSelector {
                        part_ids: o.part_ids.clone(),
                        part_tags: o.part_tags.clone(),
                    },
                    sprite_only: o.sprite_only,
                })
                .collect();

            // Per-animation ground anchor: derive from frame data if entity
            // has an entity-level ground anchor to compare against.
            let anim_ground = if atlas.spawn_anchor == SpawnAnchorMode::Origin {
                derive_animation_ground_anchor(
                    anim,
                    &sprite_index,
                    &sprite_sizes,
                    atlas.ground_anchor_y,
                )
            } else {
                None
            };

            Ok(CompactAnimation {
                tag: anim.tag.clone(),
                direction,
                repeats: anim.repeats,
                frames,
                part_overrides,
                ground_anchor_y: anim_ground,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // ── Gameplay ────────────────────────────────────────────────────────
    let mut health_pools = Vec::with_capacity(atlas.gameplay.health_pools.len());
    for pool in &atlas.gameplay.health_pools {
        let max_health = u16::try_from(pool.max_health).map_err(|_| {
            anyhow!(
                "health pool '{}' max_health {} exceeds u16 range",
                pool.id,
                pool.max_health,
            )
        })?;
        health_pools.push(CompactHealthPool {
            id: pool.id.clone(),
            max_health,
        });
    }

    let gameplay = CompactGameplay {
        entity_health_pool: atlas.gameplay.entity_health_pool.clone(),
        health_pools,
    };

    let canvas_w = u16::try_from(atlas.canvas.w)
        .map_err(|_| anyhow!("canvas.w {} exceeds u16 range", atlas.canvas.w))?;
    let canvas_h = u16::try_from(atlas.canvas.h)
        .map_err(|_| anyhow!("canvas.h {} exceeds u16 range", atlas.canvas.h))?;
    let origin_x = i16::try_from(atlas.origin.x)
        .map_err(|_| anyhow!("origin.x {} exceeds i16 range", atlas.origin.x))?;
    let origin_y = i16::try_from(atlas.origin.y)
        .map_err(|_| anyhow!("origin.y {} exceeds i16 range", atlas.origin.y))?;

    Ok((
        CompactComposedAtlas {
            entity: atlas.entity.clone(),
            depth: atlas.depth,
            canvas: (canvas_w, canvas_h),
            origin: (origin_x, origin_y),
            spawn_anchor: atlas.spawn_anchor,
            ground_anchor_y: atlas.ground_anchor_y,
            air_anchor_y: atlas.air_anchor_y,
            part_names,
            sprite_names,
            sprite_sizes,
            parts,
            animations,
            gameplay,
        },
        warnings,
    ))
}

/// Derive per-animation ground anchor.  Returns `Some(value)` only when the
/// animation's lowest visible pixel differs from the entity-level default.
fn derive_animation_ground_anchor(
    anim: &crate::aseprite::Animation,
    sprite_index: &std::collections::HashMap<String, u8>,
    sprite_sizes: &[(u16, u16)],
    entity_ground: Option<i16>,
) -> Option<i16> {
    let entity_default = entity_ground?;
    let mut max_bottom: Option<i32> = None;
    for frame in &anim.frames {
        for pose in &frame.parts {
            if !pose.visible {
                continue;
            }
            let Some(&idx) = sprite_index.get(&pose.sprite_id) else {
                continue;
            };
            if let Some(&(_, h)) = sprite_sizes.get(idx as usize) {
                let bottom = pose.local_offset.y + i32::from(h);
                max_bottom = Some(max_bottom.map_or(bottom, |prev| prev.max(bottom)));
            }
        }
    }
    let anim_ground = i16::try_from(max_bottom?).ok()?;
    if anim_ground == entity_default {
        None // same as default — no override needed
    } else {
        Some(anim_ground)
    }
}

fn merge_gameplay(
    def: Option<&crate::aseprite::PartGameplayMetadata>,
    inst: &crate::aseprite::PartGameplayMetadata,
    part_id: &str,
) -> Result<CompactPartGameplay> {
    let def_default = crate::aseprite::PartGameplayMetadata::default();
    let def = def.unwrap_or(&def_default);

    let armour = if inst.armour > 0 {
        inst.armour
    } else {
        def.armour
    };
    let armour = u8::try_from(armour)
        .map_err(|_| anyhow!("part '{}' armour {} exceeds u8 range", part_id, armour))?;
    let durability = inst.durability.or(def.durability);
    let durability = durability
        .map(|d| {
            u8::try_from(d)
                .map_err(|_| anyhow!("part '{}' durability {} exceeds u8 range", part_id, d))
        })
        .transpose()?;

    Ok(CompactPartGameplay {
        targetable: inst.targetable.or(def.targetable).unwrap_or(false),
        health_pool: inst.health_pool.clone().or_else(|| def.health_pool.clone()),
        armour,
        durability,
        breakable: inst.breakable.or(def.breakable).unwrap_or(false),
        pool_damage_ratio: inst.pool_damage_ratio.or(def.pool_damage_ratio),
        collisions: merge_collisions(def, inst),
    })
}

fn merge_collisions(
    def: &crate::aseprite::PartGameplayMetadata,
    inst: &crate::aseprite::PartGameplayMetadata,
) -> Vec<CompactCollision> {
    // Instance collisions override definition collisions entirely when present.
    let source = if inst.collision.is_empty() {
        &def.collision
    } else {
        &inst.collision
    };
    source
        .iter()
        .map(|vol| CompactCollision {
            role: vol.role,
            shape: vol.shape.clone(),
        })
        .collect()
}

/// Serialize a compact atlas to RON string (compact, non-pretty).
pub fn to_ron(compact: &CompactComposedAtlas) -> Result<String> {
    Ok(ron::to_string(compact)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aseprite::CompositionAtlas;
    use std::{fs, path::PathBuf};

    fn load_test_atlas() -> CompositionAtlas {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/sprites/enemies/mosquiton_3/atlas.json");
        let body = fs::read_to_string(path).expect("atlas.json should exist");
        serde_json::from_str(&body).expect("atlas.json should deserialize")
    }

    #[test]
    fn encode_mosquiton_produces_valid_compact_atlas() {
        let atlas = load_test_atlas();
        let compact = encode(&atlas).unwrap();

        assert_eq!(compact.part_names.len(), atlas.parts.len());
        assert_eq!(compact.sprite_sizes.len(), atlas.sprites.len());
        assert_eq!(compact.animations.len(), atlas.animations.len());

        // Verify all animations have the right frame counts.
        for (orig, comp) in atlas.animations.iter().zip(compact.animations.iter()) {
            assert_eq!(orig.tag, comp.tag);
            assert_eq!(orig.frames.len(), comp.frames.len());
            for (orig_frame, comp_frame) in orig.frames.iter().zip(comp.frames.iter()) {
                assert_eq!(orig_frame.parts.len(), comp_frame.poses.len());
            }
        }
    }

    #[test]
    fn compact_ron_is_significantly_smaller_than_json() {
        let atlas = load_test_atlas();
        let compact = encode(&atlas).unwrap();
        let ron_str = to_ron(&compact).unwrap();
        let json_str = serde_json::to_string_pretty(&atlas).unwrap();

        let reduction = 100.0 - (ron_str.len() as f64 / json_str.len() as f64 * 100.0);
        println!(
            "JSON: {} bytes, RON: {} bytes, reduction: {:.1}%",
            json_str.len(),
            ron_str.len(),
            reduction
        );
        // Expect at least 60% reduction.
        assert!(
            reduction > 60.0,
            "expected >60% reduction, got {reduction:.1}%"
        );
    }

    #[test]
    fn compact_ron_roundtrips() {
        let atlas = load_test_atlas();
        let compact = encode(&atlas).unwrap();
        let ron_str = to_ron(&compact).unwrap();
        let roundtripped: CompactComposedAtlas = ron::from_str(&ron_str).unwrap();

        assert_eq!(roundtripped.part_names, compact.part_names);
        assert_eq!(roundtripped.sprite_sizes, compact.sprite_sizes);
        assert_eq!(roundtripped.animations.len(), compact.animations.len());
    }

    #[test]
    #[ignore = "generates atlas.composed.ron for existing assets"]
    fn generate_mosquiton_composed_ron() {
        let atlas = load_test_atlas();
        let compact = encode(&atlas).unwrap();
        let ron_str = to_ron(&compact).unwrap();
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/sprites/enemies/mosquiton_3/atlas.composed.ron");
        fs::write(&out, &ron_str).unwrap();
        println!("Wrote {} bytes to {}", ron_str.len(), out.display());
    }

    #[test]
    fn encode_is_deterministic() {
        let atlas = load_test_atlas();
        let a = to_ron(&encode(&atlas).unwrap()).unwrap();
        let b = to_ron(&encode(&atlas).unwrap()).unwrap();
        assert_eq!(a, b);
    }

    // ── Hardening tests ────────────────────────────────────────────────

    use crate::aseprite::{
        Animation, AnimationFrame, AtlasSprite, CompositionGameplay, PartDefinition, PartInstance,
        PartPose, Point, Rect, Size,
    };

    /// Minimal valid atlas for invariant testing.
    fn tiny_atlas() -> CompositionAtlas {
        CompositionAtlas {
            schema_version: 3,
            entity: "test".into(),
            depth: 0,
            source: "test.aseprite".into(),
            canvas: Size { w: 8, h: 8 },
            origin: Point { x: 4, y: 4 },
            spawn_anchor: Default::default(),
            ground_anchor_y: None,
            air_anchor_y: None,
            atlas_image: "source.png".into(),
            part_definitions: vec![PartDefinition {
                id: "body".into(),
                tags: vec![],
                gameplay: Default::default(),
            }],
            parts: vec![PartInstance {
                id: "body".into(),
                definition_id: "body".into(),
                name: "Body".into(),
                parent_id: None,
                source_layer: Some("body".into()),
                source_region: None,
                split: None,
                draw_order: 0,
                pivot: Point::default(),
                tags: vec![],
                visible_by_default: true,
                gameplay: Default::default(),
            }],
            sprites: vec![AtlasSprite {
                id: "sprite_0000".into(),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 4,
                    h: 4,
                },
            }],
            animations: vec![Animation {
                tag: "idle".into(),
                direction: "forward".into(),
                repeats: None,
                frames: vec![AnimationFrame {
                    source_frame: 0,
                    duration_ms: 100,
                    events: vec![],
                    parts: vec![PartPose {
                        part_id: "body".into(),
                        sprite_id: "sprite_0000".into(),
                        local_offset: Point { x: 0, y: 0 },

                        flip_x: false,
                        flip_y: false,
                        visible: true,
                        opacity: 255,
                        fragment: 0,
                    }],
                }],
                part_overrides: vec![],
            }],
            gameplay: CompositionGameplay::default(),
        }
    }

    // -- Dropped-field invariants --

    #[test]
    fn rejects_non_opaque_pose() {
        let mut atlas = tiny_atlas();
        atlas.animations[0].frames[0].parts[0].opacity = 200;

        let err = encode(&atlas).unwrap_err();
        assert!(
            err.to_string().contains("opacity 200 is not 255"),
            "got: {err}",
        );
    }

    #[test]
    fn rejects_invisible_pose() {
        let mut atlas = tiny_atlas();
        atlas.animations[0].frames[0].parts[0].visible = false;

        let err = encode(&atlas).unwrap_err();
        assert!(err.to_string().contains("visible is false"), "got: {err}",);
    }

    // -- Numeric narrowing --

    #[test]
    fn rejects_offset_exceeding_i8() {
        let mut atlas = tiny_atlas();
        atlas.animations[0].frames[0].parts[0].local_offset = Point { x: 200, y: 0 };

        let err = encode(&atlas).unwrap_err();
        assert!(err.to_string().contains("exceeds i8 range"), "got: {err}",);
    }

    #[test]
    fn rejects_negative_offset_exceeding_i8() {
        let mut atlas = tiny_atlas();
        atlas.animations[0].frames[0].parts[0].local_offset = Point { x: 0, y: -200 };

        let err = encode(&atlas).unwrap_err();
        assert!(err.to_string().contains("exceeds i8 range"), "got: {err}",);
    }

    #[test]
    fn accepts_max_valid_i8_offset() {
        let mut atlas = tiny_atlas();
        atlas.animations[0].frames[0].parts[0].local_offset = Point { x: 127, y: -128 };

        encode(&atlas).expect("max i8 values should be accepted");
    }

    #[test]
    fn rejects_duration_exceeding_u16() {
        let mut atlas = tiny_atlas();
        atlas.animations[0].frames[0].duration_ms = 70_000;

        let err = encode(&atlas).unwrap_err();
        assert!(err.to_string().contains("exceeds u16 range"), "got: {err}",);
    }

    #[test]
    fn rejects_fragment_exceeding_u8() {
        let mut atlas = tiny_atlas();
        atlas.animations[0].frames[0].parts[0].fragment = 300;

        let err = encode(&atlas).unwrap_err();
        assert!(err.to_string().contains("exceeds u8 range"), "got: {err}",);
    }

    #[test]
    fn rejects_draw_order_exceeding_u8() {
        let mut atlas = tiny_atlas();
        atlas.parts[0].draw_order = 300;

        let err = encode(&atlas).unwrap_err();
        assert!(err.to_string().contains("exceeds u8 range"), "got: {err}",);
    }

    #[test]
    fn rejects_health_exceeding_u16() {
        let mut atlas = tiny_atlas();
        atlas
            .gameplay
            .health_pools
            .push(crate::aseprite::HealthPool {
                id: "core".into(),
                max_health: 70_000,
            });

        let err = encode(&atlas).unwrap_err();
        assert!(err.to_string().contains("exceeds u16 range"), "got: {err}",);
    }

    #[test]
    fn rejects_sprite_dimensions_exceeding_u16() {
        let mut atlas = tiny_atlas();
        atlas.sprites[0].rect.w = u16::MAX as u32 + 1;

        let err = encode(&atlas).unwrap_err();
        assert!(
            err.to_string().contains("sprite 'sprite_0000' width"),
            "got: {err}"
        );
    }

    #[test]
    fn warns_at_high_part_count() {
        let mut atlas = tiny_atlas();
        // Add parts until we reach the 80% threshold (204).
        for i in 1..=205 {
            let id = format!("part_{i}");
            atlas.part_definitions.push(PartDefinition {
                id: id.clone(),
                tags: vec![],
                gameplay: Default::default(),
            });
            atlas.parts.push(PartInstance {
                id: id.clone(),
                definition_id: id,
                name: format!("Part {i}"),
                parent_id: None,
                source_layer: None,
                source_region: None,
                split: None,
                draw_order: 0,
                pivot: Point::default(),
                tags: vec![],
                visible_by_default: true,
                gameplay: Default::default(),
            });
        }

        let (_compact, warnings) =
            encode_with_diagnostics(&atlas).expect("should encode with warnings");
        assert!(
            warnings.iter().any(|w| w.contains("part count")),
            "expected part count warning, got: {warnings:?}"
        );
    }

    #[test]
    fn warns_at_high_offset() {
        let mut atlas = tiny_atlas();
        atlas.animations[0].frames[0].parts[0].local_offset = Point { x: 102, y: 0 };

        let (_compact, warnings) =
            encode_with_diagnostics(&atlas).expect("should encode with warnings");
        assert!(
            warnings.iter().any(|w| w.contains("offset")),
            "expected offset warning, got: {warnings:?}"
        );
    }

    #[test]
    fn no_warning_below_threshold() {
        let atlas = tiny_atlas();
        let (_compact, warnings) = encode_with_diagnostics(&atlas).expect("should encode cleanly");
        assert!(
            warnings.is_empty(),
            "expected no warnings, got: {warnings:?}"
        );
    }
}
