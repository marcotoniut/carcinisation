//! Player input resolution layer.
//!
//! Separates raw [`GBInput`] button state from semantic player actions.
//! The chord resolver disambiguates Select+A (melee) from Select-alone
//! (item select) using a short grace window.
//!
//! Data flow: `GBInput` → [`resolve_player_intent`] → [`PlayerIntent`] → gameplay systems.

use crate::input::GBInput;
use crate::stage::resources::StageTimeDomain;
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

/// Grace window for Select+A melee chord detection.
///
/// 80 ms ≈ 5 frames at 60 fps. Long enough for slightly imprecise chord
/// presses to register; short enough that deliberate sequential presses
/// (Select, pause, then A) stay as item-select followed by shoot.
const MELEE_GRACE_WINDOW_SECS: f32 = 0.08;

/// Resolved player input for the current frame.
///
/// Written by [`resolve_player_intent`] each frame before any gameplay
/// system reads it. Gameplay systems consume this instead of polling
/// raw `GBInput` for the player action path.
#[derive(Resource, Default, Debug)]
#[allow(clippy::struct_excessive_bools)] // intentional input flag struct
pub struct PlayerIntent {
    // --- Continuous state ---
    /// Normalized movement direction (zero when no directional input).
    pub move_direction: Vec2,
    /// Whether the slow-movement modifier (B held) is active.
    pub slow_modifier: bool,

    // --- Discrete one-frame events ---
    /// A pressed outside any chord context → arm ranged attack.
    pub shoot_just_pressed: bool,
    /// A currently held (for hold-type weapons while armed).
    pub shoot_held: bool,
    /// A just released (for release-type weapons while armed).
    pub shoot_just_released: bool,
    /// Select+A chord resolved → trigger melee (Pincer).
    pub melee_triggered: bool,
    /// Select resolved without A → cycle weapon loadout.
    pub item_select_triggered: bool,
}

/// Internal state machine for Select+A chord disambiguation.
#[derive(Resource, Default, Debug)]
pub struct SelectChordState {
    phase: ChordPhase,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
enum ChordPhase {
    /// No chord in progress. A signals forwarded normally.
    #[default]
    Idle,
    /// Select was pressed; waiting for A or grace-window timeout.
    /// A signals are held back during this phase.
    GraceWindow { since: f32 },
    /// Melee resolved. Suppress the current A press/release cycle
    /// so it does not also fire a ranged shot.
    SuppressingA,
}

/// Snapshot of raw button states for a single frame.
///
/// Extracted from [`ActionState<GBInput>`] in the system wrapper and
/// passed to the pure resolver. Tests construct this directly.
#[derive(Default, Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)] // intentional input flag struct
struct RawButtons {
    select_just_pressed: bool,
    a_just_pressed: bool,
    a_pressed: bool,
    a_just_released: bool,
    b_pressed: bool,
    move_direction: Vec2,
}

impl RawButtons {
    fn from_action_state(gb_input: &ActionState<GBInput>) -> Self {
        let raw = Vec2::new(
            (i32::from(gb_input.pressed(&GBInput::Right))
                - i32::from(gb_input.pressed(&GBInput::Left))) as f32,
            (i32::from(gb_input.pressed(&GBInput::Up))
                - i32::from(gb_input.pressed(&GBInput::Down))) as f32,
        );
        Self {
            select_just_pressed: gb_input.just_pressed(&GBInput::Select),
            a_just_pressed: gb_input.just_pressed(&GBInput::A),
            a_pressed: gb_input.pressed(&GBInput::A),
            a_just_released: gb_input.just_released(&GBInput::A),
            b_pressed: gb_input.pressed(&GBInput::B),
            move_direction: if raw.length_squared() > 0.0 {
                raw.normalize_or_zero()
            } else {
                Vec2::ZERO
            },
        }
    }
}

/// Reads raw [`GBInput`] and writes [`PlayerIntent`] each frame.
///
/// Must run before any system that reads [`PlayerIntent`].
pub fn resolve_player_intent(
    gb_input: Res<ActionState<GBInput>>,
    time: Res<Time<StageTimeDomain>>,
    mut chord: ResMut<SelectChordState>,
    mut intent: ResMut<PlayerIntent>,
) {
    let buttons = RawButtons::from_action_state(&gb_input);
    resolve_intent(buttons, time.elapsed_secs(), &mut chord, &mut intent);
}

/// Pure chord resolution logic. Testable without ECS or `ActionState`.
fn resolve_intent(
    buttons: RawButtons,
    now: f32,
    chord: &mut SelectChordState,
    intent: &mut PlayerIntent,
) {
    // Reset all fields each frame.
    *intent = PlayerIntent::default();

    // --- Movement ---
    intent.move_direction = buttons.move_direction;
    intent.slow_modifier = buttons.b_pressed;

    // --- Chord disambiguation ---
    match chord.phase {
        ChordPhase::Idle => {
            if buttons.select_just_pressed {
                if buttons.a_pressed {
                    // A already held when Select arrives → immediate melee.
                    intent.melee_triggered = true;
                    chord.phase = ChordPhase::SuppressingA;
                } else {
                    // No A yet — open grace window.
                    chord.phase = ChordPhase::GraceWindow { since: now };
                }
            } else {
                // Normal A forwarding (no chord context).
                intent.shoot_just_pressed = buttons.a_just_pressed;
                intent.shoot_held = buttons.a_pressed;
                intent.shoot_just_released = buttons.a_just_released;
            }
        }
        ChordPhase::GraceWindow { since } => {
            if buttons.a_just_pressed {
                // A arrived within grace → melee.
                intent.melee_triggered = true;
                chord.phase = ChordPhase::SuppressingA;
            } else if now - since > MELEE_GRACE_WINDOW_SECS {
                // Grace expired → item select.
                intent.item_select_triggered = true;
                chord.phase = ChordPhase::Idle;
            }
            // While in grace window, A signals are NOT forwarded.
        }
        ChordPhase::SuppressingA => {
            // Consume the A release so it never becomes a ranged shot.
            if buttons.a_just_released || !buttons.a_pressed {
                chord.phase = ChordPhase::Idle;
            }
            // Allow rapid melee chaining: Select tapped again while A held.
            if buttons.select_just_pressed && buttons.a_pressed {
                intent.melee_triggered = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_intent() -> PlayerIntent {
        PlayerIntent::default()
    }

    fn no_buttons() -> RawButtons {
        RawButtons::default()
    }

    fn select_just_pressed() -> RawButtons {
        RawButtons {
            select_just_pressed: true,
            ..Default::default()
        }
    }

    fn a_just_pressed() -> RawButtons {
        RawButtons {
            a_just_pressed: true,
            a_pressed: true,
            ..Default::default()
        }
    }

    fn a_held() -> RawButtons {
        RawButtons {
            a_pressed: true,
            ..Default::default()
        }
    }

    fn a_just_released() -> RawButtons {
        RawButtons {
            a_just_released: true,
            ..Default::default()
        }
    }

    fn a_held_select_just_pressed() -> RawButtons {
        RawButtons {
            select_just_pressed: true,
            a_pressed: true,
            ..Default::default()
        }
    }

    // ---------------------------------------------------------------
    // Core chord resolution
    // ---------------------------------------------------------------

    #[test]
    fn a_alone_resolves_to_shoot_never_melee() {
        let mut chord = SelectChordState::default();
        let mut intent = default_intent();

        resolve_intent(a_just_pressed(), 0.0, &mut chord, &mut intent);

        assert!(intent.shoot_just_pressed);
        assert!(intent.shoot_held);
        assert!(!intent.melee_triggered);
        assert!(!intent.item_select_triggered);
    }

    #[test]
    fn select_then_a_within_grace_resolves_to_melee() {
        let mut chord = SelectChordState::default();
        let mut intent = default_intent();

        // Frame 1: Select pressed, no A.
        resolve_intent(select_just_pressed(), 0.0, &mut chord, &mut intent);
        assert!(!intent.melee_triggered);
        assert!(!intent.item_select_triggered);
        assert_eq!(chord.phase, ChordPhase::GraceWindow { since: 0.0 });

        // Frame 2: A pressed within grace window (40 ms < 80 ms).
        intent = default_intent();
        resolve_intent(a_just_pressed(), 0.04, &mut chord, &mut intent);
        assert!(intent.melee_triggered);
        assert!(!intent.shoot_just_pressed);
        assert_eq!(chord.phase, ChordPhase::SuppressingA);
    }

    #[test]
    fn select_alone_resolves_to_item_select_after_grace() {
        let mut chord = SelectChordState::default();
        let mut intent = default_intent();

        // Frame 1: Select pressed.
        resolve_intent(select_just_pressed(), 0.0, &mut chord, &mut intent);

        // Frame 2: Grace expired (100 ms > 80 ms), no A.
        intent = default_intent();
        resolve_intent(no_buttons(), 0.1, &mut chord, &mut intent);
        assert!(intent.item_select_triggered);
        assert!(!intent.melee_triggered);
        assert_eq!(chord.phase, ChordPhase::Idle);
    }

    #[test]
    fn a_held_then_select_resolves_to_immediate_melee() {
        let mut chord = SelectChordState::default();
        let mut intent = default_intent();

        resolve_intent(a_held_select_just_pressed(), 0.0, &mut chord, &mut intent);

        assert!(intent.melee_triggered);
        assert!(!intent.shoot_just_pressed);
        assert!(!intent.item_select_triggered);
        assert_eq!(chord.phase, ChordPhase::SuppressingA);
    }

    #[test]
    fn melee_suppresses_shoot_release() {
        let mut chord = SelectChordState::default();
        let mut intent = default_intent();

        // Trigger melee via A-held + Select.
        resolve_intent(a_held_select_just_pressed(), 0.0, &mut chord, &mut intent);
        assert!(intent.melee_triggered);

        // Next frame: A released while in SuppressingA.
        intent = default_intent();
        resolve_intent(a_just_released(), 0.016, &mut chord, &mut intent);

        // The A release must NOT become a shoot.
        assert!(!intent.shoot_just_released);
        assert!(!intent.shoot_just_pressed);
        assert!(!intent.melee_triggered);
        assert_eq!(chord.phase, ChordPhase::Idle);
    }

    #[test]
    fn melee_suppresses_item_select() {
        let mut chord = SelectChordState::default();
        let mut intent = default_intent();

        // Select pressed → grace window opens.
        resolve_intent(select_just_pressed(), 0.0, &mut chord, &mut intent);
        assert!(!intent.item_select_triggered);

        // A arrives within grace → melee, NOT item select.
        intent = default_intent();
        resolve_intent(a_just_pressed(), 0.02, &mut chord, &mut intent);
        assert!(intent.melee_triggered);
        assert!(!intent.item_select_triggered);
    }

    #[test]
    fn a_after_grace_window_is_normal_shoot() {
        let mut chord = SelectChordState::default();
        let mut intent = default_intent();

        // Frame 1: Select pressed.
        resolve_intent(select_just_pressed(), 0.0, &mut chord, &mut intent);

        // Frame 2: Grace expires → item select.
        intent = default_intent();
        resolve_intent(no_buttons(), 0.1, &mut chord, &mut intent);
        assert!(intent.item_select_triggered);

        // Frame 3: A pressed — normal shoot, not retroactive melee.
        intent = default_intent();
        resolve_intent(a_just_pressed(), 0.15, &mut chord, &mut intent);
        assert!(intent.shoot_just_pressed);
        assert!(!intent.melee_triggered);
    }

    #[test]
    fn b_held_sets_slow_modifier() {
        let buttons = RawButtons {
            b_pressed: true,
            ..Default::default()
        };
        let mut chord = SelectChordState::default();
        let mut intent = default_intent();

        resolve_intent(buttons, 0.0, &mut chord, &mut intent);
        assert!(intent.slow_modifier);
    }

    #[test]
    fn b_not_held_clears_slow_modifier() {
        let mut chord = SelectChordState::default();
        let mut intent = default_intent();

        resolve_intent(no_buttons(), 0.0, &mut chord, &mut intent);
        assert!(!intent.slow_modifier);
    }

    #[test]
    fn directional_input_passes_through() {
        let buttons = RawButtons {
            move_direction: Vec2::new(1.0, 0.0),
            ..Default::default()
        };
        let mut chord = SelectChordState::default();
        let mut intent = default_intent();

        resolve_intent(buttons, 0.0, &mut chord, &mut intent);
        assert_eq!(intent.move_direction, Vec2::new(1.0, 0.0));
    }

    #[test]
    fn rapid_melee_chaining_with_select_retap() {
        let mut chord = SelectChordState::default();
        let mut intent = default_intent();

        // First melee: A held + Select.
        resolve_intent(a_held_select_just_pressed(), 0.0, &mut chord, &mut intent);
        assert!(intent.melee_triggered);
        assert_eq!(chord.phase, ChordPhase::SuppressingA);

        // A still held, Select tapped again → second melee.
        intent = default_intent();
        resolve_intent(a_held_select_just_pressed(), 0.5, &mut chord, &mut intent);
        assert!(intent.melee_triggered);
    }

    #[test]
    fn grace_window_holds_back_a_signals() {
        let mut chord = SelectChordState::default();
        let mut intent = default_intent();

        // Open grace window.
        resolve_intent(select_just_pressed(), 0.0, &mut chord, &mut intent);

        // During grace: A held but NOT forwarded as shoot.
        intent = default_intent();
        resolve_intent(a_held(), 0.02, &mut chord, &mut intent);
        assert!(!intent.shoot_held);
        assert!(!intent.shoot_just_pressed);
    }
}
