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
    QuickTurnState, TurnChordInput, TurnChordState, TurnKind, request_snap_turn, resolve_turn_chord,
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
    mut quick_turn: ResMut<QuickTurnState>,
    config: Res<carcinisation_fps::plugin::Config>,
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
    if let Some(kind) = resolve_turn_chord(&chord_input, &mut turn_chord) {
        // Block snap turns while moving forward (matches SP).
        let blocked = up_held
            || (matches!(kind, TurnKind::SideTurnLeft | TurnKind::SideTurnRight) && down_held);
        if !blocked {
            match kind {
                TurnKind::QuickTurn => actions.set(PlayerActions::QUICK_TURN),
                TurnKind::SideTurnLeft => actions.set(PlayerActions::SNAP_TURN_LEFT),
                TurnKind::SideTurnRight => actions.set(PlayerActions::SNAP_TURN_RIGHT),
            }
            // Animate locally for client-side turn suppression.
            request_snap_turn(&mut quick_turn, kind, &config);
        }
    }

    // -- Movement --
    // Backward is always applied (matches SP). B+Down arms a quick turn chord
    // but does NOT suppress backward movement — the turn fires on release.
    let mut movement = Vec2::ZERO;
    if up_held {
        movement.y += 1.0;
    }
    if down_held {
        movement.y -= 1.0;
    }
    // Strafe: B + Left/Right held.
    if b_held {
        if left_held {
            movement.x -= 1.0;
        }
        if right_held {
            movement.x += 1.0;
        }
    }
    if movement.length_squared() > 1.0 {
        movement = movement.normalize();
    }

    // -- Continuous turn (suppressed during snap turn animation) --
    let mut turn: f32 = 0.0;
    if !quick_turn.is_active() && !b_held {
        if left_held {
            turn += 1.0;
        }
        if right_held {
            turn -= 1.0;
        }
    }

    // -- Fire --
    // Network intent reports raw player input. Cooldown/ammo/eligibility
    // is decided by the simulation (server), not by client presentation state.
    let fire_held = a_held && !select_held;

    // -- Weapon switch (Select alone, not with A) --
    if select_just_pressed && !a_held {
        actions.set(PlayerActions::WEAPON_SWITCH);
    }

    // -- Melee chord (Select+A) --
    let melee = (select_held && a_just_pressed) || (select_just_pressed && a_held);
    if melee {
        actions.set(PlayerActions::MELEE);
    }

    // -- Build intent --
    let intent = ClientIntent {
        sequence: input_sequence.0,
        movement,
        turn,
        fire_held,
        actions,
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
