//! Canonical Spidey animation tags exported from the composed Aseprite source.
//!
//! Spidey is a ground-based arachnid enemy that uses jump locomotion for depth
//! traversal. The gallery exposes the full authored tag list.

use crate::stage::enemy::composed::ComposedAnimationState;

/// Resting idle pose with all legs planted.
pub const TAG_IDLE: &str = "idle";
/// Lounging pose - legs sourced from idle via atlas metadata `part_overrides`.
pub const TAG_LOUNGE: &str = "lounge";
/// Ranged attack (web shot) animation.
pub const TAG_SHOOT: &str = "shoot";
/// Jump locomotion used for depth traversal.
pub const TAG_JUMP: &str = "jump";
/// Landing freeze frame after a jump arc.
pub const TAG_LANDING: &str = "landing";

/// Full authored tag list exposed in the gallery.
pub const GALLERY_TAGS: &[&str] = &[TAG_IDLE, TAG_LOUNGE, TAG_SHOOT, TAG_JUMP, TAG_LANDING];

/// Core action tags surfaced as deterministic gallery verification controls.
pub const GALLERY_ACTION_TAGS: &[&str] = &[TAG_IDLE, TAG_LOUNGE, TAG_SHOOT, TAG_JUMP, TAG_LANDING];

/// Applies the canonical Spidey composed-animation request for a semantic tag.
///
/// Spidey has no runtime part overrides -- lounge leg overrides are declared
/// in the atlas metadata and resolved generically by the composed renderer.
pub fn apply_spidey_animation_state(
    animation_state: &mut ComposedAnimationState,
    requested_tag: &str,
) {
    apply_spidey_animation_state_with_hold(animation_state, requested_tag, false);
}

/// Applies a Spidey animation request and optionally freezes it on the tag's
/// terminal frame.
pub fn apply_spidey_animation_state_with_hold(
    animation_state: &mut ComposedAnimationState,
    requested_tag: &str,
    hold_last_frame: bool,
) {
    if animation_state.requested_tag != requested_tag {
        animation_state.requested_tag.clear();
        animation_state.requested_tag.push_str(requested_tag);
    }
    animation_state.set_hold_last_frame(hold_last_frame);

    if !animation_state.part_overrides.is_empty() {
        animation_state.set_part_overrides(Vec::new());
    }
}
