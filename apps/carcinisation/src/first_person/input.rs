//! Client input collection: resolves `GBInput` actions into semantic `ClientIntent`.
//!
//! Uses leafwing `ActionState<GBInput>` as the input source — the same
//! abstraction as singleplayer. Physical key mapping is centralized in
//! `carcinisation_input::init_gb_input`.
//!
//! Send policy:
//! - **Immediate on change**: if intent differs from last sent, send now.
//! - **Periodic 30 Hz**: if non-idle, resend at 30 Hz.
//! - **Idle skip**: successive idle frames after an idle was sent are skipped.

use bevy::prelude::*;
use bevy_replicon::prelude::ClientTriggerExt;
use carcinisation_fps::plugin::{
    QuickTurnState, SelectActionOutcome, SelectActionTurnInput, SelectActionTurnState,
    TurnChordInput, TurnChordState, TurnKind, request_snap_turn, resolve_select_action_turn,
    resolve_turn_chord, select_actions_allowed_outside_aim_mode,
};
use carcinisation_input::GBInput;
use carcinisation_net::{ClientIntent, InputSequence, PlayerActions};
use leafwing_input_manager::prelude::ActionState;

use crate::first_person::LocalPlayerId;
use crate::first_person::prediction::PendingInput;

/// Sequence counter persisted across sends.
#[derive(Resource, Default)]
pub struct ClientInputSequence(pub InputSequence);

/// Simulated one-way latency (half RTT) in seconds.
///
/// Set via `CARCINISATION_SIMULATED_PING_MS` env var. The value is treated
/// as full round-trip; half is applied to outgoing packets.
///
/// Only affects `ClientIntent` send timing — prediction runs locally
/// without delay, so the visual gap between predicted and server state
/// becomes visible, which is the point of this tool.
#[derive(Resource)]
pub struct SimulatedLatency {
    half_rtt_secs: f32,
    buffer: Vec<(f32, ClientIntent)>,
}

impl SimulatedLatency {
    /// Read from env var. Returns `None` if unset or zero.
    pub fn from_env() -> Option<Self> {
        let ms: u32 = std::env::var("CARCINISATION_SIMULATED_PING_MS")
            .ok()?
            .parse()
            .ok()?;
        if ms == 0 {
            return None;
        }
        let half_rtt = ms as f32 / 2000.0;
        bevy::log::info!("Simulated latency: {ms}ms RTT ({}ms one-way)", ms / 2);
        Some(Self {
            half_rtt_secs: half_rtt,
            buffer: Vec::new(),
        })
    }

    fn push(&mut self, intent: ClientIntent) {
        self.buffer.push((self.half_rtt_secs, intent));
    }

    fn drain_ready(&mut self, dt: f32) -> Vec<ClientIntent> {
        let mut ready = Vec::new();
        self.buffer.retain_mut(|(remaining, intent)| {
            *remaining -= dt;
            if *remaining <= 0.0 {
                ready.push(intent.clone());
                false
            } else {
                true
            }
        });
        ready
    }
}

/// Timer that gates periodic resends to 30 Hz.
#[derive(Resource)]
pub struct InputSendTimer {
    pub timer: Timer,
    /// True if the last sent intent was idle (for change detection).
    pub last_was_idle: bool,
}

impl Default for InputSendTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0 / 30.0, TimerMode::Repeating),
            last_was_idle: true,
        }
    }
}

const fn set_snap_action(actions: &mut PlayerActions, kind: TurnKind) {
    match kind {
        TurnKind::QuickTurn => actions.set(PlayerActions::QUICK_TURN),
        TurnKind::SideTurnLeft => actions.set(PlayerActions::SNAP_TURN_LEFT),
        TurnKind::SideTurnRight => actions.set(PlayerActions::SNAP_TURN_RIGHT),
    }
}

fn non_aim_movement_from_input(
    action_turn: Option<TurnKind>,
    up_held: bool,
    down_held: bool,
) -> Vec2 {
    let mut movement = Vec2::ZERO;
    if action_turn.is_none() {
        if up_held {
            movement.y += 1.0;
        }
        if down_held {
            movement.y -= 1.0;
        }
    }
    movement
}

/// Collects `GBInput` state, resolves chords, and sends `ClientIntent` to server.
///
/// Uses the same `ActionState<GBInput>` as singleplayer, ensuring key mapping
/// parity. The chord FSM (snap turns) matches the SP `resolve_turn_chord` path.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn collect_and_send_intent(
    mut commands: Commands,
    action: Res<ActionState<GBInput>>,
    mut input_sequence: ResMut<ClientInputSequence>,
    mut send_timer: ResMut<InputSendTimer>,
    time: Res<Time>,
    local_player_id: Res<LocalPlayerId>,
    mut turn_chord: ResMut<TurnChordState>,
    mut select_action_turn: ResMut<SelectActionTurnState>,
    mut quick_turn: ResMut<QuickTurnState>,
    config: Res<carcinisation_fps::plugin::Config>,
    combat_config: Res<carcinisation_fps_core::FpsCombatConfig>,
    mut pending_input: ResMut<PendingInput>,
    mut latency_opt: Option<ResMut<SimulatedLatency>>,
) {
    if local_player_id.0.is_none() {
        return;
    }

    send_timer.timer.tick(time.delta());

    // -- Read GBInput state (same source as SP fps_test.rs handle_input) --
    let b_held = action.pressed(&GBInput::B);
    let up_held = action.pressed(&GBInput::Up);
    let down_held = action.pressed(&GBInput::Down);
    let left_held = action.pressed(&GBInput::Left);
    let right_held = action.pressed(&GBInput::Right);
    let a_held = action.pressed(&GBInput::A);
    let a_just_pressed = action.just_pressed(&GBInput::A);
    let select_held = action.pressed(&GBInput::Select);
    let select_just_pressed = action.just_pressed(&GBInput::Select);
    let select_just_released = action.just_released(&GBInput::Select);

    // -- Chord detection (identical to SP fps_test.rs) --
    let chord_input = TurnChordInput {
        b_pressed: b_held,
        b_just_pressed: action.just_pressed(&GBInput::B),
        b_just_released: action.just_released(&GBInput::B),
        down_pressed: down_held,
        down_just_pressed: action.just_pressed(&GBInput::Down),
        down_just_released: action.just_released(&GBInput::Down),
        left_pressed: left_held,
        left_just_pressed: action.just_pressed(&GBInput::Left),
        left_just_released: action.just_released(&GBInput::Left),
        right_pressed: right_held,
        right_just_pressed: action.just_pressed(&GBInput::Right),
        right_just_released: action.just_released(&GBInput::Right),
        now_secs: time.elapsed_secs(),
    };

    let mut actions = PlayerActions::default();

    // Resolve chord → snap turn action.
    // Legacy keeps B+direction snap turns. AimCommitment uses B as immediate
    // aim, so `resolve_turn_chord` only mirrors B to AimMode there.
    let aim_commitment = matches!(
        combat_config.combat_control_mode,
        carcinisation_fps_core::CombatControlMode::AimCommitment
    );
    if let Some(kind) = resolve_turn_chord(&chord_input, &mut turn_chord, aim_commitment) {
        // Block snap turns while moving forward (matches SP).
        let blocked = up_held
            || (matches!(kind, TurnKind::SideTurnLeft | TurnKind::SideTurnRight) && down_held);
        if !blocked {
            set_snap_action(&mut actions, kind);
            // Animate locally for client-side turn suppression.
            request_snap_turn(&mut quick_turn, kind, &config);
        }
    }

    let in_aim_mode = aim_commitment && turn_chord.is_aim_mode();

    let select_action = if aim_commitment {
        resolve_select_action_turn(
            &SelectActionTurnInput {
                select_pressed: select_held,
                select_just_pressed,
                select_just_released,
                down_pressed: down_held,
                down_just_pressed: action.just_pressed(&GBInput::Down),
                left_pressed: left_held,
                left_just_pressed: action.just_pressed(&GBInput::Left),
                right_pressed: right_held,
                right_just_pressed: action.just_pressed(&GBInput::Right),
                now_secs: time.elapsed_secs(),
            },
            &mut select_action_turn,
            select_actions_allowed_outside_aim_mode(in_aim_mode),
        )
    } else {
        None
    };

    let action_turn = match select_action {
        Some(SelectActionOutcome::SnapTurn(kind)) => Some(kind),
        _ => None,
    };

    if let Some(kind) = action_turn {
        set_snap_action(&mut actions, kind);
        request_snap_turn(&mut quick_turn, kind, &config);
    }

    // -- Movement / Turn / Aim --
    let (movement, turn, aim_held, fire_held);

    if in_aim_mode {
        // AimCommitment: feet locked, body turns freely, Up/Down = visual pitch.
        movement = Vec2::ZERO;
        aim_held = true;

        // Continuous turn — body rotates while aiming.
        let mut t: f32 = 0.0;
        if !quick_turn.is_active() {
            if left_held {
                t += 1.0;
            }
            if right_held {
                t -= 1.0;
            }
        }
        turn = t;

        // Vertical pitch (visual-only).
        quick_turn.update_aim_pitch(
            up_held,
            down_held,
            combat_config.aim_pitch_speed,
            time.delta_secs(),
        );

        // Fire only while aiming.
        fire_held = a_held && !select_held;
    } else {
        // LEGACY(strafe) or not-yet-aiming: normal movement/turn.
        aim_held = false;
        quick_turn.reset_aim_pitch_offset();

        let mut mv = non_aim_movement_from_input(action_turn, up_held, down_held);
        // LEGACY(strafe): B + Left/Right = strafe. Only in Legacy mode.
        if b_held && !aim_commitment {
            if left_held {
                mv.x -= 1.0;
            }
            if right_held {
                mv.x += 1.0;
            }
        }
        if mv.length_squared() > 1.0 {
            mv = mv.normalize();
        }
        movement = mv;

        // Continuous turn (suppressed during snap animation and B-strafe).
        let mut t: f32 = 0.0;
        if !quick_turn.is_active() && !b_held {
            if left_held {
                t += 1.0;
            }
            if right_held {
                t -= 1.0;
            }
        }
        turn = t;

        // AimCommitment: cannot fire without aiming. Legacy: fire anytime.
        fire_held = if aim_commitment {
            false
        } else {
            a_held && !select_held
        };
    }

    // -- Weapon switch --
    // Legacy keeps Select-press switch. AimCommitment uses Select release so
    // Select+direction can become snap turn without also switching.
    if (!aim_commitment && select_just_pressed && !a_held)
        || matches!(select_action, Some(SelectActionOutcome::WeaponSwitch))
    {
        actions.set(PlayerActions::WEAPON_SWITCH);
    }

    // -- Melee chord (Select+A) --
    let melee = (select_held && a_just_pressed) || (select_just_pressed && a_held);
    if !aim_commitment && melee {
        actions.set(PlayerActions::MELEE);
    }

    // -- Build intent --
    let intent = ClientIntent {
        sequence: input_sequence.0,
        movement,
        turn,
        fire_held,
        actions,
        aim_held,
    };

    // -- Send policy --
    let is_idle = intent.is_idle();
    let timer_fired = send_timer.timer.just_finished();
    let changed = if is_idle {
        !send_timer.last_was_idle
    } else {
        true
    };

    // Send immediately on action edges or state change; periodic 30Hz for held state.
    let should_send = !actions.is_empty() || changed || (!is_idle && timer_fired);

    if !should_send {
        return;
    }

    if changed {
        send_timer.timer.reset();
    }

    send_timer.last_was_idle = is_idle;
    input_sequence.0.increment();

    let mut sent = intent;
    sent.sequence = input_sequence.0;

    // If simulated latency is active, buffer the packet instead of sending.
    if let Some(ref mut latency) = latency_opt {
        latency.push(sent);
    } else {
        commands.client_trigger(sent);
    }

    // -- Store predicted input for client-side prediction --
    // Written AFTER send + increment so the sequence matches the packet
    // the server will process (post-increment value).
    {
        use carcinisation_fps_core::movement::SnapTurnKind;

        let snap_turn_kind = if actions.has(PlayerActions::QUICK_TURN) {
            Some(SnapTurnKind::QuickTurn)
        } else if actions.has(PlayerActions::SNAP_TURN_LEFT) {
            Some(SnapTurnKind::Left)
        } else if actions.has(PlayerActions::SNAP_TURN_RIGHT) {
            Some(SnapTurnKind::Right)
        } else {
            None
        };

        pending_input.0.push((
            input_sequence.0,
            carcinisation_net::prediction::PredictedInput {
                movement,
                turn,
                snap_turn: snap_turn_kind,
                aim_held,
            },
        ));
    }
}

/// Flush delayed `ClientIntent` packets when simulated latency is active.
///
/// Runs every frame in `Update`. Does nothing if `SimulatedLatency` is absent.
pub fn flush_delayed_intents(
    mut commands: Commands,
    time: Res<Time>,
    mut latency: ResMut<SimulatedLatency>,
) {
    let dt = time.delta_secs();
    for intent in latency.drain_ready(dt) {
        commands.client_trigger(intent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aim_pitch_resets_on_reset_aim_pitch_offset() {
        let mut quick_turn = QuickTurnState::default();
        quick_turn.aim_pitch_offset = 8.0;

        quick_turn.reset_aim_pitch_offset();

        assert!(
            (quick_turn.aim_pitch_offset - 0.0).abs() < f32::EPSILON,
            "aim_pitch_offset should be reset to 0.0, got {}",
            quick_turn.aim_pitch_offset,
        );
    }

    #[test]
    fn aim_commitment_select_release_switches_weapon() {
        let mut state = SelectActionTurnState::default();
        let press = SelectActionTurnInput {
            select_pressed: true,
            select_just_pressed: true,
            now_secs: 0.0,
            ..Default::default()
        };
        assert_eq!(resolve_select_action_turn(&press, &mut state, true), None);

        let release = SelectActionTurnInput {
            select_just_released: true,
            now_secs: 0.05,
            ..Default::default()
        };
        assert_eq!(
            resolve_select_action_turn(&release, &mut state, true),
            Some(SelectActionOutcome::WeaponSwitch)
        );
    }

    #[test]
    fn aim_commitment_select_release_while_aiming_does_not_switch_weapon() {
        let mut state = SelectActionTurnState::default();
        let press = SelectActionTurnInput {
            select_pressed: true,
            select_just_pressed: true,
            now_secs: 0.0,
            ..Default::default()
        };
        assert_eq!(resolve_select_action_turn(&press, &mut state, false), None);

        let release = SelectActionTurnInput {
            select_just_released: true,
            now_secs: 0.05,
            ..Default::default()
        };
        assert_eq!(
            resolve_select_action_turn(&release, &mut state, false),
            None
        );
    }

    #[test]
    fn aim_commitment_select_direction_outside_aim_snaps_and_suppresses_movement() {
        for (label, down_held, left_held, right_held, kind, action_flag) in [
            (
                "down",
                true,
                false,
                false,
                TurnKind::QuickTurn,
                PlayerActions::QUICK_TURN,
            ),
            (
                "left",
                false,
                true,
                false,
                TurnKind::SideTurnLeft,
                PlayerActions::SNAP_TURN_LEFT,
            ),
            (
                "right",
                false,
                false,
                true,
                TurnKind::SideTurnRight,
                PlayerActions::SNAP_TURN_RIGHT,
            ),
        ] {
            let mut state = SelectActionTurnState::default();
            let press = SelectActionTurnInput {
                select_pressed: true,
                select_just_pressed: true,
                now_secs: 0.0,
                ..Default::default()
            };
            assert_eq!(resolve_select_action_turn(&press, &mut state, true), None);

            let action_turn = resolve_select_action_turn(
                &SelectActionTurnInput {
                    select_pressed: true,
                    down_pressed: down_held,
                    down_just_pressed: down_held,
                    left_pressed: left_held,
                    left_just_pressed: left_held,
                    right_pressed: right_held,
                    right_just_pressed: right_held,
                    now_secs: 0.05,
                    ..Default::default()
                },
                &mut state,
                true,
            );
            assert_eq!(
                action_turn,
                Some(SelectActionOutcome::SnapTurn(kind)),
                "{label} should request {kind:?}"
            );

            let mut actions = PlayerActions::default();
            if let Some(SelectActionOutcome::SnapTurn(kind)) = action_turn {
                set_snap_action(&mut actions, kind);
            }

            assert!(
                actions.has(action_flag),
                "{label} should set action flag {action_flag}"
            );
            assert!(
                !actions.has(PlayerActions::WEAPON_SWITCH),
                "{label} snap should not also switch weapon"
            );
        }

        assert_eq!(
            non_aim_movement_from_input(Some(TurnKind::QuickTurn), false, true),
            Vec2::ZERO
        );
    }

    #[test]
    fn aim_commitment_select_direction_while_aiming_does_not_snap_or_switch() {
        let mut state = SelectActionTurnState::default();
        let press = SelectActionTurnInput {
            select_pressed: true,
            select_just_pressed: true,
            now_secs: 0.0,
            ..Default::default()
        };
        assert_eq!(resolve_select_action_turn(&press, &mut state, false), None);

        let direction = SelectActionTurnInput {
            select_pressed: true,
            down_pressed: true,
            down_just_pressed: true,
            now_secs: 0.05,
            ..Default::default()
        };
        assert_eq!(
            resolve_select_action_turn(&direction, &mut state, false),
            None
        );

        let release = SelectActionTurnInput {
            select_just_released: true,
            now_secs: 0.06,
            ..Default::default()
        };
        assert_eq!(
            resolve_select_action_turn(&release, &mut state, false),
            None
        );
    }
}
