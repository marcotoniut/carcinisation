//! Server input handling: receive semantic intent → buffer → apply in `FixedUpdate`.

use crate::{ClientPlayerId, ServerMap};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::FpsMovementConfig;
use carcinisation_fps_core::movement;
use carcinisation_net::tick::{InputSequence, STALE_INPUT_TICKS};
use carcinisation_net::{ClientIntent, InputAck, NetPlayer, PlayerActions, PlayerId, TickCounter};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Tracks last processed input sequence per player (for dedup).
#[derive(Resource, Default)]
pub struct PlayerInputTracker {
    sequences: HashMap<PlayerId, u32>,
}

impl PlayerInputTracker {
    #[must_use]
    pub fn last_sequence(&self, player_id: &PlayerId) -> Option<u32> {
        self.sequences.get(player_id).copied()
    }

    pub fn remove_player(&mut self, player_id: &PlayerId) {
        self.sequences.remove(player_id);
    }
}

// STALE_INPUT_TICKS imported from carcinisation_net::tick (shared constant).

/// Per-player intent entry with staleness counter.
#[derive(Clone)]
struct IntentEntry {
    movement: Vec2,
    turn: f32,
    fire_held: bool,
    /// Pending one-shot actions. OR'd from network packets, consumed once per tick.
    pending_actions: PlayerActions,
    /// Ticks since last network update. Reset to 0 on each `set()`.
    age_ticks: u32,
}

/// Latest validated player intent, written by the network observer,
/// read by `FixedUpdate` systems.
///
/// Entries expire after `STALE_INPUT_TICKS` without a fresh packet.
#[derive(Resource, Default)]
pub struct PlayerIntentBuffer {
    entries: HashMap<PlayerId, IntentEntry>,
}

impl PlayerIntentBuffer {
    /// Store new intent from network. Resets age. Actions are OR'd so
    /// multiple packets in one tick accumulate edge triggers.
    pub fn set(&mut self, player_id: PlayerId, intent: &ClientIntent) {
        let entry = self.entries.entry(player_id).or_insert(IntentEntry {
            movement: Vec2::ZERO,
            turn: 0.0,
            fire_held: false,
            pending_actions: PlayerActions::default(),
            age_ticks: 0,
        });
        entry.movement = intent.movement;
        entry.turn = intent.turn;
        entry.fire_held = intent.fire_held;
        entry.pending_actions.merge(intent.actions);
        entry.age_ticks = 0;
    }

    /// Read continuous state and advance staleness counter.
    /// Returns `(movement, turn, fire_held)`. Zeroed if stale or missing.
    pub fn get_continuous_and_age(&mut self, pid: &PlayerId) -> (Vec2, f32, bool) {
        let Some(entry) = self.entries.get_mut(pid) else {
            return (Vec2::ZERO, 0.0, false);
        };
        if entry.age_ticks >= STALE_INPUT_TICKS {
            entry.movement = Vec2::ZERO;
            entry.turn = 0.0;
            entry.fire_held = false;
            return (Vec2::ZERO, 0.0, false);
        }
        entry.age_ticks += 1;
        (entry.movement, entry.turn, entry.fire_held)
    }

    /// Consume pending one-shot actions (clears them after reading).
    pub fn take_actions(&mut self, pid: &PlayerId) -> PlayerActions {
        let Some(entry) = self.entries.get_mut(pid) else {
            return PlayerActions::default();
        };
        let actions = entry.pending_actions;
        entry.pending_actions = PlayerActions::default();
        actions
    }

    /// Peek `fire_held` without aging (for combat system after movement aged).
    #[must_use]
    pub fn peek_fire_held(&self, pid: &PlayerId) -> bool {
        self.entries
            .get(pid)
            .filter(|e| e.age_ticks < STALE_INPUT_TICKS)
            .is_some_and(|e| e.fire_held)
    }

    pub fn remove_player(&mut self, pid: &PlayerId) {
        self.entries.remove(pid);
    }
}

// ---------------------------------------------------------------------------
// Observer: receive + validate + buffer
// ---------------------------------------------------------------------------

/// Receives `ClientIntent` from the network, validates the sequence,
/// and stores it in `PlayerIntentBuffer`.
pub(crate) fn receive_client_intent(
    trigger: On<FromClient<ClientIntent>>,
    clients: Query<&ClientPlayerId>,
    mut tracker: ResMut<PlayerInputTracker>,
    mut buffer: ResMut<PlayerIntentBuffer>,
) {
    let from_client = trigger.event();
    let Some(client_entity) = from_client.client_id.entity() else {
        warn!("Received ClientIntent with invalid client entity");
        return;
    };
    let intent = &from_client.message;

    let player_id = if let Ok(id) = clients.get(client_entity) {
        id.0
    } else {
        warn!(
            "Received intent from unknown client entity: {:?}",
            client_entity
        );
        return;
    };

    // Wrapping-aware sequence validation.
    let last_seq = tracker.sequences.entry(player_id).or_insert(0);
    let current_seq = intent.sequence.0;
    let diff = current_seq.wrapping_sub(*last_seq);
    if diff == 0 || diff > (1 << 31) {
        trace!(
            "Dropping stale intent: seq {} (last {})",
            current_seq, *last_seq
        );
        return;
    }
    *last_seq = current_seq;

    // Reject non-finite floats (NaN/Inf) from untrusted client input.
    let mut validated = intent.clone();
    if !validated.movement.x.is_finite() || !validated.movement.y.is_finite() {
        validated.movement = Vec2::ZERO;
    }
    if !validated.turn.is_finite() {
        validated.turn = 0.0;
    }

    // Clamp movement to unit length (prevent speed hacks).
    if validated.movement.length_squared() > 1.0001 {
        validated.movement = validated.movement.normalize();
    }
    validated.turn = validated.turn.clamp(-1.0, 1.0);

    buffer.set(player_id, &validated);
}

// ---------------------------------------------------------------------------
// FixedUpdate system: apply buffered intent
// ---------------------------------------------------------------------------

/// Server-only per-player snap turn animation state.
/// Delegates to `fps_core::snap_turn_params` / `tick_snap_turn` for shared math.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ServerQuickTurn {
    pub remaining_radians: f32,
    pub speed: f32,
    pub direction: f32,
}

impl ServerQuickTurn {
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.remaining_radians > 0.0
    }

    /// Start a snap turn. Delegates to `fps_core::snap_turn_params`.
    pub fn request(
        &mut self,
        kind: carcinisation_fps_core::SnapTurnKind,
        quick_turn_duration_secs: f32,
    ) {
        if self.remaining_radians > 0.0 {
            return;
        }
        let params = carcinisation_fps_core::snap_turn_params(kind, quick_turn_duration_secs);
        self.remaining_radians = params.remaining_radians;
        self.speed = params.speed;
        self.direction = params.direction;
    }

    /// Tick animation. Delegates to `fps_core::tick_snap_turn`.
    pub fn tick(&mut self, angle: &mut f32, dt: f32) {
        carcinisation_fps_core::tick_snap_turn(
            angle,
            &mut self.remaining_radians,
            self.speed,
            self.direction,
            dt,
        );
    }
}

use carcinisation_fps_core::SnapTurnKind;

/// Runs in `FixedUpdate` (`MovementSet`). Reads latest buffered intent
/// for each player and applies server-authoritative movement + turning.
pub fn apply_buffered_movement(
    mut buffer: ResMut<PlayerIntentBuffer>,
    mut players: Query<(&mut NetPlayer, &mut ServerQuickTurn)>,
    server_map: Res<ServerMap>,
    fixed_time: Res<Time<Fixed>>,
    movement_config: Res<FpsMovementConfig>,
) {
    let dt = fixed_time.delta_secs();

    for (mut player, mut snap_turn) in &mut players {
        if !matches!(player.state, carcinisation_net::PlayerNetState::Alive) {
            buffer.get_continuous_and_age(&player.player_id);
            buffer.take_actions(&player.player_id);
            continue;
        }

        let (movement_intent, turn_dir, _fire) = buffer.get_continuous_and_age(&player.player_id);
        let actions = buffer.take_actions(&player.player_id);

        // Process one-shot actions (edge-triggered).
        if actions.has(PlayerActions::WEAPON_SWITCH) {
            player.current_attack = match player.current_attack {
                carcinisation_net::NetAttackId::None => carcinisation_net::NetAttackId::Projectile,
                _ => carcinisation_net::NetAttackId::None,
            };
        }
        if actions.has(PlayerActions::QUICK_TURN) {
            snap_turn.request(
                SnapTurnKind::QuickTurn,
                movement_config.quick_turn_duration_secs,
            );
        }
        if actions.has(PlayerActions::SNAP_TURN_LEFT) {
            snap_turn.request(SnapTurnKind::Left, movement_config.quick_turn_duration_secs);
        }
        if actions.has(PlayerActions::SNAP_TURN_RIGHT) {
            snap_turn.request(
                SnapTurnKind::Right,
                movement_config.quick_turn_duration_secs,
            );
        }

        // Tick snap turn animation (same as SP tick_quick_turn).
        snap_turn.tick(&mut player.angle, dt);

        // Suppress continuous turn during snap turn animation (matches SP).
        if !snap_turn.is_active() && turn_dir != 0.0 {
            player.angle += turn_dir * movement_config.turn_speed * dt;
            player.angle = player.angle.rem_euclid(std::f32::consts::TAU);
        }

        // Apply movement (client already resolved strafe into movement.x).
        if movement_intent != Vec2::ZERO {
            let angle = player.angle;
            movement::apply_movement(
                &mut player.position,
                angle,
                movement_intent,
                movement_config.move_speed,
                dt,
                &server_map.0,
                movement_config.collision_margin,
            );
        }
    }
}

/// Sends an `InputAck` to all clients for each player that needs correction.
///
/// Runs in `FixedUpdate` (`MovementSet`) after `apply_buffered_movement`.
/// Each ack carries the player's authoritative position/angle and the last
/// sequence the server processed, enabling client-side prediction reconciliation.
///
/// # Ack send conditions
///
/// An ack is sent when ANY of these hold:
///
/// 1. **Sequence advanced** — the server processed a new input.
/// 2. **Snap turn active** — the client needs continuous correction during
///    the snap animation. Without this, the ack dedup would suppress acks
///    for ~10 of a 12-tick quick turn, causing the client's snap state to
///    freeze while the server's completes.
/// 3. **Snap just completed** — the final angle must reach the client so
///    it lands on the exact server angle.
///
/// When none of these hold (idle player, no snap), no ack is sent.
#[allow(clippy::implicit_hasher)]
pub fn send_input_acks(
    mut commands: Commands,
    players: Query<(&NetPlayer, &ServerQuickTurn)>,
    tracker: Res<PlayerInputTracker>,
    tick_counter: Res<TickCounter>,
    mut last_acked: Local<HashMap<PlayerId, u32>>,
    mut had_snap: Local<HashMap<PlayerId, bool>>,
) {
    for (player, snap_turn) in &players {
        let Some(seq) = tracker.last_sequence(&player.player_id) else {
            continue;
        };

        let snap_active = snap_turn.is_active();
        let was_snapping = had_snap.get(&player.player_id).copied().unwrap_or(false);
        had_snap.insert(player.player_id, snap_active);

        // Send an ack when:
        // - Input sequence advanced (normal case), OR
        // - A snap turn is active (client needs continuous correction), OR
        // - A snap turn just completed (client needs the final angle).
        let seq_changed = last_acked.get(&player.player_id) != Some(&seq);
        let snap_needs_ack = snap_active || was_snapping;

        if !seq_changed && !snap_needs_ack {
            continue;
        }

        last_acked.insert(player.player_id, seq);
        commands.server_trigger(ToClients {
            mode: SendMode::Broadcast,
            message: InputAck {
                player_id: player.player_id,
                last_processed_sequence: InputSequence(seq),
                server_tick: tick_counter.0,
                position: player.position,
                angle: player.angle,
                snap_remaining_radians: snap_turn.remaining_radians,
                snap_speed: snap_turn.speed,
                snap_direction: snap_turn.direction,
            },
        });
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn make_intent(movement: Vec2, turn: f32, fire: bool, actions: PlayerActions) -> ClientIntent {
        ClientIntent {
            sequence: InputSequence(1),
            movement,
            turn,
            fire_held: fire,
            actions,
        }
    }

    #[test]
    fn buffer_stores_and_returns_continuous_state() {
        let mut buf = PlayerIntentBuffer::default();
        let pid = PlayerId(1);
        let intent = make_intent(Vec2::new(0.0, 1.0), 0.5, true, PlayerActions::default());
        buf.set(pid, &intent);
        let (mv, turn, fire) = buf.get_continuous_and_age(&pid);
        assert_eq!(mv, Vec2::new(0.0, 1.0));
        assert!((turn - 0.5).abs() < 1e-6);
        assert!(fire);
    }

    #[test]
    fn buffer_expires_after_stale_ticks() {
        let mut buf = PlayerIntentBuffer::default();
        let pid = PlayerId(1);
        buf.set(
            pid,
            &make_intent(Vec2::Y, 1.0, true, PlayerActions::default()),
        );
        for _ in 0..STALE_INPUT_TICKS {
            let (mv, _, _) = buf.get_continuous_and_age(&pid);
            assert_ne!(mv, Vec2::ZERO);
        }
        let (mv, turn, fire) = buf.get_continuous_and_age(&pid);
        assert_eq!(mv, Vec2::ZERO);
        assert_eq!(turn, 0.0);
        assert!(!fire);
    }

    #[test]
    fn buffer_actions_accumulate_and_consume() {
        let mut buf = PlayerIntentBuffer::default();
        let pid = PlayerId(1);
        // Two intents with different actions.
        buf.set(
            pid,
            &make_intent(
                Vec2::ZERO,
                0.0,
                false,
                PlayerActions::from_raw(PlayerActions::WEAPON_SWITCH),
            ),
        );
        buf.set(
            pid,
            &make_intent(
                Vec2::ZERO,
                0.0,
                false,
                PlayerActions::from_raw(PlayerActions::SNAP_TURN_LEFT),
            ),
        );
        let actions = buf.take_actions(&pid);
        assert!(actions.has(PlayerActions::WEAPON_SWITCH));
        assert!(actions.has(PlayerActions::SNAP_TURN_LEFT));
        // Consumed — second call returns empty.
        let actions2 = buf.take_actions(&pid);
        assert!(actions2.is_empty());
    }

    #[test]
    fn peek_fire_held_without_aging() {
        let mut buf = PlayerIntentBuffer::default();
        let pid = PlayerId(1);
        buf.set(
            pid,
            &make_intent(Vec2::ZERO, 0.0, true, PlayerActions::default()),
        );
        // Peek many times — fire_held should persist.
        for _ in 0..STALE_INPUT_TICKS + 5 {
            assert!(buf.peek_fire_held(&pid));
        }
        // get_continuous_and_age should still see fire.
        let (_, _, fire) = buf.get_continuous_and_age(&pid);
        assert!(fire);
    }

    #[test]
    fn peek_fire_held_false_after_stale() {
        let mut buf = PlayerIntentBuffer::default();
        let pid = PlayerId(1);
        buf.set(
            pid,
            &make_intent(Vec2::ZERO, 0.0, true, PlayerActions::default()),
        );
        for _ in 0..=STALE_INPUT_TICKS {
            buf.get_continuous_and_age(&pid);
        }
        assert!(!buf.peek_fire_held(&pid));
    }

    #[test]
    fn movement_clamped_to_unit_length() {
        // Simulate what receive_client_intent does.
        let mut intent = ClientIntent {
            sequence: InputSequence(1),
            movement: Vec2::new(5.0, 5.0),
            turn: 3.0,
            fire_held: false,
            actions: PlayerActions::default(),
        };
        if intent.movement.length_squared() > 1.0001 {
            intent.movement = intent.movement.normalize();
        }
        intent.turn = intent.turn.clamp(-1.0, 1.0);
        assert!((intent.movement.length() - 1.0).abs() < 0.01);
        assert!((intent.turn - 1.0).abs() < 1e-6);
    }
}
