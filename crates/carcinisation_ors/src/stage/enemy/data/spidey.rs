//! Spidey animation actions and request helpers.
//!
//! Gameplay uses semantic action names (`ACTION_*`) — physical atlas tags are
//! resolved internally via [`carcinisation_base::direction::SpriteDirection`].

use carcinisation_base::direction::SpriteDirection;

use crate::stage::enemy::composed::ComposedAnimationState;

/// ORS direction — all ORS rendering uses front-facing sprites.
const ORS_DIRECTION: SpriteDirection = SpriteDirection::Front;

// ── Semantic action constants ───────────────────────────────────────────────

/// Resting idle pose with all legs planted.
pub const ACTION_IDLE: &str = "idle";
/// Lounging pose — legs sourced from idle via atlas metadata `part_overrides`.
pub const ACTION_LOUNGE: &str = "lounge";
/// Ranged attack (web shot) animation.
pub const ACTION_SHOOT: &str = "shoot";
/// Jump locomotion used for depth traversal.
pub const ACTION_JUMP: &str = "jump";
/// Landing freeze frame after a jump arc.
pub const ACTION_LANDING: &str = "landing";

// ── Gallery ─────────────────────────────────────────────────────────────────

/// Full authored action list exposed in the gallery.
pub const GALLERY_ACTIONS: &[&str] = &[
    ACTION_IDLE,
    ACTION_LOUNGE,
    ACTION_SHOOT,
    ACTION_JUMP,
    ACTION_LANDING,
];

/// Core actions surfaced as deterministic gallery verification controls.
pub const GALLERY_VERIFICATION_ACTIONS: &[&str] = &[
    ACTION_IDLE,
    ACTION_LOUNGE,
    ACTION_SHOOT,
    ACTION_JUMP,
    ACTION_LANDING,
];

// ── Action request helpers ──────────────────────────────────────────────────

/// Request a Spidey animation by semantic action name.
///
/// Resolves the action to a physical atlas tag via [`ORS_DIRECTION`].
/// Spidey has no runtime part overrides — lounge leg overrides are declared
/// in the atlas metadata and resolved generically by the composed renderer.
pub fn request_spidey_action(animation_state: &mut ComposedAnimationState, action: &str) {
    request_spidey_action_with_hold(animation_state, action, false);
}

/// Request a Spidey animation and optionally freeze on the terminal frame.
pub fn request_spidey_action_with_hold(
    animation_state: &mut ComposedAnimationState,
    action: &str,
    hold_last_frame: bool,
) {
    let tag = ORS_DIRECTION.tag_name(action);

    if animation_state.requested_tag != tag {
        animation_state.requested_tag.clear();
        animation_state.requested_tag.push_str(&tag);
    }
    animation_state.set_hold_last_frame(hold_last_frame);

    if !animation_state.part_overrides.is_empty() {
        animation_state.set_part_overrides(Vec::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_action_sets_front_physical_tag() {
        let mut state = ComposedAnimationState::new("placeholder");
        request_spidey_action(&mut state, ACTION_IDLE);
        assert_eq!(state.requested_tag, "front_idle");
    }

    #[test]
    fn request_jump_with_hold_sets_flag() {
        let mut state = ComposedAnimationState::new("placeholder");
        request_spidey_action_with_hold(&mut state, ACTION_JUMP, true);
        assert_eq!(state.requested_tag, "front_jump");
        assert!(state.hold_last_frame);
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
