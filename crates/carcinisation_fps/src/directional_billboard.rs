//! Directional billboard system for 8-way sprite selection.
//!
//! Resolves the correct sprite frame and horizontal flip for a billboarded
//! entity based on the relative angle between an observer and the entity's
//! facing direction.
//!
//! Supports N physical directions (atlas-backed) mapped to 8 virtual
//! directions via mirroring, keeping the atlas compact while the renderer
//! stays simple.

use std::collections::HashMap;
use std::f32::consts::TAU;
use std::sync::Arc;

use carapace::image::CxImage;
use carcinisation_base::direction::{
    self, NUM_PHYSICAL_DIRECTIONS, ParsedDirectionalTag, SpriteDirection,
};

/// Playback mode for an animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Playback {
    Forward,
    Reverse,
    PingPong,
}

/// Frame ordering within an animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameOrder {
    Normal,
    Reversed,
}

/// One of the 8 virtual directions.
#[derive(Clone, Debug)]
pub struct BillboardDirection {
    /// The semantic direction.
    pub direction: SpriteDirection,
    /// Index into the physical direction's sprite data.
    /// For mirrored directions, this points to the source direction
    /// (or a virtual slot when custom layer ordering requires recomposition).
    pub source_direction_idx: usize,
    /// Whether to horizontally flip the sprite at render time.
    pub flip_x: bool,
}

/// Animation definition for one action (e.g. `idle_stand`, `walk_forward`).
#[derive(Clone, Debug)]
pub struct BillboardAnimation {
    pub playback: Playback,
    pub frame_order: FrameOrder,
    pub frame_count: usize,
    pub duration_secs: f32,
}

/// Per-direction sprite frames, extracted from the atlas.
#[derive(Clone, Debug)]
pub struct DirectionFrames {
    /// One `CxImage` per animation frame for this direction+action combo.
    pub frames: Vec<Arc<CxImage>>,
}

/// Complete directional billboard atlas — loaded once, shared across entities.
pub struct DirectionalBillboardAtlas {
    /// 8 virtual directions sorted by angle for quantization.
    pub directions: Vec<BillboardDirection>,
    /// Animation definitions keyed by action name.
    pub animations: HashMap<String, BillboardAnimation>,
    /// Sprite data: `sprites[action_name][physical_direction_idx][frame_idx]`.
    /// Physical directions are the 5 atlas-backed directions (front, frontleft,
    /// left, backleft, back).
    pub sprites: HashMap<String, Vec<DirectionFrames>>,
}

/// Actions that must exist in the embedded player billboard atlas.
const REQUIRED_PLAYER_ACTIONS: &[&str] = &["idle_stand", "walk_forward", "death"];

/// Per-entity animation state.
#[derive(Clone, Debug)]
pub struct BillboardAnimationState {
    /// Current action name (e.g. `idle_stand`, `walk_forward`, death).
    pub action: String,
    /// Elapsed time in seconds since this action started.
    pub elapsed: f32,
}

impl BillboardAnimationState {
    #[must_use]
    pub fn new(action: &str) -> Self {
        Self {
            action: action.to_string(),
            elapsed: 0.0,
        }
    }

    /// Advance elapsed time by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        self.elapsed += dt;
    }

    /// Switch to a new action, resetting elapsed time.
    pub fn set_action(&mut self, action: &str) {
        if self.action != action {
            self.action = action.to_string();
            self.elapsed = 0.0;
        }
    }
}

/// Result of resolving a billboard's facing: which sprite to render and how.
#[derive(Clone, Debug)]
pub struct ResolvedBillboard {
    /// The sprite image for this frame.
    pub sprite: Arc<CxImage>,
    /// Whether to horizontally flip the sprite.
    pub flip_x: bool,
}

/// Normalize an angle to [0, TAU).
fn normalize_angle(angle: f32) -> f32 {
    let a = angle % TAU;
    if a < 0.0 { a + TAU } else { a }
}

/// Compute the relative viewing angle for billboard direction selection.
///
/// Returns an angle in [0, TAU) where:
/// - 0 = observer sees the target's front
/// - PI = observer sees the target's back
///
/// # Arguments
/// - `observer_pos`: position of the viewer (camera)
/// - `target_pos`: position of the billboarded entity
/// - `target_angle`: the direction the target entity is facing (radians, 0 = east)
#[must_use]
pub fn relative_viewing_angle(
    observer_pos: bevy_math::Vec2,
    target_pos: bevy_math::Vec2,
    target_angle: f32,
) -> f32 {
    let delta = observer_pos - target_pos;
    let angle_to_observer = delta.y.atan2(delta.x);
    // Relative angle: how far the observer is from the target's forward direction.
    // 0 means the observer is directly in front of the target.
    normalize_angle(angle_to_observer - target_angle)
}

/// Quantize a relative viewing angle to the nearest direction index.
///
/// Divides the circle into `n` equal sectors centered on each direction's angle.
/// Returns the index of the closest direction.
#[must_use]
pub fn quantize_direction(relative_angle: f32, num_directions: usize) -> usize {
    let sector = TAU / num_directions as f32;
    let shifted = normalize_angle(relative_angle + sector / 2.0);
    let idx = (shifted / sector) as usize;
    idx.min(num_directions - 1)
}

/// Resolve the billboard sprite for a given observer-target relationship.
///
/// Returns `None` if the action or direction data is missing.
#[must_use]
pub fn resolve_billboard(
    atlas: &DirectionalBillboardAtlas,
    observer_pos: bevy_math::Vec2,
    target_pos: bevy_math::Vec2,
    target_angle: f32,
    state: &BillboardAnimationState,
) -> Option<ResolvedBillboard> {
    let anim = atlas.animations.get(&state.action)?;
    let action_sprites = atlas.sprites.get(&state.action)?;

    let rel_angle = relative_viewing_angle(observer_pos, target_pos, target_angle);
    let dir_idx = quantize_direction(rel_angle, atlas.directions.len());
    let dir = &atlas.directions[dir_idx];

    let frames = action_sprites
        .get(dir.source_direction_idx)
        .filter(|f| !f.frames.is_empty())
        // Fallback: if virtual slot is empty, use the physical source.
        .or_else(|| {
            action_sprites
                .get(dir.direction.physical_index())
                .filter(|f| !f.frames.is_empty())
        })?;

    let frame_idx = compute_frame_index(state.elapsed, anim);
    let sprite = Arc::clone(frames.frames.get(frame_idx)?);

    Some(ResolvedBillboard {
        sprite,
        flip_x: dir.flip_x,
    })
}

/// Compute the animation frame index from elapsed time and animation config.
///
/// All playback modes loop indefinitely. One-shot (play-once-and-hold) is not
/// supported — callers must stop ticking `elapsed` externally if needed.
///
/// Note: `PingPong` with low frame counts (e.g. 3) causes the middle frame to
/// display twice as long as the endpoints, since the bounce point hits it on
/// both the forward and reverse passes. This is a standard `PingPong` quirk.
fn compute_frame_index(elapsed: f32, anim: &BillboardAnimation) -> usize {
    if anim.frame_count <= 1 {
        return 0;
    }

    let progress = if anim.duration_secs > 0.0 {
        elapsed / anim.duration_secs
    } else {
        0.0
    };

    match anim.playback {
        Playback::Forward => {
            let looped = progress % 1.0;
            let raw = (looped * anim.frame_count as f32) as usize;
            let idx = raw.min(anim.frame_count - 1);
            apply_frame_order(idx, anim.frame_count, anim.frame_order)
        }
        Playback::Reverse => {
            let looped = progress % 1.0;
            let forward = (looped * anim.frame_count as f32) as usize;
            let raw = anim.frame_count - 1 - forward.min(anim.frame_count - 1);
            let idx = raw.min(anim.frame_count - 1);
            apply_frame_order(idx, anim.frame_count, anim.frame_order)
        }
        Playback::PingPong => {
            let cycle = progress % 2.0;
            let t = if cycle < 1.0 { cycle } else { 2.0 - cycle };
            let raw = (t * anim.frame_count as f32) as usize;
            let idx = raw.min(anim.frame_count - 1);
            apply_frame_order(idx, anim.frame_count, anim.frame_order)
        }
    }
}

const fn apply_frame_order(idx: usize, frame_count: usize, order: FrameOrder) -> usize {
    match order {
        FrameOrder::Normal => idx,
        FrameOrder::Reversed => frame_count - 1 - idx,
    }
}

/// Sprite-asset key per quantization bucket (bucket index = relative viewing
/// angle quantized into 45° CCW sectors from front).
///
/// These are **asset keys**, not geometric octants: several buckets display a
/// horizontally mirrored (`flip_x`) asset whose name is the opposite side.
/// The collision system's `BillboardFacing8` names the *geometric* octant for
/// the same bucket index — convert between the two with
/// [`sprite_direction_for_facing`], never by matching variant names.
const QUANTIZED_SPRITE_DIRECTIONS: [SpriteDirection; 8] = [
    SpriteDirection::Front,
    SpriteDirection::FrontRight,
    SpriteDirection::Right,
    SpriteDirection::BackLeft,
    SpriteDirection::Back,
    SpriteDirection::BackRight,
    SpriteDirection::Left,
    SpriteDirection::FrontLeft,
];

/// Standard 8-direction layout in angle order (0° front → 315° frontleft).
///
/// 5 physical directions backed by atlas sprites, 3 virtual directions
/// derived via horizontal mirroring. Quantized uniformly by
/// [`quantize_direction`] into 45° sectors.
#[must_use]
pub fn standard_8_directions() -> Vec<BillboardDirection> {
    QUANTIZED_SPRITE_DIRECTIONS
        .into_iter()
        .map(|dir| BillboardDirection {
            direction: dir,
            source_direction_idx: dir.physical_index(),
            flip_x: dir.requires_flip(),
        })
        .collect()
}

/// The renderer sprite-asset key displayed for a (geometric) collision facing.
///
/// [`BillboardFacing8`] and the renderer quantize the same relative viewing
/// angle with identical sector boundaries, so the conversion is by bucket
/// index. The **names do not line up** for mirrored buckets: e.g. geometric
/// [`BillboardFacing8::FrontLeft`] (+45°, attacker on the target's left)
/// displays the asset key [`SpriteDirection::FrontRight`]. This function is
/// the single sanctioned bridge between the two namespaces — never match the
/// enums by variant name.
///
/// [`BillboardFacing8`]: carcinisation_fps_core::collision::BillboardFacing8
/// [`BillboardFacing8::FrontLeft`]: carcinisation_fps_core::collision::BillboardFacing8::FrontLeft
#[must_use]
pub fn sprite_direction_for_facing(
    facing: carcinisation_fps_core::collision::BillboardFacing8,
) -> SpriteDirection {
    QUANTIZED_SPRITE_DIRECTIONS[facing.index()]
}

type ActionDirectionFrames = HashMap<String, Vec<(usize, Vec<Arc<CxImage>>)>>;

fn validate_directional_action_coverage(
    animations: &HashMap<String, BillboardAnimation>,
    sprites: &HashMap<String, Vec<DirectionFrames>>,
    required_actions: &[&str],
) -> Result<(), String> {
    for required in required_actions {
        if !animations.contains_key(*required) {
            return Err(format!("missing required billboard action '{required}'"));
        }
    }

    for action in animations.keys() {
        let dir_frames = sprites
            .get(action)
            .ok_or_else(|| format!("action '{action}' has animation metadata but no sprites"))?;
        if dir_frames.len() < NUM_PHYSICAL_DIRECTIONS {
            return Err(format!(
                "action '{}' has {} directions, expected at least {}",
                action,
                dir_frames.len(),
                NUM_PHYSICAL_DIRECTIONS
            ));
        }
    }

    for required in required_actions {
        let dir_frames = sprites
            .get(*required)
            .ok_or_else(|| format!("missing sprites for required billboard action '{required}'"))?;
        // Check at least the 5 physical directions have frames.
        for (i, df) in dir_frames.iter().enumerate().take(NUM_PHYSICAL_DIRECTIONS) {
            if df.frames.is_empty() {
                return Err(format!("action '{required}' direction {i} has no frames"));
            }
        }
    }

    for (action, dir_frames) in sprites {
        let anim = animations
            .get(action)
            .ok_or_else(|| format!("action '{action}' has sprites but no animation metadata"))?;
        for (i, df) in dir_frames.iter().enumerate() {
            if !df.frames.is_empty() && df.frames.len() != anim.frame_count {
                return Err(format!(
                    "action '{}' direction {} has {} frames, expected {}",
                    action,
                    i,
                    df.frames.len(),
                    anim.frame_count
                ));
            }
        }
    }

    Ok(())
}

/// Build a `DirectionalBillboardAtlas` from composed atlas data.
///
/// Composites per-part sprites into full-frame `CxImage`s at load time,
/// using the same pipeline as Mosquiton billboard sprites. Tag names are
/// expected to follow `{direction}_{action}` convention.
/// Tags starting with `_` are skipped.
///
/// # Errors
///
/// Returns an error if the composed data is malformed or required tags are
/// missing.
pub fn load_directional_billboard_atlas(
    composed_ron: &str,
    px_atlas_ron: &str,
    pxi_bytes: &[u8],
) -> Result<DirectionalBillboardAtlas, String> {
    load_directional_billboard_atlas_with_config(composed_ron, px_atlas_ron, pxi_bytes, &[], None)
}

fn load_directional_billboard_atlas_with_config(
    composed_ron: &str,
    px_atlas_ron: &str,
    pxi_bytes: &[u8],
    required_actions: &[&str],
    layer_order_override: Option<&carcinisation_base::layer_order::LayerOrderConfig>,
) -> Result<DirectionalBillboardAtlas, String> {
    use crate::mosquiton::{
        PxAtlasDescriptor, compose_tag_frames, compose_tag_frames_reversed,
        compose_tag_frames_with_layer_order, decode_pxi,
    };
    use asset_pipeline::composed_ron::{CompactComposedAtlas, CompactDirection};

    let composed: CompactComposedAtlas = ron::from_str(composed_ron)
        .map_err(|err| format!("Failed to parse composed.ron: {err}"))?;
    let atlas: PxAtlasDescriptor = ron::from_str(px_atlas_ron)
        .map_err(|err| format!("Failed to parse px_atlas.ron: {err}"))?;
    let (atlas_width, _atlas_height, atlas_pixels) = decode_pxi(pxi_bytes)?;

    // Layer order: prefer explicit override, then composed RON embedded config,
    // then fall back to hardcoded Back-only reversal for backward compatibility.
    let embedded_layer_order = composed.directional_layer_order.as_ref();
    let layer_order = layer_order_override.or(embedded_layer_order);

    // Parse composed animation tags into (direction, action) pairs.
    let mut action_direction_frames: ActionDirectionFrames = HashMap::new();
    let mut action_animations: HashMap<String, BillboardAnimation> = HashMap::new();

    for anim in &composed.animations {
        let tag_name = &anim.tag;

        // Skip dev/internal tags.
        if tag_name.starts_with('_') {
            continue;
        }

        let Some(ParsedDirectionalTag { direction, action }) =
            direction::parse_directional_tag(tag_name)
        else {
            continue; // Unrecognized tag format, skip.
        };
        let dir_idx = direction.physical_index();

        // Compose per-part sprites into full-frame CxImages with direction-
        // aware draw order from LayerOrderConfig.
        let frames = if let Some(lo) = layer_order {
            compose_tag_frames_with_layer_order(
                &composed,
                &atlas,
                &atlas_pixels,
                atlas_width,
                tag_name,
                direction,
                lo,
            )?
        } else if direction == SpriteDirection::Back {
            // Backward compat: no layer order config → hardcoded Back reversal.
            compose_tag_frames_reversed(&composed, &atlas, &atlas_pixels, atlas_width, tag_name)?
        } else {
            compose_tag_frames(&composed, &atlas, &atlas_pixels, atlas_width, tag_name)?
        };
        let frame_count = frames.len();

        // Record animation metadata.
        if !action_animations.contains_key(&action) {
            let total_ms: u32 = anim.frames.iter().map(|f| u32::from(f.duration_ms)).sum();
            let mut duration_secs = total_ms as f32 / 1000.0;

            let (playback, frame_order) = match anim.direction {
                CompactDirection::Forward => (Playback::Forward, FrameOrder::Normal),
                CompactDirection::Reverse => (Playback::Reverse, FrameOrder::Normal),
                CompactDirection::PingPong => (Playback::PingPong, FrameOrder::Normal),
                CompactDirection::PingPongReverse => (Playback::PingPong, FrameOrder::Reversed),
            };

            // Idle animations play at half speed for a more relaxed breathing feel.
            if action.starts_with("idle") {
                duration_secs *= 2.0;
            }

            action_animations.insert(
                action.clone(),
                BillboardAnimation {
                    playback,
                    frame_order,
                    frame_count,
                    duration_secs,
                },
            );
        }

        action_direction_frames
            .entry(action)
            .or_default()
            .push((dir_idx, frames));
    }

    // Build the sprites map: for each action, create a Vec of DirectionFrames
    // indexed by physical direction (0..5).
    let mut sprites: HashMap<String, Vec<DirectionFrames>> = HashMap::new();

    for (action, dir_entries) in &action_direction_frames {
        let mut direction_frames =
            vec![DirectionFrames { frames: Vec::new() }; NUM_PHYSICAL_DIRECTIONS];

        for (dir_idx, frames) in dir_entries {
            direction_frames[*dir_idx] = DirectionFrames {
                frames: frames.clone(),
            };
        }

        sprites.insert(action.clone(), direction_frames);
    }

    // Compose separate frames for virtual directions that have EXPLICIT layer
    // ordering declared in config. Virtual directions without their own config
    // simply mirror the physical source's flattened image (whole-composite
    // mirroring). Only re-compose when the virtual direction has its own
    // declared entry in the layer_order config.
    let directions = standard_8_directions();
    if let Some(lo) = layer_order {
        for dir_info in &directions {
            if !dir_info.flip_x {
                continue; // Physical direction — already composed.
            }
            let virtual_dir = dir_info.direction;

            // Only re-compose if the virtual direction has an EXPLICIT config
            // entry. No entry = whole-composite mirroring from physical source.
            if !lo.direction.contains_key(&virtual_dir) {
                continue;
            }

            let physical_source = SpriteDirection::PHYSICAL[dir_info.source_direction_idx];

            for anim in &composed.animations {
                let tag_name = &anim.tag;
                if tag_name.starts_with('_') {
                    continue;
                }
                let Some(ParsedDirectionalTag {
                    direction: tag_dir,
                    action,
                }) = direction::parse_directional_tag(tag_name)
                else {
                    continue;
                };
                if tag_dir != physical_source {
                    continue;
                }

                let frames = compose_tag_frames_with_layer_order(
                    &composed,
                    &atlas,
                    &atlas_pixels,
                    atlas_width,
                    tag_name,
                    virtual_dir,
                    lo,
                )?;

                if let Some(action_sprites) = sprites.get_mut(&action) {
                    let virtual_slot = NUM_PHYSICAL_DIRECTIONS + dir_info.source_direction_idx;
                    while action_sprites.len() <= virtual_slot {
                        action_sprites.push(DirectionFrames { frames: Vec::new() });
                    }
                    action_sprites[virtual_slot] = DirectionFrames { frames };
                }
            }
        }
    }

    // Update direction source indices for virtual directions that got their
    // own composed frames.
    let mut directions = directions;
    if let Some(lo) = layer_order {
        for dir_info in &mut directions {
            if !dir_info.flip_x {
                continue;
            }
            if !lo.direction.contains_key(&dir_info.direction) {
                continue;
            }
            dir_info.source_direction_idx += NUM_PHYSICAL_DIRECTIONS;
        }
    }

    validate_directional_action_coverage(&action_animations, &sprites, required_actions)?;

    Ok(DirectionalBillboardAtlas {
        directions,
        animations: action_animations,
        sprites,
    })
}

/// Embedded player composed billboard atlas data.
const PLAYER_COMPOSED_RON: &str =
    include_str!("../../../assets/sprites/player/player_3/atlas.composed.ron");
const PLAYER_PX_ATLAS_RON: &str =
    include_str!("../../../assets/sprites/player/player_3/atlas.px_atlas.ron");
const PLAYER_PXI: &[u8] = include_bytes!("../../../assets/sprites/player/player_3/atlas.pxi");

/// Build the player directional billboard atlas from embedded composed asset data.
///
/// Composites per-part sprites (body, head, legs, arms, weapon) into
/// full-frame billboard sprites at load time. Layer ordering respects the
/// `directional.layer_order` config embedded in the composed RON manifest.
///
/// # Errors
///
/// Returns an error if the embedded assets are malformed.
pub fn make_player_billboard_atlas() -> Result<DirectionalBillboardAtlas, String> {
    load_directional_billboard_atlas_with_config(
        PLAYER_COMPOSED_RON,
        PLAYER_PX_ATLAS_RON,
        PLAYER_PXI,
        REQUIRED_PLAYER_ACTIONS,
        None, // uses directional_layer_order embedded in composed RON
    )
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use carcinisation_fps_core::collision::BillboardFacing8;

    use super::*;

    #[test]
    fn normalize_angle_wraps_negative() {
        let a = normalize_angle(-PI / 2.0);
        assert!((a - 3.0 * PI / 2.0).abs() < 1e-5);
    }

    #[test]
    fn normalize_angle_wraps_over_tau() {
        let a = normalize_angle(TAU + 1.0);
        assert!((a - 1.0).abs() < 1e-5);
    }

    #[test]
    fn quantize_8_directions_front() {
        // Angle 0 (front) should map to direction 0.
        assert_eq!(quantize_direction(0.0, 8), 0);
    }

    #[test]
    fn quantize_8_directions_back() {
        // Angle PI (back) should map to direction 4.
        assert_eq!(quantize_direction(PI, 8), 4);
    }

    #[test]
    fn quantize_8_directions_slight_right() {
        // Angle just past 22.5° maps to frontright (direction 1).
        let angle = PI / 4.0 + 0.01;
        assert_eq!(quantize_direction(angle, 8), 1);
    }

    #[test]
    fn quantize_8_directions_boundary() {
        // Angle exactly at boundary between front and frontright (22.5°) rounds to frontright.
        let boundary = PI / 8.0;
        assert_eq!(quantize_direction(boundary, 8), 1);
    }

    #[test]
    fn quantize_8_directions_just_before_boundary() {
        // Just before 22.5° stays front.
        let just_before = PI / 8.0 - 0.01;
        assert_eq!(quantize_direction(just_before, 8), 0);
    }

    #[test]
    fn quantize_wraps_near_360() {
        // Angle just before 360° should wrap to front (direction 0).
        let angle = TAU - 0.01;
        assert_eq!(quantize_direction(angle, 8), 0);
    }

    #[test]
    fn relative_viewing_angle_front() {
        // Observer directly in front of target (target faces east, observer is east).
        let observer = bevy_math::Vec2::new(5.0, 0.0);
        let target = bevy_math::Vec2::new(0.0, 0.0);
        let target_angle = 0.0; // facing east
        let angle = relative_viewing_angle(observer, target, target_angle);
        assert!(
            !(0.01..=TAU - 0.01).contains(&angle),
            "expected ~0, got {angle}"
        );
    }

    #[test]
    fn relative_viewing_angle_back() {
        // Observer behind target (target faces east, observer is west).
        let observer = bevy_math::Vec2::new(-5.0, 0.0);
        let target = bevy_math::Vec2::new(0.0, 0.0);
        let target_angle = 0.0; // facing east
        let angle = relative_viewing_angle(observer, target, target_angle);
        assert!((angle - PI).abs() < 0.01, "expected ~PI, got {angle}");
    }

    #[test]
    fn relative_viewing_angle_left() {
        // Observer north of east-facing target → angle PI/2.
        let observer = bevy_math::Vec2::new(0.0, 5.0);
        let target = bevy_math::Vec2::new(0.0, 0.0);
        let target_angle = 0.0;
        let angle = relative_viewing_angle(observer, target, target_angle);
        assert!(
            (angle - PI / 2.0).abs() < 0.01,
            "expected ~PI/2, got {angle}"
        );
    }

    #[test]
    fn relative_viewing_angle_right() {
        // Observer south of east-facing target → angle 3PI/2.
        let observer = bevy_math::Vec2::new(0.0, -5.0);
        let target = bevy_math::Vec2::new(0.0, 0.0);
        let target_angle = 0.0;
        let angle = relative_viewing_angle(observer, target, target_angle);
        assert!(
            (angle - 3.0 * PI / 2.0).abs() < 0.01,
            "expected ~3PI/2, got {angle}"
        );
    }

    #[test]
    fn relative_viewing_angle_rotated_target() {
        // Target faces north (PI/2). Observer is north of target = directly in front.
        let observer = bevy_math::Vec2::new(0.0, 5.0);
        let target = bevy_math::Vec2::new(0.0, 0.0);
        let target_angle = PI / 2.0; // facing north
        let angle = relative_viewing_angle(observer, target, target_angle);
        assert!(
            !(0.01..=TAU - 0.01).contains(&angle),
            "expected ~0, got {angle}"
        );
    }

    #[test]
    fn collision_facing_matches_renderer_direction_indices() {
        // Per-angle parity: collision facings are GEOMETRIC octants, the
        // renderer's directions are sprite-ASSET keys. They share bucket
        // boundaries/indices, but the names disagree on mirrored buckets —
        // `sprite_direction_for_facing` is the explicit bridge. The +135°
        // and +225° cases are the easiest to silently mirror: both
        // namespaces use the names BackLeft/BackRight there, and they DO
        // coincide (the back diagonals are not name-mirrored).
        let dirs = standard_8_directions();
        let cases = [
            (0.0, SpriteDirection::Front, BillboardFacing8::Front),
            (
                PI / 4.0,
                // Geometric front-LEFT octant displays the mirrored
                // front-RIGHT asset key.
                SpriteDirection::FrontRight,
                BillboardFacing8::FrontLeft,
            ),
            (PI / 2.0, SpriteDirection::Right, BillboardFacing8::Left),
            (
                3.0 * PI / 4.0,
                SpriteDirection::BackLeft,
                BillboardFacing8::BackLeft,
            ),
            (PI, SpriteDirection::Back, BillboardFacing8::Back),
            (
                5.0 * PI / 4.0,
                SpriteDirection::BackRight,
                BillboardFacing8::BackRight,
            ),
            (
                3.0 * PI / 2.0,
                SpriteDirection::Left,
                BillboardFacing8::Right,
            ),
            (
                7.0 * PI / 4.0,
                SpriteDirection::FrontLeft,
                BillboardFacing8::FrontRight,
            ),
        ];

        for (angle, sprite_direction, collision_facing) in cases {
            let renderer_index = quantize_direction(angle, dirs.len());
            assert_eq!(dirs[renderer_index].direction, sprite_direction);

            let facing = BillboardFacing8::from_relative_angle(angle);
            assert_eq!(facing, collision_facing);
            assert_eq!(facing.index(), renderer_index);
            assert_eq!(
                sprite_direction_for_facing(facing),
                sprite_direction,
                "conversion helper must agree with the quantized asset list"
            );
        }
    }

    #[test]
    fn sprite_direction_for_facing_covers_all_buckets() {
        let dirs = standard_8_directions();
        for facing in BillboardFacing8::ALL {
            assert_eq!(
                sprite_direction_for_facing(facing),
                dirs[facing.index()].direction,
                "{facing:?} must map to its bucket's asset key"
            );
        }
    }

    #[test]
    fn collision_from_positions_matches_renderer_relative_angle() {
        let dirs = standard_8_directions();
        let target = bevy_math::Vec2::ZERO;
        let cases = [
            (0.0, bevy_math::Vec2::new(5.0, 0.0)),
            (0.0, bevy_math::Vec2::new(0.0, 5.0)),
            (0.0, bevy_math::Vec2::new(0.0, -5.0)),
            (PI / 2.0, bevy_math::Vec2::new(0.0, 5.0)),
            (PI / 2.0, bevy_math::Vec2::new(-5.0, 0.0)),
            (PI / 2.0, bevy_math::Vec2::new(5.0, 0.0)),
        ];

        for (target_angle, observer) in cases {
            let renderer_angle = relative_viewing_angle(observer, target, target_angle);
            let renderer_index = quantize_direction(renderer_angle, dirs.len());
            let collision_facing = BillboardFacing8::from_positions(target, target_angle, observer);

            assert_eq!(collision_facing.index(), renderer_index);
        }
    }

    #[test]
    fn standard_8_directions_count() {
        let dirs = standard_8_directions();
        assert_eq!(dirs.len(), 8);
    }

    #[test]
    fn standard_8_directions_mirror_indices() {
        let dirs = standard_8_directions();
        // Order: 0=front, 1=frontright(flip), 2=right(flip), 3=backleft,
        //        4=back, 5=backright(flip), 6=left, 7=frontleft
        // frontright mirrors frontleft (phys idx 1)
        assert_eq!(
            dirs[1].source_direction_idx,
            SpriteDirection::FrontLeft.physical_index()
        );
        assert!(dirs[1].flip_x);
        // right mirrors left (phys idx 2)
        assert_eq!(
            dirs[2].source_direction_idx,
            SpriteDirection::Left.physical_index()
        );
        assert!(dirs[2].flip_x);
        // backright mirrors backleft (phys idx 3)
        assert_eq!(
            dirs[5].source_direction_idx,
            SpriteDirection::BackLeft.physical_index()
        );
        assert!(dirs[5].flip_x);
        // Physical directions should NOT flip.
        assert!(!dirs[0].flip_x); // front
        assert!(!dirs[3].flip_x); // backleft
        assert!(!dirs[4].flip_x); // back
        assert!(!dirs[6].flip_x); // left
        assert!(!dirs[7].flip_x); // frontleft
    }

    #[test]
    fn compute_frame_forward_loops() {
        let anim = BillboardAnimation {
            playback: Playback::Forward,
            frame_order: FrameOrder::Normal,
            frame_count: 4,
            duration_secs: 0.8,
        };
        assert_eq!(compute_frame_index(0.0, &anim), 0);
        assert_eq!(compute_frame_index(0.2, &anim), 1);
        assert_eq!(compute_frame_index(0.4, &anim), 2);
        assert_eq!(compute_frame_index(0.6, &anim), 3);
        // Loops
        assert_eq!(compute_frame_index(0.8, &anim), 0);
        assert_eq!(compute_frame_index(1.0, &anim), 1);
    }

    #[test]
    fn compute_frame_pingpong() {
        let anim = BillboardAnimation {
            playback: Playback::PingPong,
            frame_order: FrameOrder::Normal,
            frame_count: 2,
            duration_secs: 0.4,
        };
        assert_eq!(compute_frame_index(0.0, &anim), 0);
        assert_eq!(compute_frame_index(0.1, &anim), 0);
        assert_eq!(compute_frame_index(0.3, &anim), 1);
        // Reverse phase
        assert_eq!(compute_frame_index(0.5, &anim), 1);
        assert_eq!(compute_frame_index(0.7, &anim), 0);
    }

    #[test]
    fn compute_frame_reversed_order() {
        let anim = BillboardAnimation {
            playback: Playback::Forward,
            frame_order: FrameOrder::Reversed,
            frame_count: 4,
            duration_secs: 0.8,
        };
        // Frame order reversed: logical 0→3, 1→2, 2→1, 3→0.
        assert_eq!(compute_frame_index(0.0, &anim), 3);
        assert_eq!(compute_frame_index(0.2, &anim), 2);
        assert_eq!(compute_frame_index(0.4, &anim), 1);
        assert_eq!(compute_frame_index(0.6, &anim), 0);
    }

    #[test]
    fn compute_frame_reverse_playback_starts_on_last_frame() {
        let anim = BillboardAnimation {
            playback: Playback::Reverse,
            frame_order: FrameOrder::Normal,
            frame_count: 4,
            duration_secs: 0.8,
        };
        // Reverse: 3, 2, 1, 0 (each for 0.2s).
        assert_eq!(compute_frame_index(0.0, &anim), 3);
        assert_eq!(compute_frame_index(0.19, &anim), 3);
        assert_eq!(compute_frame_index(0.2, &anim), 2);
        assert_eq!(compute_frame_index(0.4, &anim), 1);
        assert_eq!(compute_frame_index(0.6, &anim), 0);
        // Loops back to 3.
        assert_eq!(compute_frame_index(0.8, &anim), 3);
    }

    #[test]
    fn validate_coverage_rejects_missing_required_action() {
        let animations = HashMap::new();
        let sprites = HashMap::new();

        let error = validate_directional_action_coverage(&animations, &sprites, &["idle_stand"])
            .expect_err("required action should be enforced");

        assert!(error.contains("missing required billboard action"));
    }

    #[test]
    fn validate_coverage_rejects_missing_physical_direction() {
        let mut animations = HashMap::new();
        animations.insert(
            "idle_stand".to_string(),
            BillboardAnimation {
                playback: Playback::Forward,
                frame_order: FrameOrder::Normal,
                frame_count: 1,
                duration_secs: 0.2,
            },
        );

        let frame = Arc::new(CxImage::new(vec![1], 1));
        let mut dir_frames = vec![
            DirectionFrames {
                frames: vec![Arc::clone(&frame)],
            };
            NUM_PHYSICAL_DIRECTIONS
        ];
        dir_frames[SpriteDirection::Left.physical_index()]
            .frames
            .clear();

        let mut sprites = HashMap::new();
        sprites.insert("idle_stand".to_string(), dir_frames);

        let error = validate_directional_action_coverage(&animations, &sprites, &["idle_stand"])
            .expect_err("empty physical direction should be rejected");

        assert!(error.contains("direction 2 has no frames"));
    }

    #[test]
    fn full_resolve_front_idle() {
        let dirs = standard_8_directions();
        let frame = Arc::new(CxImage::new(vec![1; 4], 2));

        let mut sprites = HashMap::new();
        let mut action_sprites = Vec::new();
        for _ in 0..5 {
            action_sprites.push(DirectionFrames {
                frames: vec![Arc::clone(&frame), Arc::clone(&frame)],
            });
        }
        sprites.insert("idle_stand".to_string(), action_sprites);

        let mut animations = HashMap::new();
        animations.insert(
            "idle_stand".to_string(),
            BillboardAnimation {
                playback: Playback::Forward,
                frame_order: FrameOrder::Normal,
                frame_count: 2,
                duration_secs: 0.4,
            },
        );

        let atlas = DirectionalBillboardAtlas {
            directions: dirs,
            animations,
            sprites,
        };

        let state = BillboardAnimationState::new("idle_stand");

        // Observer in front of target
        let observer = bevy_math::Vec2::new(5.0, 0.0);
        let target = bevy_math::Vec2::new(0.0, 0.0);
        let resolved = resolve_billboard(&atlas, observer, target, 0.0, &state).unwrap();
        assert!(!resolved.flip_x, "front should not flip");
    }

    #[test]
    fn full_resolve_right_flips() {
        let dirs = standard_8_directions();
        let frame = Arc::new(CxImage::new(vec![1; 4], 2));

        let mut sprites = HashMap::new();
        let mut action_sprites = Vec::new();
        for _ in 0..5 {
            action_sprites.push(DirectionFrames {
                frames: vec![Arc::clone(&frame)],
            });
        }
        sprites.insert("idle_stand".to_string(), action_sprites);

        let mut animations = HashMap::new();
        animations.insert(
            "idle_stand".to_string(),
            BillboardAnimation {
                playback: Playback::Forward,
                frame_order: FrameOrder::Normal,
                frame_count: 1,
                duration_secs: 0.4,
            },
        );

        let atlas = DirectionalBillboardAtlas {
            directions: dirs,
            animations,
            sprites,
        };

        let state = BillboardAnimationState::new("idle_stand");

        // Observer north of east-facing target → angle PI/2 → direction "right"
        // (flip of physical "left") → flip_x=true.
        let observer = bevy_math::Vec2::new(0.0, 5.0);
        let target = bevy_math::Vec2::new(0.0, 0.0);
        let resolved = resolve_billboard(&atlas, observer, target, 0.0, &state).unwrap();
        assert!(resolved.flip_x, "right should flip (mirrors left)");
    }

    #[test]
    fn load_player_atlas_succeeds() {
        let atlas = make_player_billboard_atlas().expect("player atlas should load");
        assert_eq!(atlas.directions.len(), 8);

        // Should have idle_stand, walk_forward, death actions.
        assert!(
            atlas.animations.contains_key("idle_stand"),
            "missing idle_stand animation"
        );
        assert!(
            atlas.animations.contains_key("walk_forward"),
            "missing walk_forward animation"
        );
        assert!(
            atlas.animations.contains_key("death"),
            "missing death animation"
        );

        // idle_stand should have 2 frames.
        assert_eq!(atlas.animations["idle_stand"].frame_count, 2);
        // walk_forward should have 4 frames.
        assert_eq!(atlas.animations["walk_forward"].frame_count, 4);
        // death should have 1 frame.
        assert_eq!(atlas.animations["death"].frame_count, 1);

        // Each action should have sprites for at least the 5 physical directions.
        // Actions with custom virtual direction layer ordering may have more slots.
        for action in ["idle_stand", "walk_forward", "death"] {
            let dir_frames = &atlas.sprites[action];
            assert!(
                dir_frames.len() >= NUM_PHYSICAL_DIRECTIONS,
                "action '{action}' should have at least {NUM_PHYSICAL_DIRECTIONS} directions, got {}",
                dir_frames.len()
            );
            for (i, frame_data) in dir_frames.iter().enumerate().take(NUM_PHYSICAL_DIRECTIONS) {
                assert!(
                    !frame_data.frames.is_empty(),
                    "action '{action}' physical direction {i} has no frames"
                );
            }
        }
    }

    #[test]
    fn player_atlas_resolve_all_directions() {
        let atlas = make_player_billboard_atlas().expect("player atlas should load");
        let state = BillboardAnimationState::new("idle_stand");

        // Test all 8 directions resolve without error.
        let target = bevy_math::Vec2::new(0.0, 0.0);
        let target_angle = 0.0;

        let test_positions = [
            (bevy_math::Vec2::new(5.0, 0.0), "front"),
            (bevy_math::Vec2::new(3.5, 3.5), "frontright"),
            (bevy_math::Vec2::new(0.0, 5.0), "right"),
            (bevy_math::Vec2::new(-3.5, 3.5), "backleft"),
            (bevy_math::Vec2::new(-5.0, 0.0), "back"),
            (bevy_math::Vec2::new(-3.5, -3.5), "backright"),
            (bevy_math::Vec2::new(0.0, -5.0), "left"),
            (bevy_math::Vec2::new(3.5, -3.5), "frontleft"),
        ];

        for (observer_pos, dir_name) in &test_positions {
            let resolved = resolve_billboard(&atlas, *observer_pos, target, target_angle, &state);
            assert!(
                resolved.is_some(),
                "failed to resolve direction '{dir_name}'"
            );
        }

        // Observer north (+Y) → angle PI/2 → "right" (flip of left).
        let right = resolve_billboard(
            &atlas,
            bevy_math::Vec2::new(0.0, 5.0),
            target,
            target_angle,
            &state,
        )
        .unwrap();
        assert!(right.flip_x, "right should flip");

        // Observer south (-Y) → angle 3PI/2 → "left" (physical, no flip).
        let left = resolve_billboard(
            &atlas,
            bevy_math::Vec2::new(0.0, -5.0),
            target,
            target_angle,
            &state,
        )
        .unwrap();
        assert!(!left.flip_x, "left should not flip");
    }

    #[test]
    fn player_atlas_has_unarmed_idle() {
        let atlas = make_player_billboard_atlas().expect("player atlas should load");
        assert!(
            atlas.animations.contains_key("idle_unarmed_stand"),
            "missing idle_unarmed_stand animation"
        );
        assert_eq!(
            atlas.animations["idle_unarmed_stand"].playback,
            Playback::PingPong,
            "unarmed idle should use PingPong"
        );
    }

    #[test]
    fn player_atlas_excludes_dev_prefixed_tags() {
        let atlas = make_player_billboard_atlas().expect("player atlas should load");
        assert!(
            !atlas.animations.contains_key("idle_stand_wrong_arm"),
            "_-prefixed source tags must not become runtime actions"
        );
    }

    #[test]
    fn colour_groups_indices_exist_in_atlas() {
        let atlas = make_player_billboard_atlas().expect("player atlas should load");
        let groups = crate::avatar_palette::colour_groups();
        let prot = crate::avatar_palette::protected_indices();
        let mut used = [false; 16];

        for frames in atlas.sprites.values().flat_map(|dirs| dirs.iter()) {
            for frame in &frames.frames {
                for &px in frame.data() {
                    if px != 0 {
                        used[px as usize] = true;
                    }
                }
            }
        }

        for &g in groups {
            assert!(
                used[g as usize],
                "colour-groups index {g} does not appear in any player billboard frame — \
                 phantom index will produce invisible pixels when remapped"
            );
        }

        for &p in prot {
            assert!(
                !groups.contains(&p),
                "protected index {p} is also in colour-groups — \
                 protected pass will override the remap, wasting a colour group slot"
            );
        }
    }
}
