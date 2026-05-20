//! Mosquiton animation actions and request helpers.
//!
//! Gameplay uses semantic action names (`ACTION_*`) — physical atlas tags are
//! resolved internally via [`carcinisation_base::direction::SpriteDirection`].

use carcinisation_base::direction::SpriteDirection;

use crate::stage::enemy::composed::{ComposedAnimationOverride, ComposedAnimationState};

/// ORS direction — all ORS rendering uses front-facing sprites.
const ORS_DIRECTION: SpriteDirection = SpriteDirection::Front;

// ── Semantic action constants ───────────────────────────────────────────────

/// Hovering idle used during normal airborne gameplay.
pub const ACTION_IDLE_FLY: &str = "idle_fly";
/// Ranged-attack animation used by the reused mosquito shooting behaviour.
pub const ACTION_SHOOT_FLY: &str = "shoot_fly";
/// Melee attack animation used if Mosquiton ever enters mosquito melee mode.
pub const ACTION_MELEE_FLY: &str = "melee_fly";
/// One-shot death preview exposed in the gallery.
pub const ACTION_DEATH_FLY: &str = "death_fly";
/// Falling animation used when wings are destroyed.
pub const ACTION_FALL: &str = "fall";
/// Grounded idle used after a wing-loss landing.
pub const ACTION_IDLE_STAND: &str = "idle_stand";

pub const MOSQUITON_WING_PART_TAGS: &[&str] = &["wings"];

// ── Gallery ─────────────────────────────────────────────────────────────────

/// Full authored action list exposed in the gallery.
pub const GALLERY_ACTIONS: &[&str] = &[
    ACTION_IDLE_FLY,
    ACTION_SHOOT_FLY,
    ACTION_MELEE_FLY,
    ACTION_DEATH_FLY,
    ACTION_FALL,
    "liftoff",
    "idle_stand",
    "walk_forward",
    "dance",
    "shoot_stand",
    "melee_stand",
    "death",
];

/// Core flying actions surfaced as deterministic gallery verification controls.
pub const GALLERY_VERIFICATION_ACTIONS: &[&str] = &[
    ACTION_IDLE_FLY,
    ACTION_SHOOT_FLY,
    ACTION_MELEE_FLY,
    ACTION_DEATH_FLY,
];

// ── Action request helpers ──────────────────────────────────────────────────

/// Request a Mosquiton animation by semantic action name.
///
/// Resolves the action to a physical atlas tag via [`ORS_DIRECTION`] and
/// applies the canonical wing-override stack for flying actions.
pub fn request_mosquiton_action(animation_state: &mut ComposedAnimationState, action: &str) {
    let tag = ORS_DIRECTION.tag_name(action);
    let wing_source_tag = ORS_DIRECTION.tag_name(ACTION_IDLE_FLY);

    if animation_state.requested_tag != tag {
        animation_state.requested_tag.clear();
        animation_state.requested_tag.push_str(&tag);
    }

    let overrides = if is_flying_action(action) {
        vec![ComposedAnimationOverride::for_part_tags(
            &wing_source_tag,
            MOSQUITON_WING_PART_TAGS.iter().copied(),
        )]
    } else {
        Vec::new()
    };

    if animation_state.part_overrides != overrides {
        animation_state.set_part_overrides(overrides);
    }
}

/// Whether this action uses the flying wing override (`idle_fly` wing cycle).
#[must_use]
pub fn is_flying_action(action: &str) -> bool {
    matches!(
        action,
        ACTION_IDLE_FLY | ACTION_SHOOT_FLY | ACTION_MELEE_FLY
    )
    // Note: ACTION_FALL is intentionally excluded — wingless animation has no wing track
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_action_sets_front_physical_tag() {
        let mut state = ComposedAnimationState::new("placeholder");
        request_mosquiton_action(&mut state, ACTION_IDLE_FLY);
        assert_eq!(state.requested_tag, "front_idle_fly");
    }

    #[test]
    fn request_flying_action_adds_wing_override() {
        let mut state = ComposedAnimationState::new("placeholder");
        request_mosquiton_action(&mut state, ACTION_SHOOT_FLY);
        assert_eq!(state.part_overrides.len(), 1);
        assert_eq!(state.part_overrides[0].tag, "front_idle_fly");
    }

    #[test]
    fn request_non_flying_action_clears_overrides() {
        let mut state = ComposedAnimationState::new("placeholder");
        request_mosquiton_action(&mut state, ACTION_SHOOT_FLY);
        assert_eq!(state.part_overrides.len(), 1);
        request_mosquiton_action(&mut state, ACTION_FALL);
        assert!(state.part_overrides.is_empty());
    }

    #[test]
    fn gallery_actions_are_semantic() {
        for action in GALLERY_ACTIONS {
            assert!(
                !action.starts_with("front_"),
                "gallery action '{action}' should be semantic, not a physical tag"
            );
        }
    }
}
