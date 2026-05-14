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
use std::f32::consts::{PI, TAU};
use std::sync::Arc;

use carapace::image::CxImage;

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
    /// Human-readable name (e.g. "frontleft", "backright").
    pub name: String,
    /// Canonical angle in radians, 0 = front (facing observer), increases CCW.
    pub angle_rad: f32,
    /// Index into the physical direction's sprite data.
    /// For mirrored directions, this points to the source direction.
    pub source_direction_idx: usize,
    /// Whether to horizontally flip the sprite at render time.
    pub flip_x: bool,
}

/// Animation definition for one action (e.g. "idle_stand", "walk_forward").
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
    /// Current action name (e.g. "idle_stand", "walk_forward", "death").
    pub action: String,
    /// Elapsed time in seconds since this action started.
    pub elapsed: f32,
    /// Future: palette variant for player differentiation.
    pub skin_id: Option<u8>,
}

impl BillboardAnimationState {
    #[must_use]
    pub fn new(action: &str) -> Self {
        Self {
            action: action.to_string(),
            elapsed: 0.0,
            skin_id: None,
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
pub fn quantize_direction(relative_angle: f32, num_directions: usize) -> usize {
    let sector = TAU / num_directions as f32;
    let shifted = normalize_angle(relative_angle + sector / 2.0);
    let idx = (shifted / sector) as usize;
    idx.min(num_directions - 1)
}

/// Resolve the billboard sprite for a given observer-target relationship.
///
/// Returns `None` if the action or direction data is missing.
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

    let frames = action_sprites.get(dir.source_direction_idx)?;
    if frames.frames.is_empty() {
        return None;
    }

    let frame_idx = compute_frame_index(state.elapsed, anim);
    let sprite = Arc::clone(frames.frames.get(frame_idx)?);

    Some(ResolvedBillboard {
        sprite,
        flip_x: dir.flip_x,
    })
}

/// Compute the animation frame index from elapsed time and animation config.
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

fn apply_frame_order(idx: usize, frame_count: usize, order: FrameOrder) -> usize {
    match order {
        FrameOrder::Normal => idx,
        FrameOrder::Reversed => frame_count - 1 - idx,
    }
}

/// Standard 8-direction layout: 5 physical, 3 mirrored.
///
/// Physical direction indices:
///   0 = front (0°)
///   1 = frontleft (45°)
///   2 = left (90°)
///   3 = backleft (135°)
///   4 = back (180°)
pub fn standard_8_directions() -> Vec<BillboardDirection> {
    // Physical direction indices:
    //   0 = front, 1 = frontleft, 2 = left, 3 = backleft, 4 = back
    //
    // The relative viewing angle increases CCW: 0=front, PI/2=left, PI=back.
    // In the game's coordinate system (0=east, CCW positive), observer north
    // of an east-facing target has angle PI/2 which maps to the "left" physical
    // sprite (the observer sees the character's LEFT side).
    //
    // Mirrored right-side directions use the corresponding left-side physical
    // sprites with flip_x=true.
    vec![
        BillboardDirection {
            name: "front".into(),
            angle_rad: 0.0,
            source_direction_idx: 0,
            flip_x: false,
        },
        BillboardDirection {
            name: "frontright".into(),
            angle_rad: PI / 4.0,
            source_direction_idx: 1, // mirror of frontleft
            flip_x: true,
        },
        BillboardDirection {
            name: "right".into(),
            angle_rad: PI / 2.0,
            source_direction_idx: 2, // mirror of left
            flip_x: true,
        },
        BillboardDirection {
            name: "backleft".into(),
            angle_rad: 3.0 * PI / 4.0,
            source_direction_idx: 3,
            flip_x: false,
        },
        BillboardDirection {
            name: "back".into(),
            angle_rad: PI,
            source_direction_idx: 4,
            flip_x: false,
        },
        BillboardDirection {
            name: "backright".into(),
            angle_rad: 5.0 * PI / 4.0,
            source_direction_idx: 3, // mirror of backleft
            flip_x: true,
        },
        BillboardDirection {
            name: "left".into(),
            angle_rad: 3.0 * PI / 2.0,
            source_direction_idx: 2,
            flip_x: false,
        },
        BillboardDirection {
            name: "frontleft".into(),
            angle_rad: 7.0 * PI / 4.0,
            source_direction_idx: 1,
            flip_x: false,
        },
    ]
}

/// Number of physical (atlas-backed) directions.
pub const NUM_PHYSICAL_DIRECTIONS: usize = 5;

/// Physical direction index constants.
pub const DIR_FRONT: usize = 0;
pub const DIR_FRONTLEFT: usize = 1;
pub const DIR_LEFT: usize = 2;
pub const DIR_BACKLEFT: usize = 3;
pub const DIR_BACK: usize = 4;

/// Tag-name prefix → physical direction index.
/// Sorted longest-first to prevent ambiguous prefix matches
/// (e.g. "frontleft" must be tried before "front").
const DIRECTION_TAG_MAP: [(&str, usize); 5] = [
    ("frontleft", DIR_FRONTLEFT),
    ("backleft", DIR_BACKLEFT),
    ("front", DIR_FRONT),
    ("left", DIR_LEFT),
    ("back", DIR_BACK),
];

type ActionDirectionFrames = HashMap<String, Vec<(usize, Vec<Arc<CxImage>>)>>;

fn parse_directional_tag(tag_name: &str) -> Option<(usize, String)> {
    for &(prefix, dir_idx) in &DIRECTION_TAG_MAP {
        if let Some(rest) = tag_name.strip_prefix(prefix)
            && let Some(action) = rest.strip_prefix('_')
        {
            return Some((dir_idx, action.to_string()));
        }
    }
    None
}

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
        if dir_frames.len() != NUM_PHYSICAL_DIRECTIONS {
            return Err(format!(
                "action '{}' has {} physical directions, expected {}",
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
        for (i, df) in dir_frames.iter().enumerate() {
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
    load_directional_billboard_atlas_with_required_actions(
        composed_ron,
        px_atlas_ron,
        pxi_bytes,
        &[],
    )
}

fn load_directional_billboard_atlas_with_required_actions(
    composed_ron: &str,
    px_atlas_ron: &str,
    pxi_bytes: &[u8],
    required_actions: &[&str],
) -> Result<DirectionalBillboardAtlas, String> {
    use crate::mosquiton::{
        PxAtlasDescriptor, compose_tag_frames, compose_tag_frames_reversed, decode_pxi,
    };
    use asset_pipeline::composed_ron::{CompactComposedAtlas, CompactDirection};

    let composed: CompactComposedAtlas = ron::from_str(composed_ron)
        .map_err(|err| format!("Failed to parse composed.ron: {err}"))?;
    let atlas: PxAtlasDescriptor = ron::from_str(px_atlas_ron)
        .map_err(|err| format!("Failed to parse px_atlas.ron: {err}"))?;
    let (atlas_width, _atlas_height, atlas_pixels) = decode_pxi(pxi_bytes)?;

    // Parse composed animation tags into (direction, action) pairs.
    let mut action_direction_frames: ActionDirectionFrames = HashMap::new();
    let mut action_animations: HashMap<String, BillboardAnimation> = HashMap::new();

    for anim in &composed.animations {
        let tag_name = &anim.tag;

        // Skip dev/internal tags.
        if tag_name.starts_with('_') {
            continue;
        }

        let Some((dir_idx, action)) = parse_directional_tag(tag_name) else {
            continue; // Unrecognized tag format, skip.
        };

        // Compose per-part sprites into full-frame CxImages.
        // Pure back direction reverses draw order so the body renders on top
        // of arms/head/weapon (viewer sees the character's back).
        // backleft/backright are angled views that keep normal draw order.
        let frames = if dir_idx == DIR_BACK {
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

    validate_directional_action_coverage(&action_animations, &sprites, required_actions)?;

    Ok(DirectionalBillboardAtlas {
        directions: standard_8_directions(),
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
/// full-frame billboard sprites at load time. Uses the same composed atlas
/// pipeline as the Mosquiton.
///
/// # Errors
///
/// Returns an error if the embedded assets are malformed.
pub fn make_player_billboard_atlas() -> Result<DirectionalBillboardAtlas, String> {
    load_directional_billboard_atlas_with_required_actions(
        PLAYER_COMPOSED_RON,
        PLAYER_PX_ATLAS_RON,
        PLAYER_PXI,
        REQUIRED_PLAYER_ACTIONS,
    )
}

#[cfg(test)]
mod tests {
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
    fn quantize_8_directions_slight_left() {
        // Angle just past 22.5° should still be frontleft (direction 1).
        let angle = PI / 4.0 + 0.01;
        assert_eq!(quantize_direction(angle, 8), 1);
    }

    #[test]
    fn quantize_8_directions_boundary() {
        // Angle exactly at boundary between front and frontleft (22.5°) rounds to frontleft.
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
        assert_eq!(dirs[1].source_direction_idx, DIR_FRONTLEFT);
        assert!(dirs[1].flip_x);
        // right mirrors left (phys idx 2)
        assert_eq!(dirs[2].source_direction_idx, DIR_LEFT);
        assert!(dirs[2].flip_x);
        // backright mirrors backleft (phys idx 3)
        assert_eq!(dirs[5].source_direction_idx, DIR_BACKLEFT);
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
        dir_frames[DIR_LEFT].frames.clear();

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

        // Each action should have sprites for all 5 physical directions.
        for action in ["idle_stand", "walk_forward", "death"] {
            let dir_frames = &atlas.sprites[action];
            assert_eq!(
                dir_frames.len(),
                NUM_PHYSICAL_DIRECTIONS,
                "action '{action}' should have {NUM_PHYSICAL_DIRECTIONS} physical directions"
            );
            for (i, df) in dir_frames.iter().enumerate() {
                assert!(
                    !df.frames.is_empty(),
                    "action '{action}' direction {i} has no frames"
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
            (bevy_math::Vec2::new(3.5, 3.5), "frontleft"),
            (bevy_math::Vec2::new(0.0, 5.0), "left"),
            (bevy_math::Vec2::new(-3.5, 3.5), "backleft"),
            (bevy_math::Vec2::new(-5.0, 0.0), "back"),
            (bevy_math::Vec2::new(-3.5, -3.5), "backright"),
            (bevy_math::Vec2::new(0.0, -5.0), "right"),
            (bevy_math::Vec2::new(3.5, -3.5), "frontright"),
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
}
