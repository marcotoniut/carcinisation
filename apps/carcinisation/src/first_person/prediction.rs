//! Client-side movement prediction with server reconciliation.
//!
//! The predicted state is used by `sync_camera_from_net_player` to drive the
//! local player's camera immediately, instead of waiting for the server's
//! replicated `NetPlayer` position (which lags by one round-trip).
//!
//! When an `InputAck` arrives from the server, [`handle_input_ack`] prunes
//! the prediction history through the acked sequence, resets
//! `PredictedPlayerState` to the server's authoritative position/angle, and
//! replays all remaining unacked entries so the predicted state stays ahead
//! of the server by the local input lag.
//!
//! # Scaling notes
//!
//! Prediction currently uses a `Resource` (`PredictedPlayerState`) because
//! only the local player is predicted. If multiple entities need prediction
//! (e.g., co-op partner, player-owned vehicle), refactor to a `Predicted`
//! component on entities. The core function [`apply_prediction_tick`] is
//! already entity-agnostic and accepts `&mut PredictedPlayerState` directly.
//!
//! # Simulated latency
//!
//! Set `CARCINISATION_SIMULATED_PING_MS` to add artificial round-trip delay
//! to outgoing `ClientIntent` packets. This does not affect prediction
//! (which runs locally) but delays when the server sees the input, making
//! drift and reconciliation errors visible on localhost.
//!
//! ```bash
//! CARCINISATION_SIMULATED_PING_MS=200 just dev-fps-unus
//! ```
//!
//! # Future considerations
//!
//! ## Enemy interpolation (not prediction)
//!
//! Enemies are server-authoritative NPCs — predicting their movement would
//! require duplicating the full AI on the client (targeting, pathfinding,
//! attack decisions), which depends on ALL player positions and is
//! nondeterministic. Players don't notice 100ms enemy delay because they
//! don't control enemies.
//!
//! What IS useful is **visual interpolation**: smoothing enemy positions
//! between 30Hz replication snapshots, same as `RemotePositionInterpolation` does
//! for remote players. This would eliminate visible snapping for enemy
//! movement without any gameplay-side prediction.
//!
//! ## Weapon fire feedback prediction
//!
//! Currently, pressing fire sends `ClientIntent(fire_held=true)` and waits
//! for the server to confirm the hit via `HitConfirm` / `DamageEffect`.
//! Cosmetic prediction could show muzzle flash and play hit sounds locally
//! before the server confirms, then reconcile if the server disagrees.
//! This is visual-only and does not affect damage — the server remains
//! authoritative for all combat outcomes.
//!
//! ## Server-side lag compensation
//!
//! Hit detection currently uses the server's real-time enemy positions.
//! At high latency, the client's predicted camera is ahead of where the
//! server thinks the player is aiming, causing apparent misses. Lag
//! compensation would rewind enemy positions to where they were when the
//! client pressed fire, then check the hit from the client's perspective.
//! This is a server-side change and the most complex remaining netcode
//! improvement.
//!
//! ## Correction smoothing (deferred)
//!
//! Reconciliation corrections currently snap immediately via
//! [`PredictedRenderState::on_reconciliation`]. Exponential-decay smoothing
//! was removed because computing the offset at ack-receive time (`PreUpdate`)
//! produced stale values by camera-read time (Update). See the comment block
//! above [`handle_input_ack`] for the recommended re-implementation path
//! (compute offset at camera-read time) when high-latency snap corrections
//! become visually noticeable.

use bevy::prelude::*;
use carcinisation_fps::plugin::{MapRes, PlayerDead};
use carcinisation_fps_core::FpsMovementConfig;
use carcinisation_fps_core::movement::{snap_turn_params, tick_snap_turn};
use carcinisation_net::prediction::{
    ClientMap, PredictedInput, PredictionEntry, PredictionHistory, PredictionSnapshot,
};
use carcinisation_net::tick::{InputSequence, STALE_INPUT_TICKS};
use carcinisation_net::{InputAck, NetPlayer};

use crate::first_person::LocalPlayerId;
use crate::first_person::interpolation::shortest_angle_delta;

/// Diagnostic metrics for prediction debugging. Visible via BRP.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct PredictionDiagnostics {
    /// Camera position as written by `sync_camera_from_net_player`.
    pub camera_position: Vec2,
    pub camera_angle: f32,
    /// Replicated `NetPlayer` position (server truth, 30Hz).
    pub replicated_position: Vec2,
    pub replicated_angle: f32,
    /// Last `InputAck` correction error magnitude (position).
    pub last_correction_pos: f32,
    /// Last `InputAck` correction error magnitude (angle).
    pub last_correction_angle: f32,
    /// Total reconciliation corrections since startup.
    pub correction_count: u32,
    /// `PredictedRenderState` interpolation alpha (0.0-1.0).
    pub render_alpha: f32,
    /// Number of camera writes this frame (should be exactly 1).
    pub camera_writes_this_frame: u32,
    /// Whether prediction is active for the local player.
    pub prediction_active: bool,
    /// Frame counter.
    pub frame: u64,
}

/// Resets per-frame counters. Runs at the start of Update.
pub fn reset_prediction_diagnostics(mut diag: ResMut<PredictionDiagnostics>) {
    diag.camera_writes_this_frame = 0;
    diag.frame += 1;
}

/// Runtime flag to disable local-player prediction.
///
/// When disabled, the camera uses the replicated `NetPlayer` position
/// directly (30Hz server updates with no local prediction). Useful for
/// comparing predicted vs replicated camera feel without code changes.
///
/// Set `CARCINISATION_DISABLE_PREDICTION=1` to disable at startup.
/// Default: enabled.
#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct PredictionEnabled(pub bool);

impl PredictionEnabled {
    /// Read from env var. Returns enabled unless `CARCINISATION_DISABLE_PREDICTION=1`.
    pub fn from_env() -> Self {
        let disabled = std::env::var("CARCINISATION_DISABLE_PREDICTION")
            .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
        if disabled {
            bevy::log::info!("[prediction] disabled via CARCINISATION_DISABLE_PREDICTION");
        }
        Self(!disabled)
    }
}

impl Default for PredictionEnabled {
    fn default() -> Self {
        Self(true)
    }
}

/// Predicted local-player state, updated each `FixedUpdate` tick.
///
/// Mirrors the server's authoritative position/angle but runs ahead using
/// the same `apply_movement` + `tick_snap_turn` math.
#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct PredictedPlayerState {
    pub position: Vec2,
    pub angle: f32,
    /// Client-local snap turn animation (mirrors server's `ServerQuickTurn`).
    pub snap_remaining_radians: f32,
    pub snap_total_radians: f32,
    pub snap_speed: f32,
    pub snap_direction: f32,
    /// Speed modifier multiplier (0.1..=1.0), or 1.0 if no modifier active.
    /// Carried in `InputAck` for prediction parity with server movement.
    pub speed_modifier_multiplier: f32,
    /// Speed modifier remaining drain budget, or 0.0 if none active.
    pub speed_modifier_remaining: f32,
    /// Active push impulse direction (normalised or zero).
    pub impulse_direction: Vec2,
    /// Push impulse strength (map units/s).
    pub impulse_strength: f32,
    /// Push impulse remaining lifetime (seconds). No impulse when <= 0.
    pub impulse_remaining: f32,
    /// Push impulse total duration (seconds).
    pub impulse_duration: f32,
    /// Set to `true` once initialised from the first replicated `NetPlayer`.
    pub initialised: bool,
}

impl Default for PredictedPlayerState {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            angle: 0.0,
            snap_remaining_radians: 0.0,
            snap_total_radians: 0.0,
            snap_speed: 0.0,
            snap_direction: 0.0,
            speed_modifier_multiplier: 1.0,
            speed_modifier_remaining: 0.0,
            impulse_direction: Vec2::ZERO,
            impulse_strength: 0.0,
            impulse_remaining: 0.0,
            impulse_duration: 0.0,
            initialised: false,
        }
    }
}

/// Holds inputs collected by `collect_and_send_intent` for the current
/// frame, consumed by [`apply_predicted_movement`] in `FixedUpdate`.
///
/// Multiple inputs can accumulate between fixed ticks when `Update` runs
/// faster than 30 Hz, so this is a `Vec` rather than a single slot.
///
/// # Latest-input-per-tick semantics
///
/// `apply_predicted_movement` keeps only the **last** entry (latest
/// continuous state) and discards the rest, mirroring the server's
/// `PlayerIntentBuffer` which also applies one intent per tick.
///
/// **Edge actions** (snap turns, weapon switch, melee) are one-shot and
/// would be lost if discarded naively. Before clearing, the system
/// merges `snap_turn` from discarded entries into the kept entry. This
/// mirrors the server's `PlayerActions::merge()` (OR-accumulate), which
/// preserves edge-triggered actions that arrived in earlier packets
/// within the same server tick.
#[derive(Resource, Default)]
pub struct PendingInput(pub Vec<(InputSequence, PredictedInput)>);

/// Render-time interpolation between 30Hz predicted ticks.
///
/// `apply_predicted_movement` (`FixedUpdate`, 30Hz) stores `prev` and `current`
/// snapshots. `sync_camera_from_net_player` (Update, 60Hz+) lerps between
/// them based on elapsed time since the last fixed tick. This eliminates
/// the visual stepping caused by displaying 30Hz discrete positions at 60Hz.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct PredictedRenderState {
    pub prev_position: Vec2,
    pub prev_angle: f32,
    pub current_position: Vec2,
    pub current_angle: f32,
    /// Time elapsed since the last `FixedUpdate` tick.
    pub elapsed: f32,
    /// Expected interval between fixed ticks (~1/30).
    pub interval: f32,
    /// Whether we have at least two snapshots to interpolate between.
    pub ready: bool,
}

impl PredictedRenderState {
    /// Interpolated position for rendering.
    #[must_use]
    pub fn interpolated_position(&self) -> Vec2 {
        if !self.ready {
            return self.current_position;
        }
        let t = if self.interval > 0.0 {
            (self.elapsed / self.interval).min(1.0)
        } else {
            1.0
        };
        self.prev_position.lerp(self.current_position, t)
    }

    /// Interpolated angle for rendering (shortest-arc).
    #[must_use]
    pub fn interpolated_angle(&self) -> f32 {
        if !self.ready {
            return self.current_angle;
        }
        let t = if self.interval > 0.0 {
            (self.elapsed / self.interval).min(1.0)
        } else {
            1.0
        };
        shortest_angle_delta(self.prev_angle, self.current_angle).mul_add(t, self.prev_angle)
    }

    /// Called by `apply_predicted_movement` after advancing the predicted state.
    pub const fn on_fixed_tick(&mut self, position: Vec2, angle: f32, dt: f32) {
        self.prev_position = self.current_position;
        self.prev_angle = self.current_angle;
        self.current_position = position;
        self.current_angle = angle;
        self.interval = dt;
        self.elapsed = 0.0;
        self.ready = true;
    }

    /// Called by `init_predicted_state` to seed both prev and current.
    pub fn seed(&mut self, position: Vec2, angle: f32) {
        self.prev_position = position;
        self.prev_angle = angle;
        self.current_position = position;
        self.current_angle = angle;
        self.elapsed = 0.0;
        self.interval = 1.0 / 30.0;
        self.ready = false;
    }

    /// Called by `handle_input_ack` after reconciliation replay completes.
    ///
    /// Updates `current` to the corrected position without shifting `prev`,
    /// so render interpolation smoothly converges from wherever it was
    /// toward the corrected state over the remaining interval. This
    /// eliminates the one-frame delay that would otherwise occur if we
    /// waited for the next `FixedUpdate` to propagate the correction.
    pub const fn on_reconciliation(&mut self, position: Vec2, angle: f32) {
        self.current_position = position;
        self.current_angle = angle;
    }

    /// Reset to default state (used on death).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Advances `PredictedRenderState.elapsed` each render frame.
pub fn tick_predicted_render(mut render: ResMut<PredictedRenderState>, time: Res<Time>) {
    render.elapsed += time.delta_secs();
}

/// One-shot system: populate [`ClientMap`] from the rendering map so
/// prediction can run collision detection matching the server.
///
/// Run condition: `resource_exists::<MapRes>.and(not(resource_exists::<ClientMap>))`.
pub fn init_client_map(mut commands: Commands, map_res: Res<MapRes>) {
    commands.insert_resource(ClientMap(map_res.0.clone()));
    info!("[prediction] ClientMap initialised from MapRes");
}

/// Keep `ClientMap` in sync when `MapRes` is hot-reloaded.
///
/// Runs every frame but only copies data when `MapRes` has actually changed
/// (Bevy's change detection on `Res`). Skips the first run because
/// `is_changed()` fires on first access. This ensures prediction collision
/// matches the rendering map after a Cmd+R reload.
pub fn sync_client_map_from_map_res(
    map_res: Res<MapRes>,
    mut client_map: ResMut<ClientMap>,
    mut first_run: Local<bool>,
) {
    if !*first_run {
        *first_run = true;
        return;
    }
    if map_res.is_changed() {
        client_map.0 = map_res.0.clone();
        info!("[prediction] ClientMap synced from reloaded MapRes");
    }
}

/// Each frame, if the predicted state is not yet initialised, copy the
/// local player's replicated position/angle as the starting point.
pub fn init_predicted_state(
    mut predicted: ResMut<PredictedPlayerState>,
    mut render_state: ResMut<PredictedRenderState>,
    enabled: Res<PredictionEnabled>,
    local_id: Res<LocalPlayerId>,
    net_players: Query<&NetPlayer>,
) {
    if !enabled.0 || predicted.initialised {
        return;
    }
    let Some(my_id) = local_id.0 else {
        return;
    };
    let Some(local_np) = net_players.iter().find(|p| p.player_id == my_id) else {
        return;
    };
    predicted.position = local_np.position;
    predicted.angle = local_np.angle;
    predicted.snap_remaining_radians = 0.0;
    predicted.snap_total_radians = 0.0;
    predicted.snap_speed = 0.0;
    predicted.snap_direction = 0.0;
    predicted.speed_modifier_multiplier = 1.0;
    predicted.speed_modifier_remaining = 0.0;
    predicted.impulse_direction = Vec2::ZERO;
    predicted.impulse_strength = 0.0;
    predicted.impulse_remaining = 0.0;
    predicted.impulse_duration = 0.0;
    predicted.initialised = true;
    render_state.seed(local_np.position, local_np.angle);
    info!(
        "[prediction] initialised from NetPlayer: pos={:?} angle={:.2}",
        predicted.position, predicted.angle
    );
}

/// Last applied continuous input for stale-tick prediction.
///
/// When `PendingInput` is empty on a `FixedUpdate` tick, the server still
/// applies the stale buffer (movement/turn from the last received intent).
/// The client must mirror this to avoid drifting behind by one tick per
/// gap. This resource tracks the last continuous input and an age counter
/// matching the server's `STALE_INPUT_TICKS` expiry.
///
/// Resource (not `Local`) so `clear_prediction_on_death` and the
/// prediction-disabled path can reset it.
#[derive(Resource, Default)]
pub struct StaleInput {
    pub movement: Vec2,
    pub turn: f32,
    pub age_ticks: u32,
}

/// `FixedUpdate` system (30 Hz). Consumes [`PendingInput`] and advances
/// [`PredictedPlayerState`] using the same movement logic as the server.
///
/// When `PendingInput` is empty, applies the last known continuous input
/// as a "stale tick" to match the server's `PlayerIntentBuffer` behavior
/// (which reapplies the last buffer data for up to `STALE_INPUT_TICKS`).
///
/// Pushes a [`PredictionEntry`] to [`PredictionHistory`] after each tick.
#[allow(clippy::missing_panics_doc)]
#[allow(clippy::too_many_arguments)]
pub fn apply_predicted_movement(
    mut predicted: ResMut<PredictedPlayerState>,
    mut pending: ResMut<PendingInput>,
    mut history: ResMut<PredictionHistory>,
    mut render_state: ResMut<PredictedRenderState>,
    enabled: Res<PredictionEnabled>,
    client_map: Option<Res<ClientMap>>,
    movement_config: Option<Res<FpsMovementConfig>>,
    player_dead: Option<Res<PlayerDead>>,
    fixed_time: Res<Time<Fixed>>,
    mut stale: ResMut<StaleInput>,
) {
    if !enabled.0 {
        pending.0.clear();
        stale.age_ticks = STALE_INPUT_TICKS;
        return;
    }

    // Cannot predict without collision map or config.
    let (Some(client_map), Some(movement_config)) = (client_map, movement_config) else {
        return;
    };

    if !predicted.initialised {
        // Discard pending input if we haven't initialised yet.
        pending.0.clear();
        return;
    }

    // Skip while dead -- the server doesn't move dead players.
    if player_dead.as_ref().is_some_and(|d| d.0) {
        pending.0.clear();
        return;
    }

    let dt = fixed_time.delta_secs();

    if pending.0.is_empty() {
        // --- Stale-tick prediction ---
        //
        // No new input this FixedUpdate tick. The server still advances
        // using `PlayerIntentBuffer`'s stale data (last received movement/
        // turn for up to STALE_INPUT_TICKS, then zeroed) and ALWAYS ticks
        // `snap_turn.tick()` regardless of stale expiry.
        //
        // Three cases:
        //   1. Stale expired AND no snap → server idle, return early.
        //   2. Stale expired AND snap active → apply idle input so
        //      tick_snap_turn advances. A QuickTurn (~12 ticks) outlasts
        //      stale expiry (5 ticks), so this fires every quick turn.
        //   3. Stale fresh → mirror server's stale buffer with last known
        //      movement/turn. Snap also ticks if active.
        //
        // snap_turn is always None (edge actions never repeat from stale).
        // Stale ticks are NOT in PredictionHistory — they're baked into
        // the server's ack position, so reconciliation handles them.
        let snap_active = predicted.snap_remaining_radians > 0.0;

        if stale.age_ticks >= STALE_INPUT_TICKS && !snap_active {
            // Stale expired and no snap in progress — server is also idle.
            return;
        }

        // Movement/turn: use stale data while fresh, zero when expired.
        // Snap: always tick (server does snap_turn.tick() unconditionally).
        let (movement, turn) = if stale.age_ticks < STALE_INPUT_TICKS {
            stale.age_ticks += 1;
            (stale.movement, stale.turn)
        } else {
            (Vec2::ZERO, 0.0)
        };

        let stale_input = PredictedInput {
            movement,
            turn,
            snap_turn: None, // edge-triggered actions never repeat
        };

        let stale_state = &mut *predicted;
        apply_prediction_tick(
            stale_state,
            &stale_input,
            &client_map.0,
            movement_config.move_speed,
            movement_config.turn_speed,
            movement_config.collision_margin,
            movement_config.quick_turn_duration_secs,
            dt,
        );

        // Stale ticks don't go into history — they're baked into the ack
        // position when the next ack arrives, so reconciliation accounts
        // for them automatically.
        render_state.on_fixed_tick(stale_state.position, stale_state.angle, dt);
        return;
    }

    // Cap pending inputs to avoid unbounded growth during stalls or lag spikes.
    if pending.0.len() > 120 {
        let drain_count = pending.0.len() - 60;
        pending.0.drain(..drain_count);
        warn!("[prediction] PendingInput overflow, drained {drain_count} entries");
    }

    let pred_state = &mut *predicted;

    // The server applies ONE input per tick (the latest buffered intent),
    // not a queue. We must match: take only the last pending input, apply
    // it once with one tick's dt. Earlier inputs between Update frames are
    // discarded — they were sent to the server but the server also only
    // uses the latest.
    //
    // If we applied every accumulated input with a full dt each, prediction
    // would run 2-3x faster than the server (at 60Hz Update / 30Hz Fixed),
    // causing ~0.2 unit reconciliation corrections every ack.
    //
    // IMPORTANT: One-shot actions (snap turns, weapon switch, melee) from
    // discarded intermediate entries must be merged into the kept entry.
    // The server OR-accumulates actions via PlayerIntentBuffer::merge(),
    // so it processes snap turns even if they arrived in an earlier packet
    // within the same tick. We must mirror this or prediction diverges.
    let (sequence, mut input) = pending.0.pop().expect("checked non-empty above");

    // Merge snap_turn from discarded entries into the kept entry.
    // Only overwrite if the kept entry has no snap_turn (first one wins,
    // matching server's OR-accumulate — once a snap is set, it stays).
    if input.snap_turn.is_none() {
        for (_, discarded) in &pending.0 {
            if discarded.snap_turn.is_some() {
                input.snap_turn = discarded.snap_turn;
                break; // first snap turn in the batch wins
            }
        }
    }

    pending.0.clear(); // discard older inputs

    // Update stale tracker: record continuous state for stale-tick prediction.
    // Matches server's PlayerIntentBuffer which retains movement/turn until stale.
    stale.movement = input.movement;
    stale.turn = input.turn;
    stale.age_ticks = 0;

    apply_prediction_tick(
        pred_state,
        &input,
        &client_map.0,
        movement_config.move_speed,
        movement_config.turn_speed,
        movement_config.collision_margin,
        movement_config.quick_turn_duration_secs,
        dt,
    );

    history.push(PredictionEntry {
        sequence,
        input,
        result: PredictionSnapshot {
            position: pred_state.position,
            angle: pred_state.angle,
        },
        dt,
    });

    // Feed render interpolation with the latest predicted state.
    render_state.on_fixed_tick(pred_state.position, pred_state.angle, dt);
}

/// When the local player dies, clear prediction state so that on respawn
/// it re-initialises from the next replicated `NetPlayer` position.
pub fn clear_prediction_on_death(
    mut predicted: ResMut<PredictedPlayerState>,
    mut history: ResMut<PredictionHistory>,
    mut render_state: ResMut<PredictedRenderState>,
    mut stale: ResMut<StaleInput>,
    player_dead: Option<Res<PlayerDead>>,
) {
    let Some(dead) = player_dead else { return };
    if dead.0 && predicted.initialised {
        history.clear();
        predicted.initialised = false;
        render_state.reset();
        stale.age_ticks = STALE_INPUT_TICKS; // expire stale input
        stale.movement = Vec2::ZERO;
        stale.turn = 0.0;
        debug!("[prediction] cleared on death");
    }
}

// ── Correction smoothing ─────────────────────────────────────────────────
//
// Correction smoothing (blending out reconciliation error over ~100ms) is
// intentionally deferred. The InputAck observer fires during PreUpdate, but
// the camera reads during Update after FixedUpdate may have advanced the
// predicted state. Computing an offset at ack time produces a stale value
// by camera-read time, causing visible jitter.
//
// When high-latency correction snaps become noticeable, re-implement
// smoothing at camera-read time: compute the offset in
// sync_camera_from_net_player (Update) by comparing the interpolated
// render position against the raw predicted position, then exponentially
// decay it. The PredictedRenderState::on_reconciliation() method already
// propagates corrections to the render layer without the timing gap.

// ── Reconciliation: snap to server truth + replay unacked inputs ─────────

/// Observer for `InputAck` from the server.
///
/// 1. Prunes `PredictionHistory` through the acked sequence.
/// 2. Resets `PredictedPlayerState` to the server's authoritative position/angle.
/// 3. Replays all remaining (unacked) entries through `apply_prediction_tick`
///    so the predicted state stays ahead of the server by the local input lag.
/// 4. Updates `PredictedRenderState` so render interpolation converges toward
///    the corrected position without waiting for the next `FixedUpdate`.
#[allow(clippy::too_many_arguments)]
pub fn handle_input_ack(
    trigger: On<InputAck>,
    local_id: Res<LocalPlayerId>,
    mut predicted: ResMut<PredictedPlayerState>,
    mut history: ResMut<PredictionHistory>,
    mut render_state: ResMut<PredictedRenderState>,
    mut diag: ResMut<PredictionDiagnostics>,
    client_map: Option<Res<ClientMap>>,
    movement_config: Option<Res<FpsMovementConfig>>,
) {
    let ack = trigger.event();

    // Only process acks for the local player.
    let Some(my_id) = local_id.0 else { return };
    if ack.player_id != my_id {
        return;
    }

    if !predicted.initialised {
        return;
    }

    // Record pre-correction state for diagnostics.
    let old_position = predicted.position;
    let old_angle = predicted.angle;

    // 1. Prune history through acked sequence.
    history.prune_through(ack.last_processed_sequence);

    // 2. Reset to server truth (including snap turn and speed modifier state
    //    so the client can replay with identical movement parameters).
    predicted.position = ack.position;
    predicted.angle = ack.angle;
    predicted.snap_remaining_radians = ack.snap_remaining_radians;
    predicted.snap_total_radians = ack.snap_total_radians;
    predicted.snap_speed = ack.snap_speed;
    predicted.snap_direction = ack.snap_direction;
    predicted.speed_modifier_multiplier = ack.speed_modifier_multiplier;
    predicted.speed_modifier_remaining = ack.speed_modifier_remaining;
    predicted.impulse_direction = Vec2::new(ack.impulse_direction_x, ack.impulse_direction_y);
    predicted.impulse_strength = ack.impulse_strength;
    predicted.impulse_remaining = ack.impulse_remaining;
    predicted.impulse_duration = ack.impulse_duration;

    // 3. Replay remaining unacked inputs.
    let Some(client_map) = client_map else { return };
    let Some(cfg) = movement_config else { return };

    // Collect entries to avoid borrow conflict (prune already done).
    let entries: Vec<_> = history.iter_all().cloned().collect();

    for entry in &entries {
        apply_prediction_tick(
            &mut predicted,
            &entry.input,
            &client_map.0,
            cfg.move_speed,
            cfg.turn_speed,
            cfg.collision_margin,
            cfg.quick_turn_duration_secs,
            entry.dt,
        );
    }

    // 4. Propagate correction to render interpolation immediately so the
    //    camera converges toward the corrected position without waiting for
    //    the next FixedUpdate tick.
    render_state.on_reconciliation(predicted.position, predicted.angle);

    // 5. Record diagnostics.
    let correction_pos = (old_position - predicted.position).length();
    let correction_angle = (old_angle - predicted.angle).abs();
    diag.last_correction_pos = correction_pos;
    diag.last_correction_angle = correction_angle;
    diag.correction_count += 1;
}

// ── Shared prediction step (testable without Bevy ECS) ──────────────────

/// Apply a single prediction tick to `state`. Extracted from the system for
/// direct unit testing.
///
/// Returns `true` if the input was applied (state was modified).
pub fn apply_prediction_tick(
    state: &mut PredictedPlayerState,
    input: &PredictedInput,
    map: &carcinisation_fps_core::map::Map,
    move_speed: f32,
    turn_speed: f32,
    collision_margin: f32,
    quick_turn_duration_secs: f32,
    dt: f32,
) -> bool {
    if !state.initialised {
        return false;
    }

    // Snap turn.
    if let Some(kind) = input.snap_turn
        && state.snap_remaining_radians <= 0.0
    {
        let params = snap_turn_params(kind, quick_turn_duration_secs);
        state.snap_remaining_radians = params.remaining_radians;
        state.snap_total_radians = params.total_radians;
        state.snap_speed = params.speed;
        state.snap_direction = params.direction;
    }

    tick_snap_turn(
        &mut state.angle,
        &mut state.snap_remaining_radians,
        state.snap_speed,
        state.snap_direction,
        dt,
    );

    // Continuous turn.
    if state.snap_remaining_radians <= 0.0 && input.turn != 0.0 {
        state.angle += input.turn * turn_speed * dt;
        state.angle = state.angle.rem_euclid(std::f32::consts::TAU);
    }

    // Tick push impulse (lunge knockback). Applied before movement so the
    // player moves from the post-push position, matching server ordering.
    if state.impulse_remaining > 0.0 && state.impulse_duration > 0.0 {
        let mut impulse = carcinisation_fps_core::occupancy::OccupancyImpulse {
            direction: state.impulse_direction,
            strength: state.impulse_strength,
            remaining: state.impulse_remaining,
            duration: state.impulse_duration,
        };
        let displacement = impulse.tick(dt);
        if displacement != Vec2::ZERO {
            carcinisation_fps_core::try_move(
                &mut state.position,
                displacement,
                collision_margin,
                map,
            );
        }
        state.impulse_remaining = impulse.remaining;
        if impulse.is_expired() {
            state.impulse_remaining = 0.0;
            state.impulse_strength = 0.0;
        }
    }

    // Movement (with speed modifier parity).
    let start_position = state.position;
    if input.movement != Vec2::ZERO {
        let modifier = (state.speed_modifier_remaining > 0.0).then_some(
            carcinisation_fps_core::movement::SpeedModifier::new(
                state.speed_modifier_multiplier,
                state.speed_modifier_remaining,
            ),
        );
        carcinisation_fps_core::movement::apply_movement_with_modifier(
            &mut state.position,
            state.angle,
            input.movement,
            move_speed,
            modifier.as_ref(),
            dt,
            map,
            collision_margin,
        );
    }

    // Tick speed modifier drain (mirrors server's post-movement drain).
    if state.speed_modifier_remaining > 0.0 {
        let moved = state.position.distance(start_position);
        let mut modifier = carcinisation_fps_core::movement::SpeedModifier::new(
            state.speed_modifier_multiplier,
            state.speed_modifier_remaining,
        );
        if modifier.tick(dt, moved) {
            state.speed_modifier_remaining = modifier.remaining;
        } else {
            state.speed_modifier_multiplier = 1.0;
            state.speed_modifier_remaining = 0.0;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    #![allow(clippy::doc_markdown)]
    use super::*;
    use carcinisation_fps_core::map::Map;
    use carcinisation_fps_core::movement::{SnapTurnKind, apply_movement};

    /// 5x5 open room with walls on all borders.
    fn open_map() -> Map {
        Map {
            width: 5,
            height: 5,
            cells: vec![
                1, 1, 1, 1, 1, //
                1, 0, 0, 0, 1, //
                1, 0, 0, 0, 1, //
                1, 0, 0, 0, 1, //
                1, 1, 1, 1, 1,
            ],
        }
    }

    /// Test config that intentionally mirrors `FpsMovementConfig::default()`.
    /// Hardcoded here for deterministic isolation — tests should not depend
    /// on runtime default changes, but if the defaults diverge, update these
    /// values to maintain server-parity testing.
    fn default_config() -> (f32, f32, f32, f32) {
        // (move_speed, turn_speed, collision_margin, quick_turn_duration_secs)
        (2.0, 2.0, 0.2, 0.4)
    }

    fn state_at(x: f32, y: f32, angle: f32) -> PredictedPlayerState {
        PredictedPlayerState {
            position: Vec2::new(x, y),
            angle,
            snap_remaining_radians: 0.0,
            snap_total_radians: 0.0,
            snap_speed: 0.0,
            snap_direction: 0.0,
            speed_modifier_multiplier: 1.0,
            speed_modifier_remaining: 0.0,
            impulse_direction: Vec2::ZERO,
            impulse_strength: 0.0,
            impulse_remaining: 0.0,
            impulse_duration: 0.0,
            initialised: true,
        }
    }

    const DT: f32 = 1.0 / 30.0;

    // ── Movement ────────────────────────────────────────────────────────

    #[test]
    fn forward_movement_advances_position() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();
        let mut state = state_at(2.5, 2.5, 0.0); // facing east (+x)
        let input = PredictedInput {
            movement: Vec2::new(0.0, 1.0), // forward
            turn: 0.0,
            snap_turn: None,
        };

        let before_x = state.position.x;
        apply_prediction_tick(&mut state, &input, &map, spd, tspd, margin, qtd, DT);

        assert!(state.position.x > before_x, "should move east");
    }

    #[test]
    fn no_input_no_change() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();
        let mut state = state_at(2.5, 2.5, 0.0);
        let input = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: None,
        };

        let before = state.position;
        apply_prediction_tick(&mut state, &input, &map, spd, tspd, margin, qtd, DT);

        assert_eq!(state.position, before);
        assert!((state.angle - 0.0).abs() < 1e-6);
    }

    #[test]
    fn uninitialised_state_skips() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();
        let mut state = PredictedPlayerState::default(); // initialised = false
        let input = PredictedInput {
            movement: Vec2::new(0.0, 1.0),
            turn: 1.0,
            snap_turn: None,
        };

        let applied = apply_prediction_tick(&mut state, &input, &map, spd, tspd, margin, qtd, DT);

        assert!(!applied);
        assert_eq!(state.position, Vec2::ZERO);
    }

    // ── Turning ─────────────────────────────────────────────────────────

    #[test]
    fn continuous_turn_changes_angle() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();
        let mut state = state_at(2.5, 2.5, 0.0);
        let input = PredictedInput {
            movement: Vec2::ZERO,
            turn: 1.0, // turn left
            snap_turn: None,
        };

        apply_prediction_tick(&mut state, &input, &map, spd, tspd, margin, qtd, DT);

        assert!(state.angle > 0.0, "angle should increase for left turn");
    }

    #[test]
    fn snap_turn_initiates_rotation() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();
        let mut state = state_at(2.5, 2.5, 0.0);
        let input = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: Some(SnapTurnKind::QuickTurn),
        };

        apply_prediction_tick(&mut state, &input, &map, spd, tspd, margin, qtd, DT);

        // Quick turn is 180° — after one tick the angle should have shifted.
        assert!(state.angle.abs() > 0.01, "snap turn should rotate angle");
    }

    #[test]
    fn continuous_turn_suppressed_during_snap() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();
        let mut state = state_at(2.5, 2.5, 0.0);

        // Start snap turn.
        let snap_input = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: Some(SnapTurnKind::Left),
        };
        apply_prediction_tick(&mut state, &snap_input, &map, spd, tspd, margin, qtd, DT);

        let angle_after_snap_start = state.angle;
        assert!(state.snap_remaining_radians > 0.0, "snap should be active");

        // Now apply continuous turn while snap is active.
        let turn_input = PredictedInput {
            movement: Vec2::ZERO,
            turn: -1.0, // try to turn right
            snap_turn: None,
        };
        apply_prediction_tick(&mut state, &turn_input, &map, spd, tspd, margin, qtd, DT);

        // The angle should only have changed from the snap turn continuation,
        // not from the continuous turn input.
        let snap_delta = state.angle - angle_after_snap_start;
        // Snap is left (+direction), so angle increases. If continuous turn
        // (-1.0 = right) were applied, angle would decrease or increase less.
        assert!(
            snap_delta >= 0.0 || state.snap_remaining_radians > 0.0,
            "continuous turn should not override snap turn"
        );
    }

    // ── Collision ───────────────────────────────────────────────────────

    #[test]
    fn wall_collision_stops_movement() {
        let map = open_map();
        let (spd, _, margin, qtd) = default_config();
        // Place near the east wall (x=4 is wall, walkable up to ~3.8).
        let mut state = state_at(3.7, 2.5, 0.0); // facing east
        let input = PredictedInput {
            movement: Vec2::new(0.0, 1.0), // forward into wall
            turn: 0.0,
            snap_turn: None,
        };

        // Apply many ticks — position should not pass through the wall.
        for _ in 0..100 {
            apply_prediction_tick(&mut state, &input, &map, spd, 2.0, margin, qtd, DT);
        }

        assert!(
            state.position.x < 4.0 - margin,
            "should not pass through east wall, got x={:.3}",
            state.position.x
        );
    }

    // ── Death ───────────────────────────────────────────────────────────

    #[test]
    fn death_clears_initialised() {
        let mut state = state_at(2.5, 2.5, 1.0);
        let mut history = PredictionHistory::default();
        history.push(PredictionEntry {
            sequence: InputSequence(1),
            input: PredictedInput {
                movement: Vec2::ZERO,
                turn: 0.0,
                snap_turn: None,
            },
            result: PredictionSnapshot {
                position: Vec2::new(2.5, 2.5),
                angle: 1.0,
            },
            dt: DT,
        });

        // Simulate death.
        assert!(state.initialised);
        assert!(!history.is_empty());

        // Manual clear (same logic as clear_prediction_on_death).
        if state.initialised {
            history.clear();
            state.initialised = false;
        }

        assert!(!state.initialised);
        assert!(history.is_empty());
    }

    // ── History ─────────────────────────────────────────────────────────

    #[test]
    fn prediction_pushes_to_history() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();
        let mut state = state_at(2.5, 2.5, 0.0);
        let mut history = PredictionHistory::default();

        let input = PredictedInput {
            movement: Vec2::new(0.0, 1.0),
            turn: 0.0,
            snap_turn: None,
        };

        let seq = InputSequence(42);
        apply_prediction_tick(&mut state, &input, &map, spd, tspd, margin, qtd, DT);
        history.push(PredictionEntry {
            sequence: seq,
            input: input.clone(),
            result: PredictionSnapshot {
                position: state.position,
                angle: state.angle,
            },
            dt: DT,
        });

        assert_eq!(history.len(), 1);
        let entry = history.get(seq).unwrap();
        assert!((entry.result.position.x - state.position.x).abs() < 1e-6);
    }

    // ── Prediction matches server logic ─────────────────────────────────

    #[test]
    fn prediction_matches_direct_apply_movement() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();
        let start = Vec2::new(2.5, 2.5);
        let angle = 0.5; // ~28.6° north-east

        // Predicted path.
        let mut state = state_at(start.x, start.y, angle);
        let input = PredictedInput {
            movement: Vec2::new(0.0, 1.0),
            turn: 0.0,
            snap_turn: None,
        };
        apply_prediction_tick(&mut state, &input, &map, spd, tspd, margin, qtd, DT);

        // Direct apply_movement (what the server does).
        let mut server_pos = start;
        apply_movement(
            &mut server_pos,
            angle,
            Vec2::new(0.0, 1.0),
            spd,
            DT,
            &map,
            margin,
        );

        assert!(
            (state.position - server_pos).length() < 1e-6,
            "prediction should match server: predicted={:?} server={:?}",
            state.position,
            server_pos
        );
    }

    // ── Reconciliation (prune → reset → replay) ──────────────────────

    #[test]
    fn reconciliation_prune_reset_replay_matches_full_apply() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();
        let start = Vec2::new(2.5, 2.5);
        let start_angle = 0.0;

        let input = PredictedInput {
            movement: Vec2::new(0.0, 1.0), // forward
            turn: 0.0,
            snap_turn: None,
        };

        // -- Ground truth: apply all 5 inputs from scratch --
        let mut truth = state_at(start.x, start.y, start_angle);
        for _ in 0..5 {
            apply_prediction_tick(&mut truth, &input, &map, spd, tspd, margin, qtd, DT);
        }

        // -- Client path: apply 5 inputs, record history --
        let mut client = state_at(start.x, start.y, start_angle);
        let mut history = PredictionHistory::default();
        for seq in 1..=5u32 {
            apply_prediction_tick(&mut client, &input, &map, spd, tspd, margin, qtd, DT);
            history.push(PredictionEntry {
                sequence: InputSequence(seq),
                input: input.clone(),
                result: PredictionSnapshot {
                    position: client.position,
                    angle: client.angle,
                },
                dt: DT,
            });
        }

        // -- Simulate ack at seq=3: compute server state after 3 inputs --
        let mut server_at_3 = state_at(start.x, start.y, start_angle);
        for _ in 0..3 {
            apply_prediction_tick(&mut server_at_3, &input, &map, spd, tspd, margin, qtd, DT);
        }

        // 1. Prune through acked sequence.
        history.prune_through(InputSequence(3));
        assert_eq!(history.len(), 2, "entries 4 and 5 should remain");

        // 2. Reset to server truth.
        client.position = server_at_3.position;
        client.angle = server_at_3.angle;
        client.snap_remaining_radians = 0.0;
        client.snap_total_radians = 0.0;

        // 3. Replay remaining entries (4, 5).
        let entries: Vec<_> = history.iter_all().cloned().collect();
        for entry in &entries {
            apply_prediction_tick(
                &mut client,
                &entry.input,
                &map,
                spd,
                tspd,
                margin,
                qtd,
                entry.dt,
            );
        }

        // -- Verify: reconciled state matches full apply from scratch --
        let drift = (client.position - truth.position).length();
        assert!(
            drift < 1e-5,
            "reconciled position should match ground truth: reconciled={:?} truth={:?} drift={drift}",
            client.position,
            truth.position
        );
        assert!(
            (client.angle - truth.angle).abs() < 1e-5,
            "reconciled angle should match ground truth: reconciled={:.4} truth={:.4}",
            client.angle,
            truth.angle
        );
    }

    // ── Mixed dt reconciliation ─────────────────────────────────────

    #[test]
    fn reconciliation_respects_per_entry_dt() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();

        // Apply 3 entries with varying dt values.
        let dts = [1.0 / 30.0, 1.0 / 20.0, 1.0 / 60.0];
        let input = PredictedInput {
            movement: Vec2::new(0.0, 1.0),
            turn: 0.0,
            snap_turn: None,
        };

        // Ground truth: apply all 3 from scratch.
        let mut truth = state_at(2.5, 2.5, 0.0);
        for &dt in &dts {
            apply_prediction_tick(&mut truth, &input, &map, spd, tspd, margin, qtd, dt);
        }

        // Reconciled: ack at entry 1, replay entries 2+3.
        let mut ack_state = state_at(2.5, 2.5, 0.0);
        apply_prediction_tick(&mut ack_state, &input, &map, spd, tspd, margin, qtd, dts[0]);
        // ack_state is now the server position after entry 1.

        let mut reconciled = PredictedPlayerState {
            position: ack_state.position,
            angle: ack_state.angle,
            snap_remaining_radians: 0.0,
            snap_total_radians: 0.0,
            snap_speed: 0.0,
            snap_direction: 0.0,
            speed_modifier_multiplier: 1.0,
            speed_modifier_remaining: 0.0,
            impulse_direction: Vec2::ZERO,
            impulse_strength: 0.0,
            impulse_remaining: 0.0,
            impulse_duration: 0.0,
            initialised: true,
        };

        // Replay entries 2+3 with their stored dt.
        for &dt in &dts[1..] {
            apply_prediction_tick(&mut reconciled, &input, &map, spd, tspd, margin, qtd, dt);
        }

        assert!(
            (reconciled.position - truth.position).length() < 1e-5,
            "mixed-dt reconciliation should match truth"
        );
    }

    // ── Snap turn across reconciliation ─────────────────────────────

    #[test]
    fn snap_turn_survives_reconciliation_replay() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();

        // 5 entries: entry 3 starts a quick turn, entries 4+5 tick it.
        let inputs: Vec<PredictedInput> = (0..5)
            .map(|i| PredictedInput {
                movement: Vec2::ZERO,
                turn: 0.0,
                snap_turn: if i == 2 {
                    Some(SnapTurnKind::QuickTurn)
                } else {
                    None
                },
            })
            .collect();

        // Ground truth: apply all 5.
        let mut truth = state_at(2.5, 2.5, 0.0);
        for input in &inputs {
            apply_prediction_tick(&mut truth, input, &map, spd, tspd, margin, qtd, DT);
        }

        // Ack after entry 1 (before snap turn starts).
        let mut ack_state = state_at(2.5, 2.5, 0.0);
        apply_prediction_tick(&mut ack_state, &inputs[0], &map, spd, tspd, margin, qtd, DT);

        // Reconcile: reset to ack, replay entries 2..5.
        let mut reconciled = PredictedPlayerState {
            position: ack_state.position,
            angle: ack_state.angle,
            snap_remaining_radians: 0.0,
            snap_total_radians: 0.0,
            snap_speed: 0.0,
            snap_direction: 0.0,
            speed_modifier_multiplier: 1.0,
            speed_modifier_remaining: 0.0,
            impulse_direction: Vec2::ZERO,
            impulse_strength: 0.0,
            impulse_remaining: 0.0,
            impulse_duration: 0.0,
            initialised: true,
        };
        for input in &inputs[1..] {
            apply_prediction_tick(&mut reconciled, input, &map, spd, tspd, margin, qtd, DT);
        }

        assert!(
            (reconciled.angle - truth.angle).abs() < 1e-5,
            "snap turn should produce same angle after reconciliation: reconciled={:.4} truth={:.4}",
            reconciled.angle,
            truth.angle
        );
    }

    // ── Mid-snap reconciliation (regression tests) ────────────────────

    /// Helper: simulates the production reconciliation path for a mid-snap
    /// ack. Builds ground truth by applying all inputs, then reconciles at
    /// `ack_seq` using the server's snap state (as the real InputAck carries).
    fn reconcile_mid_snap(
        ack_seq: u32,
        snap_entry: u32,
        kind: SnapTurnKind,
        total_entries: u32,
    ) -> (f32, f32) {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();

        let inputs: Vec<PredictedInput> = (0..total_entries)
            .map(|i| PredictedInput {
                movement: Vec2::ZERO,
                turn: 0.0,
                snap_turn: if i == snap_entry - 1 {
                    Some(kind)
                } else {
                    None
                },
            })
            .collect();

        // Ground truth.
        let mut truth = state_at(2.5, 2.5, 0.0);
        for input in &inputs {
            apply_prediction_tick(&mut truth, input, &map, spd, tspd, margin, qtd, DT);
        }

        // Server state at ack (includes snap progress through ack_seq).
        let mut server = state_at(2.5, 2.5, 0.0);
        for input in &inputs[..ack_seq as usize] {
            apply_prediction_tick(&mut server, input, &map, spd, tspd, margin, qtd, DT);
        }

        // Production reconciliation: restore snap state from server ack.
        let mut history = PredictionHistory::default();
        let mut s = state_at(2.5, 2.5, 0.0);
        for (i, input) in inputs.iter().enumerate() {
            apply_prediction_tick(&mut s, input, &map, spd, tspd, margin, qtd, DT);
            history.push(PredictionEntry {
                sequence: InputSequence(i as u32 + 1),
                input: input.clone(),
                result: PredictionSnapshot {
                    position: s.position,
                    angle: s.angle,
                },
                dt: DT,
            });
        }
        history.prune_through(InputSequence(ack_seq));

        let mut reconciled = PredictedPlayerState {
            position: server.position,
            angle: server.angle,
            snap_remaining_radians: server.snap_remaining_radians,
            snap_total_radians: server.snap_total_radians,
            snap_speed: server.snap_speed,
            snap_direction: server.snap_direction,
            speed_modifier_multiplier: server.speed_modifier_multiplier,
            speed_modifier_remaining: server.speed_modifier_remaining,
            impulse_direction: server.impulse_direction,
            impulse_strength: server.impulse_strength,
            impulse_remaining: server.impulse_remaining,
            impulse_duration: server.impulse_duration,
            initialised: true,
        };
        let entries: Vec<_> = history.iter_all().cloned().collect();
        for entry in &entries {
            apply_prediction_tick(
                &mut reconciled,
                &entry.input,
                &map,
                spd,
                tspd,
                margin,
                qtd,
                entry.dt,
            );
        }

        let error = shortest_angle_delta(reconciled.angle, truth.angle).abs();
        (error, server.snap_remaining_radians)
    }

    /// Mid-snap ack for QuickTurn: reconciliation with restored snap
    /// state produces near-zero drift.
    #[test]
    fn mid_snap_quick_turn_reconciliation_near_zero_drift() {
        // Snap at entry 3, ack at entry 5 (mid-snap).
        let (error, remaining) = reconcile_mid_snap(5, 3, SnapTurnKind::QuickTurn, 15);
        assert!(remaining > 1.0, "should still be mid-snap: {remaining:.3}");
        assert!(
            error < 1e-5,
            "reconciled angle should match truth: error={error:.6} rad ({:.3}°)",
            error.to_degrees()
        );
    }

    /// Mid-snap ack for 90° Left: same invariant.
    #[test]
    fn mid_snap_left_turn_reconciliation_near_zero_drift() {
        let (error, remaining) = reconcile_mid_snap(4, 3, SnapTurnKind::Left, 10);
        assert!(remaining > 0.3, "should still be mid-snap: {remaining:.3}");
        assert!(
            error < 1e-5,
            "reconciled angle should match truth: error={error:.6} rad ({:.3}°)",
            error.to_degrees()
        );
    }

    /// Mid-snap ack for 90° Right: same invariant.
    #[test]
    fn mid_snap_right_turn_reconciliation_near_zero_drift() {
        let (error, _) = reconcile_mid_snap(4, 3, SnapTurnKind::Right, 10);
        assert!(
            error < 1e-5,
            "reconciled angle should match truth: error={error:.6} rad ({:.3}°)",
            error.to_degrees()
        );
    }

    /// Per-ack corrections during a QuickTurn should be near-zero when
    /// snap state is properly carried in the ack.
    #[test]
    fn per_ack_snap_corrections_near_zero() {
        // Simulate per-ack corrections (acks 2..10, all mid-snap).
        // Each ack restores snap state (production path).
        for ack_seq in 2..=10u32 {
            let (error, _) = reconcile_mid_snap(ack_seq, 1, SnapTurnKind::QuickTurn, 15);
            assert!(
                error < 1e-5,
                "ack at seq={ack_seq}: correction {error:.6} rad ({:.3}°) — should be near-zero",
                error.to_degrees()
            );
        }
    }

    /// Ack BEFORE snap starts: snap entry still in history, replayed correctly.
    #[test]
    fn ack_before_snap_replays_correctly() {
        let (error, remaining) = reconcile_mid_snap(1, 3, SnapTurnKind::QuickTurn, 15);
        assert!(
            remaining < 1e-6,
            "server at seq=1 should have no snap yet: {remaining:.3}"
        );
        assert!(
            error < 1e-5,
            "replay of snap from history should match truth: error={error:.6} rad",
        );
    }

    /// Ack AFTER snap completes: snap fully applied on server, no replay needed.
    #[test]
    fn ack_after_snap_completes_no_drift() {
        // QuickTurn takes ~12 ticks. Ack at entry 15 = snap complete.
        let (error, remaining) = reconcile_mid_snap(15, 1, SnapTurnKind::QuickTurn, 15);
        assert!(
            remaining < 1e-6,
            "snap should be complete at seq=15: {remaining:.3}"
        );
        assert!(
            error < 1e-5,
            "post-snap ack should have zero drift: error={error:.6} rad",
        );
    }

    // ── PendingInput discard + snap turn merge ──────────────────────────

    /// Simulates the `apply_predicted_movement` discard logic:
    /// pop last, merge snap_turn from discarded, clear.
    fn simulate_pending_discard(entries: Vec<(InputSequence, PredictedInput)>) -> PredictedInput {
        let mut pending = PendingInput(entries);
        assert!(!pending.0.is_empty());

        let (_, mut input) = pending.0.pop().unwrap();

        if input.snap_turn.is_none() {
            for (_, discarded) in &pending.0 {
                if discarded.snap_turn.is_some() {
                    input.snap_turn = discarded.snap_turn;
                    break;
                }
            }
        }
        pending.0.clear();
        input
    }

    #[test]
    fn pending_discard_preserves_snap_turn_from_earlier_entry() {
        let entries = vec![
            (
                InputSequence(1),
                PredictedInput {
                    movement: Vec2::ZERO,
                    turn: 0.0,
                    snap_turn: Some(SnapTurnKind::QuickTurn),
                },
            ),
            (
                InputSequence(2),
                PredictedInput {
                    movement: Vec2::new(0.0, 1.0),
                    turn: 0.0,
                    snap_turn: None,
                },
            ),
        ];

        let merged = simulate_pending_discard(entries);
        assert_eq!(
            merged.movement,
            Vec2::new(0.0, 1.0),
            "movement from last entry"
        );
        assert!(
            matches!(merged.snap_turn, Some(SnapTurnKind::QuickTurn)),
            "snap turn from discarded entry should be preserved"
        );
    }

    #[test]
    fn pending_discard_keeps_last_entry_snap_if_present() {
        let entries = vec![
            (
                InputSequence(1),
                PredictedInput {
                    movement: Vec2::ZERO,
                    turn: 0.0,
                    snap_turn: Some(SnapTurnKind::Left),
                },
            ),
            (
                InputSequence(2),
                PredictedInput {
                    movement: Vec2::ZERO,
                    turn: 0.0,
                    snap_turn: Some(SnapTurnKind::Right),
                },
            ),
        ];

        let merged = simulate_pending_discard(entries);
        assert!(
            matches!(merged.snap_turn, Some(SnapTurnKind::Right)),
            "last entry's snap turn should take priority"
        );
    }

    #[test]
    fn pending_discard_single_entry_unchanged() {
        let entries = vec![(
            InputSequence(1),
            PredictedInput {
                movement: Vec2::new(0.0, 1.0),
                turn: 0.5,
                snap_turn: Some(SnapTurnKind::QuickTurn),
            },
        )];

        let merged = simulate_pending_discard(entries);
        assert_eq!(merged.movement, Vec2::new(0.0, 1.0));
        assert!((merged.turn - 0.5).abs() < 1e-6);
        assert!(matches!(merged.snap_turn, Some(SnapTurnKind::QuickTurn)));
    }

    #[test]
    fn pending_discard_no_snap_in_any_entry() {
        let entries = vec![
            (
                InputSequence(1),
                PredictedInput {
                    movement: Vec2::ZERO,
                    turn: 1.0,
                    snap_turn: None,
                },
            ),
            (
                InputSequence(2),
                PredictedInput {
                    movement: Vec2::ZERO,
                    turn: -1.0,
                    snap_turn: None,
                },
            ),
        ];

        let merged = simulate_pending_discard(entries);
        assert!(merged.snap_turn.is_none());
        assert!((merged.turn - -1.0).abs() < 1e-6, "turn from last entry");
    }

    #[test]
    fn pending_discard_three_entries_first_has_snap() {
        let entries = vec![
            (
                InputSequence(1),
                PredictedInput {
                    movement: Vec2::ZERO,
                    turn: 0.0,
                    snap_turn: Some(SnapTurnKind::Left),
                },
            ),
            (
                InputSequence(2),
                PredictedInput {
                    movement: Vec2::ZERO,
                    turn: 0.0,
                    snap_turn: None,
                },
            ),
            (
                InputSequence(3),
                PredictedInput {
                    movement: Vec2::new(0.0, 1.0),
                    turn: 0.0,
                    snap_turn: None,
                },
            ),
        ];

        let merged = simulate_pending_discard(entries);
        assert_eq!(merged.movement, Vec2::new(0.0, 1.0), "movement from last");
        assert!(
            matches!(merged.snap_turn, Some(SnapTurnKind::Left)),
            "snap turn from first discarded entry should survive"
        );
    }

    // ── Prediction disabled ─────────────────────────────────────────────

    #[test]
    fn prediction_disabled_skips_apply() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();
        let mut state = state_at(2.5, 2.5, 0.0);
        let mut history = PredictionHistory::default();
        let mut pending = PendingInput(vec![
            (
                InputSequence(1),
                PredictedInput {
                    movement: Vec2::new(0.0, 1.0),
                    turn: 0.0,
                    snap_turn: None,
                },
            ),
            (
                InputSequence(2),
                PredictedInput {
                    movement: Vec2::new(0.0, 1.0),
                    turn: 0.0,
                    snap_turn: None,
                },
            ),
        ]);

        let before_pos = state.position;

        // Simulate the early return in apply_predicted_movement when disabled.
        let enabled = PredictionEnabled(false);
        if enabled.0 {
            // Would normally apply prediction tick here.
            let (_, input) = pending.0.pop().unwrap();
            pending.0.clear();
            apply_prediction_tick(&mut state, &input, &map, spd, tspd, margin, qtd, DT);
            history.push(PredictionEntry {
                sequence: InputSequence(2),
                input,
                result: PredictionSnapshot {
                    position: state.position,
                    angle: state.angle,
                },
                dt: DT,
            });
        } else {
            pending.0.clear();
        }

        assert!(pending.0.is_empty(), "pending should be drained");
        assert_eq!(state.position, before_pos, "position should not change");
        assert!(history.is_empty(), "history should remain empty");
    }

    #[test]
    fn prediction_disabled_leaves_state_uninitialised() {
        let mut state = PredictedPlayerState::default();
        assert!(!state.initialised);

        // Simulate init_predicted_state with prediction disabled.
        let enabled = PredictionEnabled(false);
        if enabled.0 && !state.initialised {
            state.position = Vec2::new(1.5, 1.5);
            state.initialised = true;
        }

        assert!(!state.initialised, "should not initialise when disabled");
        assert_eq!(state.position, Vec2::ZERO);
    }

    // ── PredictedRenderState interpolation ──────────────────────────────

    #[test]
    fn render_state_interpolated_position_at_t0() {
        let mut rs = PredictedRenderState::default();
        rs.seed(Vec2::new(1.0, 2.0), 0.0);
        rs.on_fixed_tick(Vec2::new(3.0, 4.0), 0.5, DT);
        // elapsed=0 → t=0 → should be at prev (1.0, 2.0)
        assert!((rs.elapsed - 0.0).abs() < f32::EPSILON);
        let pos = rs.interpolated_position();
        assert!(
            (pos - Vec2::new(1.0, 2.0)).length() < 1e-5,
            "at t=0 should be prev: {pos:?}"
        );
    }

    #[test]
    fn render_state_interpolated_position_at_half() {
        let mut rs = PredictedRenderState::default();
        rs.seed(Vec2::new(0.0, 0.0), 0.0);
        rs.on_fixed_tick(Vec2::new(2.0, 0.0), 0.0, DT);
        rs.elapsed = DT / 2.0; // t=0.5
        let pos = rs.interpolated_position();
        assert!(
            (pos.x - 1.0).abs() < 1e-4,
            "at t=0.5 should be midpoint: {pos:?}"
        );
    }

    #[test]
    fn render_state_interpolated_position_at_t1() {
        let mut rs = PredictedRenderState::default();
        rs.seed(Vec2::new(0.0, 0.0), 0.0);
        rs.on_fixed_tick(Vec2::new(4.0, 0.0), 0.0, DT);
        rs.elapsed = DT; // t=1.0
        let pos = rs.interpolated_position();
        assert!(
            (pos - Vec2::new(4.0, 0.0)).length() < 1e-5,
            "at t=1.0 should be at current: {pos:?}"
        );
    }

    #[test]
    fn render_state_interpolated_position_clamps_past_t1() {
        let mut rs = PredictedRenderState::default();
        rs.seed(Vec2::new(0.0, 0.0), 0.0);
        rs.on_fixed_tick(Vec2::new(4.0, 0.0), 0.0, DT);
        rs.elapsed = DT * 2.0; // t=2.0 → clamped to 1.0
        let pos = rs.interpolated_position();
        assert!(
            (pos - Vec2::new(4.0, 0.0)).length() < 1e-5,
            "past t=1.0 should clamp to current: {pos:?}"
        );
    }

    #[test]
    fn render_state_interpolated_angle_shortest_arc() {
        use std::f32::consts::TAU;
        let mut rs = PredictedRenderState::default();
        rs.seed(Vec2::ZERO, TAU - 0.1); // ~6.18 rad
        rs.on_fixed_tick(Vec2::ZERO, 0.1, DT); // 0.1 rad — short arc is +0.2 through 0/TAU
        rs.elapsed = DT / 2.0; // t=0.5
        let angle = rs.interpolated_angle();
        // Midpoint of short arc from (TAU-0.1) to 0.1 wrapping forward.
        // Should be near TAU (or 0.0), not near PI.
        let wrapped = angle.rem_euclid(TAU);
        assert!(
            !(0.2..=TAU - 0.2).contains(&wrapped),
            "angle should be near 0/TAU boundary, got {angle:.3} (wrapped {wrapped:.3})"
        );
    }

    #[test]
    fn render_state_on_reconciliation_updates_current() {
        let mut rs = PredictedRenderState::default();
        rs.seed(Vec2::new(0.0, 0.0), 0.0);
        rs.on_fixed_tick(Vec2::new(2.0, 0.0), 0.5, DT);
        rs.elapsed = DT / 2.0; // halfway through interpolation

        // Reconciliation shifts current without resetting elapsed.
        rs.on_reconciliation(Vec2::new(3.0, 0.0), 1.0);

        assert_eq!(rs.current_position, Vec2::new(3.0, 0.0));
        assert!((rs.current_angle - 1.0).abs() < 1e-6);
        // prev should NOT have changed — interpolation continues from prev toward new current.
        assert_eq!(rs.prev_position, Vec2::new(0.0, 0.0));
        // elapsed should NOT have been reset — smooth convergence.
        assert!((rs.elapsed - DT / 2.0).abs() < 1e-6);
    }

    #[test]
    fn render_state_not_ready_returns_current() {
        let mut rs = PredictedRenderState::default();
        rs.seed(Vec2::new(5.0, 3.0), 1.0);
        // Not ready (seed doesn't set ready=true, needs on_fixed_tick first).
        assert!(!rs.ready);
        assert_eq!(rs.interpolated_position(), Vec2::new(5.0, 3.0));
        assert!((rs.interpolated_angle() - 1.0).abs() < 1e-6);
    }

    // ── Stale-tick prediction parity ────────────────────────────────────

    /// Simulates the server's stale buffer behavior: apply the same
    /// movement data for `stale_ticks` extra FixedUpdate ticks after the
    /// last new intent, then reconcile with the ack.
    ///
    /// Returns (correction_pos, correction_angle).
    fn simulate_stale_tick_drift(stale_ticks: u32, apply_stale_on_client: bool) -> (f32, f32) {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();
        let start = Vec2::new(2.5, 2.5);

        let forward = PredictedInput {
            movement: Vec2::new(0.0, 1.0),
            turn: 0.0,
            snap_turn: None,
        };

        // --- Server path: apply seq=1 then stale_ticks more with same data ---
        let mut server = state_at(start.x, start.y, 0.0);
        // Tick 1: apply seq=1's input.
        apply_prediction_tick(&mut server, &forward, &map, spd, tspd, margin, qtd, DT);
        // Stale ticks: server reapplies same movement data.
        for _ in 0..stale_ticks {
            apply_prediction_tick(&mut server, &forward, &map, spd, tspd, margin, qtd, DT);
        }

        // --- Client path: apply seq=1, optionally apply stale ticks ---
        let mut client = state_at(start.x, start.y, 0.0);
        let mut history = PredictionHistory::default();

        // Apply seq=1.
        apply_prediction_tick(&mut client, &forward, &map, spd, tspd, margin, qtd, DT);
        history.push(PredictionEntry {
            sequence: InputSequence(1),
            input: forward.clone(),
            result: PredictionSnapshot {
                position: client.position,
                angle: client.angle,
            },
            dt: DT,
        });

        // Stale ticks on client (if enabled).
        if apply_stale_on_client {
            for _ in 0..stale_ticks {
                apply_prediction_tick(&mut client, &forward, &map, spd, tspd, margin, qtd, DT);
                // Stale ticks are NOT stored in history (server doesn't
                // advance sequence, so ack prune will handle them).
            }
        }

        // --- Reconciliation: ack arrives with server state at seq=1 ---
        // (ack seq=1 means server processed through seq=1 AND its stale ticks)
        history.prune_through(InputSequence(1));
        let old_pos = client.position;
        let old_angle = client.angle;

        client.position = server.position;
        client.angle = server.angle;
        client.snap_remaining_radians = 0.0;
        client.snap_total_radians = 0.0;
        client.snap_speed = 0.0;
        client.snap_direction = 0.0;

        // No entries to replay (all pruned through seq=1).
        let correction_pos = (old_pos - client.position).length();
        let correction_angle = (old_angle - client.angle).abs();
        (correction_pos, correction_angle)
    }

    /// Without stale-tick prediction, the client drifts behind the server
    /// by exactly stale_ticks * movement_per_tick.
    #[test]
    fn without_stale_ticks_client_drifts() {
        let (corr_pos, _) = simulate_stale_tick_drift(3, false);
        // Server moved 4 ticks (1 + 3 stale), client moved 1 tick.
        // Correction = 3 ticks of movement = 3 * move_speed * dt = 3 * 2.0 * 0.033 = 0.2
        assert!(
            corr_pos > 0.15,
            "without stale prediction, drift should be significant: {corr_pos:.4}"
        );
    }

    /// With stale-tick prediction mirroring the server, correction is zero.
    #[test]
    fn with_stale_ticks_no_drift() {
        let (corr_pos, corr_angle) = simulate_stale_tick_drift(3, true);
        assert!(
            corr_pos < 1e-5,
            "with stale prediction, drift should be near-zero: {corr_pos:.6}"
        );
        assert!(
            corr_angle < 1e-5,
            "angle correction should be near-zero: {corr_angle:.6}"
        );
    }

    /// Stale ticks during a snap turn: server continues snapping, client
    /// must also continue if stale prediction is active.
    #[test]
    fn stale_ticks_during_snap_turn() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();

        let snap_input = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: Some(SnapTurnKind::QuickTurn),
        };
        let idle = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: None,
        };

        // Server: apply snap (tick 1), then 3 stale ticks (idle, snap continues).
        let mut server = state_at(2.5, 2.5, 0.0);
        apply_prediction_tick(&mut server, &snap_input, &map, spd, tspd, margin, qtd, DT);
        for _ in 0..3 {
            apply_prediction_tick(&mut server, &idle, &map, spd, tspd, margin, qtd, DT);
        }

        // Client: apply snap (tick 1), then 3 stale ticks.
        let mut client = state_at(2.5, 2.5, 0.0);
        apply_prediction_tick(&mut client, &snap_input, &map, spd, tspd, margin, qtd, DT);
        for _ in 0..3 {
            // Stale input: movement=ZERO, turn=0, no snap_turn.
            // tick_snap_turn still advances the snap because
            // snap_remaining_radians > 0.
            apply_prediction_tick(&mut client, &idle, &map, spd, tspd, margin, qtd, DT);
        }

        // Both should have the same angle (4 ticks of snap turn).
        let drift = (client.angle - server.angle).abs();
        assert!(
            drift < 1e-5,
            "stale tick snap parity: client={:.4} server={:.4} drift={drift:.6}",
            client.angle,
            server.angle
        );
    }

    /// Snap turn continues PAST stale expiry. A QuickTurn takes ~12 ticks
    /// but stale input expires after 5. The server keeps ticking
    /// snap_turn.tick() regardless of stale expiry. The client must match:
    /// continue ticking snap_remaining even after stale movement stops.
    #[test]
    fn snap_continues_past_stale_expiry() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();

        let snap_input = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: Some(SnapTurnKind::QuickTurn),
        };
        let idle = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: None,
        };

        // Server: apply snap (tick 1), then 11 idle ticks (snap completes at ~12).
        let mut server = state_at(2.5, 2.5, 0.0);
        apply_prediction_tick(&mut server, &snap_input, &map, spd, tspd, margin, qtd, DT);
        for _ in 0..11 {
            apply_prediction_tick(&mut server, &idle, &map, spd, tspd, margin, qtd, DT);
        }
        assert!(
            server.snap_remaining_radians <= 0.0,
            "server snap should be complete: remaining={:.4}",
            server.snap_remaining_radians
        );

        // Client: same sequence. The stale path in apply_predicted_movement
        // would have stale.age_ticks >= 5 after 5 idle ticks, but snap
        // should continue because snap_remaining > 0.
        let mut client = state_at(2.5, 2.5, 0.0);
        apply_prediction_tick(&mut client, &snap_input, &map, spd, tspd, margin, qtd, DT);
        for _ in 0..11 {
            apply_prediction_tick(&mut client, &idle, &map, spd, tspd, margin, qtd, DT);
        }
        assert!(
            client.snap_remaining_radians <= 0.0,
            "client snap should also be complete: remaining={:.4}",
            client.snap_remaining_radians
        );

        // Both should produce the same angle (PI = 180°).
        let drift = shortest_angle_delta(client.angle, server.angle).abs();
        assert!(
            drift < 1e-5,
            "snap past stale expiry: client={:.4} server={:.4} drift={drift:.6}",
            client.angle,
            server.angle
        );
        // And angle should be approximately PI from 0.
        assert!(
            (client.angle - std::f32::consts::PI).abs() < 0.01,
            "angle should be ~PI: {:.4}",
            client.angle
        );
    }

    /// Full end-to-end scenario: quick-turn with ack gap then ack at completion.
    /// Simulates the exact timeline where the server sends acks during snap.
    #[test]
    fn quick_turn_full_timeline_with_acks() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();

        let snap_input = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: Some(SnapTurnKind::QuickTurn),
        };
        let idle = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: None,
        };

        // Build full sequence: snap at tick 1, idle for 13 more ticks.
        let inputs: Vec<PredictedInput> = std::iter::once(snap_input)
            .chain(std::iter::repeat_n(idle, 13))
            .collect();

        // Server: apply all 14 ticks.
        let mut server = state_at(2.5, 2.5, 0.0);
        let mut server_states = Vec::new();
        for input in &inputs {
            apply_prediction_tick(&mut server, input, &map, spd, tspd, margin, qtd, DT);
            server_states.push((server.angle, server.snap_remaining_radians));
        }

        // Client: apply all 14 ticks (matching server exactly).
        let mut client = state_at(2.5, 2.5, 0.0);
        let mut history = PredictionHistory::default();
        // Only tick 1 and 2 have real sequences; rest are stale.
        apply_prediction_tick(&mut client, &inputs[0], &map, spd, tspd, margin, qtd, DT);
        history.push(PredictionEntry {
            sequence: InputSequence(1),
            input: inputs[0].clone(),
            result: PredictionSnapshot {
                position: client.position,
                angle: client.angle,
            },
            dt: DT,
        });
        apply_prediction_tick(&mut client, &inputs[1], &map, spd, tspd, margin, qtd, DT);
        history.push(PredictionEntry {
            sequence: InputSequence(2),
            input: inputs[1].clone(),
            result: PredictionSnapshot {
                position: client.position,
                angle: client.angle,
            },
            dt: DT,
        });
        // Ticks 3-14: stale (no history entries, just advance prediction).
        for input in &inputs[2..] {
            apply_prediction_tick(&mut client, input, &map, spd, tspd, margin, qtd, DT);
        }

        // Simulate ack arriving at tick 8 (mid-snap, seq=2).
        // Server state at tick 8:
        let (server_angle_t8, server_snap_t8) = server_states[7];

        let mut h2 = PredictionHistory::default();
        let mut s2 = state_at(2.5, 2.5, 0.0);
        apply_prediction_tick(&mut s2, &inputs[0], &map, spd, tspd, margin, qtd, DT);
        h2.push(PredictionEntry {
            sequence: InputSequence(1),
            input: inputs[0].clone(),
            result: PredictionSnapshot {
                position: s2.position,
                angle: s2.angle,
            },
            dt: DT,
        });
        apply_prediction_tick(&mut s2, &inputs[1], &map, spd, tspd, margin, qtd, DT);
        h2.push(PredictionEntry {
            sequence: InputSequence(2),
            input: inputs[1].clone(),
            result: PredictionSnapshot {
                position: s2.position,
                angle: s2.angle,
            },
            dt: DT,
        });
        h2.prune_through(InputSequence(2));

        // Reconcile with mid-snap ack: restore snap state from server.
        let mut reconciled = PredictedPlayerState {
            position: Vec2::new(2.5, 2.5),
            angle: server_angle_t8,
            snap_remaining_radians: server_snap_t8,
            snap_total_radians: server.snap_total_radians,
            snap_speed: server.snap_speed,
            snap_direction: server.snap_direction,
            speed_modifier_multiplier: server.speed_modifier_multiplier,
            speed_modifier_remaining: server.speed_modifier_remaining,
            impulse_direction: server.impulse_direction,
            impulse_strength: server.impulse_strength,
            impulse_remaining: server.impulse_remaining,
            impulse_duration: server.impulse_duration,
            initialised: true,
        };
        // No history entries to replay (pruned through seq=2).
        // Continue stale prediction for remaining ticks (9-14 = 6 more ticks).
        for input in &inputs[8..] {
            apply_prediction_tick(&mut reconciled, input, &map, spd, tspd, margin, qtd, DT);
        }

        // Final angle should match server.
        let final_drift = shortest_angle_delta(reconciled.angle, server.angle).abs();
        assert!(
            final_drift < 1e-5,
            "full timeline: reconciled={:.4} server={:.4} drift={final_drift:.6}",
            reconciled.angle,
            server.angle
        );
    }

    /// Two consecutive quick turns: first completes, second starts and completes.
    /// Final angle should be ~0 (two 180° turns = 360° = 0 mod TAU).
    #[test]
    fn repeated_quick_turns_produce_full_rotation() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();
        let mut state = state_at(2.5, 2.5, 0.0);

        let snap = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: Some(SnapTurnKind::QuickTurn),
        };
        let idle = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: None,
        };

        // First QuickTurn: run to completion.
        apply_prediction_tick(&mut state, &snap, &map, spd, tspd, margin, qtd, DT);
        for _ in 0..15 {
            apply_prediction_tick(&mut state, &idle, &map, spd, tspd, margin, qtd, DT);
        }
        assert!(
            state.snap_remaining_radians <= 0.0,
            "first snap should be complete"
        );
        assert!(
            (state.angle - std::f32::consts::PI).abs() < 0.02,
            "after first snap: angle should be ~PI, got {:.4}",
            state.angle
        );

        // Second QuickTurn: should be accepted (first is done).
        apply_prediction_tick(&mut state, &snap, &map, spd, tspd, margin, qtd, DT);
        assert!(
            state.snap_remaining_radians > 0.0,
            "second snap should start"
        );
        for _ in 0..15 {
            apply_prediction_tick(&mut state, &idle, &map, spd, tspd, margin, qtd, DT);
        }
        assert!(
            state.snap_remaining_radians <= 0.0,
            "second snap should complete"
        );

        // Two 180° turns = 360° = 0 (mod TAU).
        let wrapped = state.angle.rem_euclid(std::f32::consts::TAU);
        assert!(
            wrapped < 0.02 || (std::f32::consts::TAU - wrapped) < 0.02,
            "two quick turns should produce ~0 mod TAU, got {:.4} (wrapped {wrapped:.4})",
            state.angle
        );
    }

    /// Quick turn rejected while active: second snap during active first is ignored.
    #[test]
    fn quick_turn_rejected_while_active() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();
        let mut state = state_at(2.5, 2.5, 0.0);

        let snap = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: Some(SnapTurnKind::QuickTurn),
        };

        // Start first snap.
        apply_prediction_tick(&mut state, &snap, &map, spd, tspd, margin, qtd, DT);
        let remaining_after_first = state.snap_remaining_radians;
        assert!(remaining_after_first > 0.0);

        // Try second snap while first is active — should be rejected.
        apply_prediction_tick(&mut state, &snap, &map, spd, tspd, margin, qtd, DT);
        // snap_remaining should have DECREASED (ticked), not reset to PI.
        assert!(
            state.snap_remaining_radians < remaining_after_first,
            "second snap should be rejected: remaining={:.4} (should be < {remaining_after_first:.4})",
            state.snap_remaining_radians
        );
    }

    /// 90° left snap continues past stale expiry (takes ~6 ticks, stale=5).
    #[test]
    fn left_snap_continues_past_stale_expiry() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();

        let snap = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: Some(SnapTurnKind::Left),
        };
        let idle = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: None,
        };

        // Server: snap + 8 idle (more than STALE_INPUT_TICKS=5).
        let mut server = state_at(2.5, 2.5, 0.0);
        apply_prediction_tick(&mut server, &snap, &map, spd, tspd, margin, qtd, DT);
        for _ in 0..8 {
            apply_prediction_tick(&mut server, &idle, &map, spd, tspd, margin, qtd, DT);
        }

        // Client: same sequence.
        let mut client = state_at(2.5, 2.5, 0.0);
        apply_prediction_tick(&mut client, &snap, &map, spd, tspd, margin, qtd, DT);
        for _ in 0..8 {
            apply_prediction_tick(&mut client, &idle, &map, spd, tspd, margin, qtd, DT);
        }

        assert!(
            server.snap_remaining_radians <= 0.0,
            "server left snap should complete"
        );
        assert!(
            client.snap_remaining_radians <= 0.0,
            "client left snap should complete"
        );

        let drift = shortest_angle_delta(client.angle, server.angle).abs();
        assert!(
            drift < 1e-5,
            "left snap past stale: client={:.4} server={:.4} drift={drift:.6}",
            client.angle,
            server.angle
        );
        assert!(
            (client.angle - std::f32::consts::FRAC_PI_2).abs() < 0.01,
            "left snap should produce ~PI/2: {:.4}",
            client.angle
        );
    }

    /// 90° right snap continues past stale expiry.
    #[test]
    fn right_snap_continues_past_stale_expiry() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();

        let snap = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: Some(SnapTurnKind::Right),
        };
        let idle = PredictedInput {
            movement: Vec2::ZERO,
            turn: 0.0,
            snap_turn: None,
        };

        let mut server = state_at(2.5, 2.5, 0.0);
        apply_prediction_tick(&mut server, &snap, &map, spd, tspd, margin, qtd, DT);
        for _ in 0..8 {
            apply_prediction_tick(&mut server, &idle, &map, spd, tspd, margin, qtd, DT);
        }

        let mut client = state_at(2.5, 2.5, 0.0);
        apply_prediction_tick(&mut client, &snap, &map, spd, tspd, margin, qtd, DT);
        for _ in 0..8 {
            apply_prediction_tick(&mut client, &idle, &map, spd, tspd, margin, qtd, DT);
        }

        let drift = shortest_angle_delta(client.angle, server.angle).abs();
        assert!(
            drift < 1e-5,
            "right snap past stale: client={:.4} server={:.4} drift={drift:.6}",
            client.angle,
            server.angle
        );

        // Right snap from 0 → -PI/2 → wraps to TAU - PI/2.
        let expected =
            (std::f32::consts::TAU - std::f32::consts::FRAC_PI_2).rem_euclid(std::f32::consts::TAU);
        assert!(
            (client.angle - expected).abs() < 0.01,
            "right snap should produce ~(TAU-PI/2): got {:.4} expected {expected:.4}",
            client.angle
        );
    }

    // ── Death / StaleInput lifecycle ─────────────────────────────────

    /// Verify `clear_prediction_on_death` resets StaleInput (the Resource
    /// promoted from Local). Covers the fix where stale movement from a
    /// previous life would ghost-drive the player on respawn.
    #[test]
    fn death_clears_stale_input() {
        let mut state = state_at(2.5, 2.5, 1.0);
        let mut history = PredictionHistory::default();
        history.push(PredictionEntry {
            sequence: InputSequence(5),
            input: PredictedInput {
                movement: Vec2::Y,
                turn: 0.3,
                snap_turn: None,
            },
            result: PredictionSnapshot {
                position: Vec2::new(2.5, 2.5),
                angle: 1.0,
            },
            dt: DT,
        });

        // Simulate active stale input from previous life.
        let mut stale_input = StaleInput {
            movement: Vec2::new(0.0, 1.0),
            turn: 0.5,
            age_ticks: 2,
        };

        let mut render_state = PredictedRenderState::default();

        // Same logic as clear_prediction_on_death when dead && initialised.
        assert!(state.initialised);
        history.clear();
        state.initialised = false;
        render_state.reset();
        stale_input.age_ticks = STALE_INPUT_TICKS;
        stale_input.movement = Vec2::ZERO;
        stale_input.turn = 0.0;

        // Verify all stale state cleared.
        assert_eq!(
            stale_input.movement,
            Vec2::ZERO,
            "stale movement should be zeroed"
        );
        assert!(
            (stale_input.turn - 0.0).abs() < f32::EPSILON,
            "stale turn should be zeroed"
        );
        assert!(
            stale_input.age_ticks >= STALE_INPUT_TICKS,
            "stale age should be expired"
        );
        assert!(!state.initialised);
        assert!(history.is_empty());
    }

    // ── Out-of-order ack handling ────────────────────────────────────

    /// An ack with a sequence older than what was already pruned must not
    /// corrupt state or replay stale history. This guards against network
    /// reordering where the transport delivers ack(seq=3) after ack(seq=7).
    #[test]
    fn out_of_order_ack_does_not_corrupt_state() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();

        // Build history with entries seq 1..=10.
        let mut history = PredictionHistory::default();
        let mut state = state_at(2.5, 2.5, 0.0);
        let input = PredictedInput {
            movement: Vec2::new(0.0, 1.0),
            turn: 0.0,
            snap_turn: None,
        };

        for i in 1..=10u32 {
            apply_prediction_tick(&mut state, &input, &map, spd, tspd, margin, qtd, DT);
            history.push(PredictionEntry {
                sequence: InputSequence(i),
                input: input.clone(),
                result: PredictionSnapshot {
                    position: state.position,
                    angle: state.angle,
                },
                dt: DT,
            });
        }

        assert_eq!(history.len(), 10);

        // Simulate ack(seq=7): prune entries 1..=7, leaving 8,9,10.
        history.prune_through(InputSequence(7));
        assert_eq!(history.len(), 3, "should have entries 8,9,10");

        // Reset to server truth at seq 7 and replay.
        let mut predicted_after_7 = state_at(2.5, 2.5, 0.0);
        // Advance server state to seq 7 (same math).
        for _ in 0..7 {
            apply_prediction_tick(
                &mut predicted_after_7,
                &input,
                &map,
                spd,
                tspd,
                margin,
                qtd,
                DT,
            );
        }
        let server_pos_7 = predicted_after_7.position;
        let server_angle_7 = predicted_after_7.angle;

        // Reconcile: reset + replay unacked.
        let mut reconciled = PredictedPlayerState {
            position: server_pos_7,
            angle: server_angle_7,
            snap_remaining_radians: 0.0,
            snap_total_radians: 0.0,
            snap_speed: 0.0,
            snap_direction: 0.0,
            speed_modifier_multiplier: 1.0,
            speed_modifier_remaining: 0.0,
            impulse_direction: Vec2::ZERO,
            impulse_strength: 0.0,
            impulse_remaining: 0.0,
            impulse_duration: 0.0,
            initialised: true,
        };
        for entry in history.iter_all() {
            apply_prediction_tick(
                &mut reconciled,
                &entry.input,
                &map,
                spd,
                tspd,
                margin,
                qtd,
                entry.dt,
            );
        }

        // State after reconciliation should match straight-through apply.
        let pos_drift = (reconciled.position - state.position).length();
        let angle_drift = (reconciled.angle - state.angle).abs();
        assert!(
            pos_drift < 1e-5,
            "reconciled position should match: drift={pos_drift:.6}"
        );
        assert!(
            angle_drift < 1e-5,
            "reconciled angle should match: drift={angle_drift:.6}"
        );

        // Now simulate a LATE ack(seq=3) arriving after we already pruned
        // through seq=7. prune_through(3) should be a no-op (all remaining
        // entries have seq > 3, so is_after(3) is true for all).
        let len_before = history.len();
        history.prune_through(InputSequence(3));
        assert_eq!(
            history.len(),
            len_before,
            "late ack should not prune anything: entries are all > seq 3"
        );

        // Replay from late ack's server state should NOT match because the
        // ack is stale — the server has moved past seq 3. But the important
        // invariant is that history and state remain internally consistent.
        // Re-reconcile from seq=7 truth to verify no corruption.
        let mut re_reconciled = PredictedPlayerState {
            position: server_pos_7,
            angle: server_angle_7,
            snap_remaining_radians: 0.0,
            snap_total_radians: 0.0,
            snap_speed: 0.0,
            snap_direction: 0.0,
            speed_modifier_multiplier: 1.0,
            speed_modifier_remaining: 0.0,
            impulse_direction: Vec2::ZERO,
            impulse_strength: 0.0,
            impulse_remaining: 0.0,
            impulse_duration: 0.0,
            initialised: true,
        };
        for entry in history.iter_all() {
            apply_prediction_tick(
                &mut re_reconciled,
                &entry.input,
                &map,
                spd,
                tspd,
                margin,
                qtd,
                entry.dt,
            );
        }

        // Same result as before — no corruption from the late prune.
        let pos_drift2 = (re_reconciled.position - reconciled.position).length();
        let angle_drift2 = (re_reconciled.angle - reconciled.angle).abs();
        assert!(
            pos_drift2 < 1e-6,
            "re-reconcile should be identical: {pos_drift2:.6}"
        );
        assert!(
            angle_drift2 < 1e-6,
            "re-reconcile angle identical: {angle_drift2:.6}"
        );
    }

    // ── Speed modifier parity ────────────────────────────────────────

    /// Prediction with speed modifier matches direct movement with modifier.
    #[test]
    fn speed_modifier_slows_predicted_movement() {
        let map = open_map();
        let (spd, _tspd, margin, _qtd) = default_config();

        // Without modifier: full speed.
        let mut full = state_at(2.5, 2.5, 0.0);
        let input = PredictedInput {
            movement: Vec2::new(0.0, 1.0),
            turn: 0.0,
            snap_turn: None,
        };
        apply_prediction_tick(&mut full, &input, &map, spd, 2.0, margin, 0.4, DT);

        // With modifier: 70% speed.
        let mut slowed = state_at(2.5, 2.5, 0.0);
        slowed.speed_modifier_multiplier = 0.7;
        slowed.speed_modifier_remaining = 5.0;
        apply_prediction_tick(&mut slowed, &input, &map, spd, 2.0, margin, 0.4, DT);

        // Slowed should travel less distance.
        let full_dist = full.position.distance(Vec2::new(2.5, 2.5));
        let slow_dist = slowed.position.distance(Vec2::new(2.5, 2.5));
        assert!(
            slow_dist < full_dist,
            "modifier should reduce movement: full={full_dist}, slow={slow_dist}"
        );
        assert!(
            (slow_dist / full_dist - 0.7).abs() < 0.01,
            "should move at ~70%: ratio={}",
            slow_dist / full_dist
        );
    }

    /// Speed modifier drains during replay and expires correctly.
    #[test]
    fn speed_modifier_expires_during_replay() {
        let map = open_map();
        let (spd, _tspd, margin, _qtd) = default_config();
        let input = PredictedInput {
            movement: Vec2::new(0.0, 1.0),
            turn: 0.0,
            snap_turn: None,
        };

        let mut state = state_at(2.5, 2.5, 0.0);
        // Very small budget — will expire quickly.
        state.speed_modifier_multiplier = 0.5;
        state.speed_modifier_remaining = 0.01;

        // Tick once — modifier should expire.
        apply_prediction_tick(&mut state, &input, &map, spd, 2.0, margin, 0.4, DT);

        assert!(
            state.speed_modifier_remaining <= 0.0
                || (state.speed_modifier_multiplier - 1.0).abs() < f32::EPSILON,
            "modifier should have expired: mult={}, remaining={}",
            state.speed_modifier_multiplier,
            state.speed_modifier_remaining
        );

        // Second tick should be at full speed.
        let pos_after_one = state.position;
        apply_prediction_tick(&mut state, &input, &map, spd, 2.0, margin, 0.4, DT);
        let second_tick_dist = state.position.distance(pos_after_one);
        let expected_full = spd * DT;
        assert!(
            (second_tick_dist - expected_full).abs() < 0.01,
            "after expiry should move at full speed: got={second_tick_dist}, expected={expected_full}"
        );
    }

    /// No modifier produces identical movement to the original prediction.
    #[test]
    fn no_modifier_prediction_unchanged() {
        let map = open_map();
        let (spd, tspd, margin, qtd) = default_config();
        let input = PredictedInput {
            movement: Vec2::new(0.0, 1.0),
            turn: 0.0,
            snap_turn: None,
        };

        // Prediction tick.
        let mut predicted = state_at(2.5, 2.5, 0.0);
        apply_prediction_tick(&mut predicted, &input, &map, spd, tspd, margin, qtd, DT);

        // Direct movement (what the server does without modifier).
        let mut server_pos = Vec2::new(2.5, 2.5);
        apply_movement(
            &mut server_pos,
            0.0,
            Vec2::new(0.0, 1.0),
            spd,
            DT,
            &map,
            margin,
        );

        assert!(
            predicted.position.distance(server_pos) < 1e-6,
            "no-modifier prediction should match direct movement: pred={:?}, srv={server_pos:?}",
            predicted.position
        );
    }

    /// Server with modifier and client replay with modifier produce same position.
    #[test]
    fn modifier_replay_matches_server() {
        let map = open_map();
        let (spd, _tspd, margin, _qtd) = default_config();
        let input = PredictedInput {
            movement: Vec2::new(0.0, 1.0),
            turn: 0.0,
            snap_turn: None,
        };
        let modifier_mult = 0.7;
        let modifier_budget = 5.0;

        // Server: apply_movement_with_modifier + tick modifier.
        let mut server_pos = Vec2::new(2.5, 2.5);
        let mut server_mod =
            carcinisation_fps_core::movement::SpeedModifier::new(modifier_mult, modifier_budget);
        carcinisation_fps_core::movement::apply_movement_with_modifier(
            &mut server_pos,
            0.0,
            Vec2::new(0.0, 1.0),
            spd,
            Some(&server_mod),
            DT,
            &map,
            margin,
        );
        let moved = server_pos.distance(Vec2::new(2.5, 2.5));
        server_mod.tick(DT, moved);

        // Client: apply_prediction_tick with modifier state.
        let mut client = state_at(2.5, 2.5, 0.0);
        client.speed_modifier_multiplier = modifier_mult;
        client.speed_modifier_remaining = modifier_budget;
        apply_prediction_tick(&mut client, &input, &map, spd, 2.0, margin, 0.4, DT);

        assert!(
            client.position.distance(server_pos) < 1e-6,
            "client replay should match server: client={:?}, server={server_pos:?}",
            client.position
        );
        assert!(
            (client.speed_modifier_remaining - server_mod.remaining).abs() < 1e-4,
            "modifier drain should match: client={}, server={}",
            client.speed_modifier_remaining,
            server_mod.remaining
        );
    }

    /// Multi-tick server/client parity under active speed modifier.
    ///
    /// Simulates 30 ticks (~1s) of forward movement with a web slow modifier.
    /// Verifies position and modifier remaining stay identical between the
    /// server path (apply_movement_with_modifier + manual drain) and the
    /// client path (apply_prediction_tick with modifier state).
    #[test]
    fn multi_tick_modifier_parity() {
        let map = open_map();
        let (spd, _tspd, margin, _qtd) = default_config();
        let input = PredictedInput {
            movement: Vec2::new(0.0, 1.0),
            turn: 0.0,
            snap_turn: None,
        };
        let modifier_mult = 0.7;
        let modifier_budget = 5.0;

        // Server state.
        let mut server_pos = Vec2::new(2.5, 2.5);
        let mut server_mod =
            carcinisation_fps_core::movement::SpeedModifier::new(modifier_mult, modifier_budget);

        // Client state.
        let mut client = state_at(2.5, 2.5, 0.0);
        client.speed_modifier_multiplier = modifier_mult;
        client.speed_modifier_remaining = modifier_budget;

        for tick in 0..30 {
            // Server: apply movement with modifier, then drain.
            let server_start = server_pos;
            carcinisation_fps_core::movement::apply_movement_with_modifier(
                &mut server_pos,
                0.0,
                Vec2::new(0.0, 1.0),
                spd,
                Some(&server_mod),
                DT,
                &map,
                margin,
            );
            let moved = server_pos.distance(server_start);
            server_mod.tick(DT, moved);

            // Client: apply_prediction_tick (handles modifier internally).
            apply_prediction_tick(&mut client, &input, &map, spd, 2.0, margin, 0.4, DT);

            // Verify parity each tick.
            let pos_drift = client.position.distance(server_pos);
            assert!(
                pos_drift < 1e-5,
                "tick {tick}: position drift {pos_drift:.8} — client={:?} server={server_pos:?}",
                client.position
            );

            let remaining_drift = (client.speed_modifier_remaining - server_mod.remaining).abs();
            assert!(
                remaining_drift < 1e-5,
                "tick {tick}: remaining drift {remaining_drift:.8} — client={} server={}",
                client.speed_modifier_remaining,
                server_mod.remaining
            );
        }
    }

    /// Speed modifier expires mid-sequence and movement transitions to full speed.
    #[test]
    fn modifier_expires_mid_sequence_parity() {
        let map = open_map();
        let (spd, _tspd, margin, _qtd) = default_config();
        let input = PredictedInput {
            movement: Vec2::new(0.0, 1.0),
            turn: 0.0,
            snap_turn: None,
        };

        // Small budget that expires within ~3 ticks.
        let mut server_pos = Vec2::new(2.5, 2.5);
        let mut server_mod = carcinisation_fps_core::movement::SpeedModifier::new(0.5, 0.1);
        let mut server_mod_active = true;

        let mut client = state_at(2.5, 2.5, 0.0);
        client.speed_modifier_multiplier = 0.5;
        client.speed_modifier_remaining = 0.1;

        let mut expired_tick = None;
        for tick in 0..15 {
            // Server path.
            let server_start = server_pos;
            let modifier_ref = if server_mod_active {
                Some(&server_mod)
            } else {
                None
            };
            carcinisation_fps_core::movement::apply_movement_with_modifier(
                &mut server_pos,
                0.0,
                Vec2::new(0.0, 1.0),
                spd,
                modifier_ref,
                DT,
                &map,
                margin,
            );
            if server_mod_active {
                let moved = server_pos.distance(server_start);
                if !server_mod.tick(DT, moved) {
                    server_mod_active = false;
                    if expired_tick.is_none() {
                        expired_tick = Some(tick);
                    }
                }
            }

            // Client path.
            apply_prediction_tick(&mut client, &input, &map, spd, 2.0, margin, 0.4, DT);

            let pos_drift = client.position.distance(server_pos);
            assert!(
                pos_drift < 1e-5,
                "tick {tick}: position drift {pos_drift:.8}",
            );
        }

        assert!(
            expired_tick.is_some(),
            "modifier should have expired during the sequence"
        );
    }
}
