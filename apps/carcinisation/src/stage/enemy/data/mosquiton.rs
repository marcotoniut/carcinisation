//! Canonical Mosquiton animation tags exported from the composed Aseprite source.
//!
//! Gameplay currently uses only the airborne subset because Mosquiton reuses the
//! mosquito behaviour model. The gallery exposes the full authored tag list.

use crate::stage::enemy::composed::{ComposedAnimationOverride, ComposedAnimationState};

/// Hovering idle used during normal airborne gameplay.
pub const TAG_IDLE_FLY: &str = "idle_fly";
/// Ranged-attack animation used by the reused mosquito shooting behaviour.
pub const TAG_SHOOT_FLY: &str = "shoot_fly";
/// Melee attack animation used if Mosquiton ever enters mosquito melee mode.
pub const TAG_MELEE_FLY: &str = "melee_fly";
/// One-shot death preview exposed in the gallery.
pub const TAG_DEATH_FLY: &str = "death_fly";
/// Falling animation used when wings are destroyed.
pub const TAG_FALLING: &str = "falling";

const MOSQUITON_WING_PART_TAGS: &[&str] = &["wings"];

/// Full authored tag list exposed in the gallery.
pub const GALLERY_TAGS: &[&str] = &[
    TAG_IDLE_FLY,
    TAG_SHOOT_FLY,
    TAG_MELEE_FLY,
    TAG_DEATH_FLY,
    TAG_FALLING,
    "liftoff",
    "idle_stand",
    "walking_forward",
    "shoot_stand",
    "melee_stand",
    "death",
];

/// Core flying-action tags surfaced as deterministic gallery verification controls.
pub const GALLERY_ACTION_TAGS: &[&str] =
    &[TAG_IDLE_FLY, TAG_SHOOT_FLY, TAG_MELEE_FLY, TAG_DEATH_FLY];

/// Applies the canonical Mosquiton composed-animation request for a semantic tag.
///
/// Flying gameplay/actions keep the wing loop sourced from [`TAG_IDLE_FLY`]
/// while the base tag drives the rest of the body. Non-flying tags use no
/// override tracks, which keeps authored monolithic previews explicit.
pub fn apply_mosquiton_animation_state(
    animation_state: &mut ComposedAnimationState,
    requested_tag: &str,
) {
    if animation_state.requested_tag != requested_tag {
        animation_state.requested_tag.clear();
        animation_state.requested_tag.push_str(requested_tag);
    }

    let overrides = if uses_flying_wing_override(requested_tag) {
        vec![ComposedAnimationOverride::for_part_tags(
            TAG_IDLE_FLY,
            MOSQUITON_WING_PART_TAGS.iter().copied(),
        )]
    } else {
        Vec::new()
    };

    if animation_state.part_overrides != overrides {
        animation_state.set_part_overrides(overrides);
    }
}

#[must_use]
pub fn uses_flying_wing_override(tag: &str) -> bool {
    matches!(tag, TAG_IDLE_FLY | TAG_SHOOT_FLY | TAG_MELEE_FLY)
    // Note: TAG_FALLING is intentionally excluded - wingless animation has no wing track
}
