# Networking Architecture: Timing Audit & Future Expansion

Investigation of long-term timing correctness and expansion pressure points in the current prediction/reconciliation system.

## 1. Current Semantics Audit

### What InputSequence represents

`InputSequence` is a client-local wrapping u32 counter incremented **once per network send** in `collect_and_send_intent` (Update frame, variable rate). It is NOT incremented per FixedUpdate tick. The send policy is: immediate on action change, periodic ~30Hz for held state, skip idle.

This means InputSequence is an **input ordering token**, not a simulation tick identifier. Two consecutive sequences may correspond to the same FixedUpdate tick (if Update runs faster) or skip FixedUpdate ticks entirely (if the client was idle and skipped sends).

### What InputAck.last_processed_sequence means

"The server has applied input up to and including sequence N. Here is the authoritative state AFTER that application."

The position/angle in the ack are **post-application state**. The client prunes history entries with sequence <= N, then replays entries with sequence > N from the acked state.

### What InputAck.server_tick means

Currently: nothing operational. The field is populated by the server (`tick_counter.0`) and transmitted, but the client never reads it. It exists for diagnostic/future use only.

### Where stale-input semantics blur the sequence-timeline relationship

Stale ticks are the primary semantic gap. When `PendingInput` is empty:

- The server continues applying the last buffered intent for up to 5 ticks
- The client mirrors this via `StaleInput` with matching age semantics
- **Neither side creates history entries for stale ticks**
- The stale movement is "baked into" the ack position implicitly

This means the ack position represents: `state_after(input_N) + stale_ticks_of_movement`. The client cannot distinguish "the server applied input N and then sat idle" from "the server applied input N and then applied 3 stale ticks." Both produce the same ack with `last_processed_sequence=N`.

**This is correct today** because the client mirrors the stale behavior exactly. But it means the ack is not a pure "state after input N" snapshot — it's "state at the time the ack was emitted, which happens to be after input N plus some implicit stale advancement."

### Ambiguous cases

The sequence-timeline relationship is unambiguous for the current system because:

1. Only one input stream exists (single predicted player)
2. Stale prediction is deterministically mirrored
3. No gameplay systems depend on "which server tick did this happen on"
4. No lag compensation requires rewinding to a specific server tick

The ambiguity would surface if any future system needed to answer: "what was the server's world state at tick T?" Currently, the system can only answer: "what was the local player's state after input N?"

## 2. Tick-Timeline Model Evaluation

### What richer tick semantics would look like

```
ClientIntent {
    sequence: InputSequence,     // existing: input ordering
    client_tick: Tick,           // NEW: client's FixedUpdate counter
}

InputAck {
    last_processed_sequence: InputSequence,  // existing
    server_tick: Tick,                       // existing but unused
    applied_at_tick: Tick,                   // NEW: server tick when input was applied
    stale_age: u32,                         // NEW: how many stale ticks elapsed since input
}
```

### What problems this would solve

| Problem | Current | With tick semantics |
|---|---|---|
| Debugging stale drift | Requires inference from correction magnitude | Ack carries explicit stale_age |
| Replay determinism | Relies on identical stale mirroring | Server tick provides authoritative timeline |
| Lag compensation | Not possible (no server tick reference) | `applied_at_tick` enables server rewind |
| Client clock drift | Undetectable | `server_tick` vs `client_tick` delta reveals drift |
| Spectator mode | Would need to reconstruct timeline | Server ticks provide ordering |

### Whether it simplifies reconciliation reasoning

Marginally. The current reconciliation is sequence-driven and correct. Adding tick semantics would not change the core loop (prune, reset, replay). It would primarily help with:

- **Diagnostics**: knowing exactly how many server ticks elapsed between acks
- **Drift detection**: comparing client FixedUpdate count vs server tick count
- **Future lag compensation**: "rewind server to tick T, check hitscan"

### Whether it improves replay determinism

Yes, but only for external replay recording. The current system achieves deterministic replays via `SimHash` (FNV-1a hash of all replicated state per tick). Adding explicit tick markers would make replay files self-describing — each frame would carry its authoritative tick number rather than relying on implicit counting.

### Recommendation

**Do not add client_tick or stale_age now.** The current system is correct and the added fields would be dead code. **Do start using server_tick on the client** for BRP diagnostics (display server tick in PredictionDiagnostics). This is zero-cost and provides observability.

When lag compensation becomes needed (predicted hitscan, killcams), add `applied_at_tick` to InputAck at that time. The migration is additive — no existing fields change.

## 3. Industry Model Comparison

### QuakeWorld (1996)

**Movement prediction:** Client runs `SV_RunClientCommand` locally. Server runs same code. Client corrects on mismatch. *We match this exactly.*

**Stale input:** QuakeWorld did not have explicit stale-tick prediction. Clients sent inputs every frame; the server applied whatever it had. Packet loss meant the server applied nothing (no stale buffer). *We exceed QuakeWorld here* — our stale buffer tolerates one dropped packet without correction.

**Command buffering:** QuakeWorld sent one command per client frame. No explicit command buffer on the server. *We match this* — PlayerIntentBuffer stores the latest intent, not a queue.

**Correction timing:** QuakeWorld applied corrections immediately (snap). No smoothing. *We match this* — correction smoothing was removed after debugging temporal desync.

### Source Engine (2001)

**Movement prediction:** Same as QuakeWorld but with `cl_predict` cvar and explicit `CPrediction::RunCommand`. *We match the core loop.*

**Stale input:** Source uses `CUserCmd` buffers with explicit timestamps. The server processes commands in order, applying the latest for each tick. *Our stale buffer is simpler but serves the same purpose.*

**Command buffering:** Source sends redundant commands (current + N previous) to handle packet loss. Server processes the first unseen command. *We do not do command redundancy.* This is a potential improvement for lossy networks but adds complexity.

**Interpolation:** Source uses `cl_interp` (100ms default) for remote entities. Entities render in the past relative to the server. *We do adaptive interpolation* — interval adapts to actual elapsed time between updates rather than using a fixed delay.

**Lag compensation:** Source rewinds server state to the client's render time for hitscan verification. This requires explicit tick timelines. *We do not have this.* It would require `server_tick` consumption on the client.

### Overwatch (2016)

**Movement prediction:** Same QuakeWorld model. Deterministic prediction + server reconciliation. *We match this.*

**Ability prediction:** Overwatch predicts ability activation (cooldowns, state changes) locally. Server confirms or rejects. *We do not predict abilities* — weapon switch is server-authoritative with visual-only client feedback.

**Favor-the-shooter:** Overwatch uses server-side lag compensation with a 250ms window. Client sends its render timestamp; server rewinds to that time for hit verification. *We do not have this.* Would require the tick-timeline additions from section 2.

**Correction smoothing:** Overwatch blends corrections over multiple frames. *We removed smoothing* due to temporal desync. Overwatch solves this by computing corrections at consistent simulation boundaries, not in async observers.

### Glenn Fiedler State Synchronization (2014-2018)

**State sync vs input prediction:** Fiedler advocates sending full state snapshots at high frequency, with client-side interpolation between snapshots. This is fundamentally different from input prediction — it trades bandwidth for simplicity. *Not applicable to our architecture* (we use input prediction, not state sync for the local player).

**Jitter buffer:** Fiedler recommends buffering incoming state for one or more network ticks to smooth jitter. *We do not have a jitter buffer.* Our interpolation adapts its interval to actual update timing, which achieves a similar effect without explicit buffering.

**Input redundancy:** Fiedler recommends sending the last N inputs in each packet so the server can recover from packet loss without waiting. *We do not do this.* Our stale buffer provides partial tolerance (5 ticks / ~167ms) but full redundancy would be more robust.

### Summary: Where we sit

| Technique | QuakeWorld | Source | Overwatch | Us |
|---|---|---|---|---|
| Movement prediction | Yes | Yes | Yes | Yes |
| Stale input buffer | No | Implicit | Yes | Yes (5 ticks) |
| Command redundancy | No | Yes | Yes | No |
| Snap correction | Yes | Optional | No (smoothed) | Yes |
| Correction smoothing | No | Optional | Yes | Removed (temporal desync) |
| Lag compensation | No | Yes | Yes | No |
| Ability prediction | N/A | No | Yes | No |
| Adaptive interpolation | No | Fixed delay | Adaptive | Adaptive |
| Explicit tick timeline | No | Yes | Yes | Partial (sent, unused) |

## 4. Future Expansion Pressure

### Weapon prediction

**Pressure: Low.** Current weapons (pistol hitscan, flamethrower) are server-authoritative. The client sees the fire visual immediately but damage is server-confirmed. Predicting weapon state (cooldowns, ammo) would require adding weapon state to `PredictedPlayerState` and replaying weapon logic during reconciliation. Not needed unless weapon feedback latency becomes a problem.

### Predicted projectiles

**Pressure: Medium.** If the game adds client-owned projectiles (e.g., grenades), they would need local spawning with server confirmation. This is fundamentally harder than movement prediction because projectiles interact with the world. Would require entity-level prediction, not just player-state prediction. The singleton Resource architecture would not support this — it would need per-entity prediction components.

**Recommendation:** Cross this bridge when a projectile weapon is designed. Do not pre-build the infrastructure.

### Server rewind lag compensation

**Pressure: Medium-High for competitive play. Low for current scope.**

Lag compensation requires: (a) the server to maintain a history of world snapshots keyed by tick, (b) the client to report which server tick it was rendering when it fired, (c) the server to rewind to that tick and verify the hit.

This needs the tick-timeline additions from section 2 (`server_tick` consumed by client, `applied_at_tick` in ack). The server would also need a `WorldSnapshotHistory` ring buffer.

**Recommendation:** Not needed until competitive hitscan fairness matters. The current "server processes fire at current tick" is sufficient for PvE and casual PvP.

### Replay recording

**Pressure: Low.** Current `SimHash` system verifies determinism but doesn't record replays. A replay system would record `(tick, player_id, ClientIntent)` tuples and replay them through a headless server. This is additive and doesn't require architectural changes. Would benefit from explicit tick numbers in the recording format.

### Spectator mode

**Pressure: Low.** Spectators receive replicated `NetPlayer`/`NetEnemy` state and use remote interpolation (already implemented). No prediction needed. Would benefit from server_tick for timeline scrubbing but doesn't require it for basic spectating.

### Demo playback / killcams

**Pressure: Medium.** Killcams require rewinding the world to the moment of death and replaying from the killer's perspective. This needs: (a) server-side world snapshot history, (b) explicit tick timeline, (c) ability to render from a non-local player's viewpoint. This is the strongest argument for the tick-timeline model but is a significant feature, not a prediction architecture change.

### Local listen-server mode

**Pressure: Low.** A listen server (client + server in one process) would bypass the network layer but still use the same prediction/reconciliation logic. The `FpsAuthorityMode` already distinguishes `LocalAuthority` from `RemoteClient`. A listen server would use `RemoteClient` mode with zero-latency loopback transport. No architectural changes needed.

### Splitscreen / local co-op

**Pressure: High for the prediction architecture.** Currently all prediction state is singleton Resources. Two local players would need two `PredictedPlayerState`s, two `PredictionHistory`s, two `PendingInput`s, two `StaleInput`s. This requires converting Resources to entity-associated Components keyed by `PlayerId`.

**Recommendation:** Document the migration path but do not execute it until splitscreen is designed. The migration is mechanical (Resource → Component, system params add `Query<>`) but touches every prediction system.

### Higher tick rates

**Pressure: None.** `TickConfig` already parameterizes the tick rate. Changing from 30Hz to 60Hz would halve `dt`, double history buffer consumption, and require adjusting `STALE_INPUT_TICKS` proportionally. All math is dt-aware. No architectural changes needed.

### Variable tick rates

**Pressure: Low.** Bevy's `FixedUpdate` runs at a fixed rate. Variable tick rates would require replacing `FixedUpdate` with manual timestep logic. The prediction system stores `dt` per history entry, so replay would handle variable dt correctly. The stale-tick system would need to track elapsed time rather than tick count. Not recommended — fixed tick rates are simpler and sufficient.

## 5. Clocking / Time Model

### Are fixed-tick semantics sufficient?

Yes. The 30Hz FixedUpdate provides deterministic dt (1/30 = 0.0333s) on both client and server. Prediction entries store their dt, so replay is exact even if the tick rate changes between versions. No variable-timestep complications.

### Should prediction become "predict until server_tick + offset"?

No. The current model predicts until "all pending inputs are applied." This is simpler and more correct because:

1. The client doesn't know the server's current tick (no clock sync)
2. Input-driven prediction naturally handles variable network latency
3. "Predict until tick T" would require estimating server tick, which introduces clock drift errors

The Source Engine uses tick-based prediction (`m_nServerTick + prediction_offset`) but also has explicit clock synchronization. Without clock sync, input-driven prediction is more robust.

### Is client/server drift estimation needed?

Not for prediction correctness. The reconciliation loop corrects any accumulated drift on every ack. Drift estimation would be useful for:

- Displaying latency in the HUD (already done via `renet2`'s RTT)
- Adjusting interpolation delay for remote entities
- Detecting clock desync in debug builds

**Recommendation:** Log `server_tick` from acks into `PredictionDiagnostics` for observability. Do not build a clock sync protocol.

### Should RTT estimation affect interpolation delay?

Currently, remote entity interpolation uses adaptive intervals (time between consecutive updates). This implicitly handles RTT variation. Explicit RTT-based interpolation delay (Source's `cl_interp`) would provide smoother results at the cost of added visual latency.

**Recommendation:** Not needed now. If remote entity movement looks choppy under high jitter, add a configurable minimum interpolation delay (e.g., `max(adaptive_interval, rtt * 0.5)`). This is a tuning parameter, not an architectural change.

### Lamport clocks / vector clocks

**Not appropriate.** These are tools for establishing causal ordering in distributed systems where no shared timeline exists (e.g., distributed databases, chat systems). They answer: "did event A happen before event B across independent nodes?"

Our system has a fundamentally different structure:

- **Causal ordering** (Lamport): "A caused B" across nodes with no shared clock
- **Simulation timeline** (game server): "A happened at tick T, B at tick T+1" with a single authoritative clock

The server IS the authoritative clock. There is no distributed consensus problem. Client prediction is speculative, not authoritative. When the server says "at tick T, you were at position P," there is no ambiguity or need for causal ordering — the server's word is final.

Vector clocks would add overhead with zero benefit. The server's `TickCounter` already provides a total ordering of all simulation events.

## 6. Known Remaining Visual Issues

### Potential sources of residual "snappy" feeling

**Render quantization:** The camera reads `PredictedRenderState` which interpolates between 30Hz snapshots. At 60fps, there are exactly 2 render frames per prediction tick. The interpolation alpha jumps 0.0 → 0.5 → 1.0. At 144fps, it's smoother (4-5 intermediate values). Low-framerate displays will feel more quantized. Not a bug — it's inherent to 30Hz simulation with linear interpolation.

**Fixed-to-render cadence mismatch:** Bevy's FixedUpdate can accumulate multiple ticks per frame if the frame took too long. When this happens, `PredictedRenderState.on_fixed_tick()` is called multiple times in one frame, each shifting prev→current. The interpolation then starts from the second-to-last tick, not the original prev. This could cause micro-jumps if FixedUpdate runs 2+ ticks in a single frame. Mitigation: the 30Hz tick rate makes double-ticks rare at 60fps+.

**Reconciliation timing:** `handle_input_ack` runs in PreUpdate (observer). If a correction occurs, `on_reconciliation()` updates `PredictedRenderState.current` immediately. But the previous `prev` value is from the last FixedUpdate, not from the corrected state. The interpolation alpha might produce a position that's between the old-prev and new-current, which is geometrically reasonable but not physically simulated. This is a minor visual discontinuity, not a correctness issue.

**Ack timing jitter:** Acks arrive over the network with variable latency. Two acks might arrive in the same frame or be separated by many frames. The reconciliation observer handles each independently, but clustered acks cause multiple prune-reset-replay cycles per frame. The last one wins, so this is correct but wasteful. A debounce (process only the latest ack per frame) would be a minor optimization.

**Camera interpolation model:** The camera uses linear interpolation between prediction snapshots. Source Engine uses Hermite (cubic) interpolation for smoother curves. Linear interpolation has discontinuous velocity at snapshot boundaries (the derivative jumps). This is the most likely source of residual "snappy" feeling on direction changes.

**Bevy scheduling order:** The prediction systems run in this order per frame:
1. PreUpdate: `handle_input_ack` (observer, if ack arrived)
2. FixedUpdate/MovementSet: `apply_predicted_movement`
3. Update: `tick_predicted_render`, `sync_camera_from_net_player`

If an ack arrives in the same frame as a FixedUpdate tick, the reconciliation happens BEFORE the new prediction tick. This is correct — the new tick applies on top of the corrected state. No ordering issue.

### Recommendation

The most impactful visual improvement would be upgrading from linear to cubic (Hermite) interpolation in `PredictedRenderState`. This smooths velocity discontinuities at snapshot boundaries without changing the prediction architecture. However, it requires storing velocity (or computing it from consecutive snapshots), which adds state to the render interpolation layer.

## 7. Recommendations Summary

### Leave alone

- **Reconciliation loop:** Prune-reset-replay is correct and battle-tested. Do not change.
- **Stale-tick prediction:** The mirroring approach is correct and well-tested. Do not add complexity.
- **Sequence-driven history:** InputSequence as the history key is correct for single-player prediction. Do not switch to tick-driven history.
- **Snap turn ack bypass:** The dedup bypass during snap turns solves a real problem cleanly. Do not revert.
- **Singleton Resources:** Correct for single-player prediction. Do not convert to Components until splitscreen is designed.

### Future-safe additions worth doing now

1. **Log server_tick into PredictionDiagnostics.** Zero-cost observability. Store `last_server_tick` in `PredictionDiagnostics` from each ack. Visible via BRP. Helps debug tick-rate mismatches and ack timing.

2. **Add ack_count_this_frame to diagnostics.** Count how many acks arrive per frame. If consistently > 1, the server is sending faster than the client processes. If consistently 0, acks are stalling.

3. **Add stale_ticks_this_session counter.** Track how many stale ticks the client has applied since connection. High counts indicate packet loss or send-rate issues.

### Additions that are premature

- **Command redundancy** (sending last N inputs per packet). Useful for lossy networks but adds serialization overhead and server-side dedup complexity. Wait until packet loss is measured and problematic.
- **Lag compensation / server rewind.** Requires world snapshot history and tick-timeline semantics. Only needed for competitive hitscan fairness. Wait until PvP balance matters.
- **Correction smoothing.** The temporal desync problem is real. A correct implementation requires computing the correction offset at camera-read time (Update), not ack-receive time (PreUpdate). This is doable but not urgent while corrections are near-zero.
- **Hermite interpolation.** Would smooth visual transitions but requires velocity tracking in PredictedRenderState. Worth doing if "snappy direction changes" are reported as a visual issue.
- **Clock synchronization.** Only needed if tick-based prediction or lag compensation is added. The current input-driven model doesn't need it.

### Migration risks

| Migration | Risk | Mitigation |
|---|---|---|
| Resources → Components (splitscreen) | Touches every prediction system | Mechanical refactor, well-typed |
| Adding tick fields to InputAck | Wire format change | Versioned protocol or additive fields |
| Correction smoothing | Temporal desync (proven) | Must compute offset at camera-read time |
| Command redundancy | Server dedup complexity | Use InputSequence for idempotent processing |
| Variable tick rates | Stale-tick semantics break | Must track elapsed time, not tick count |

### Debugging & tooling recommendations

1. **BRP fields to add to PredictionDiagnostics:**
   - `last_server_tick: u32` — from latest ack
   - `acks_this_frame: u32` — reset each frame
   - `stale_ticks_total: u64` — cumulative since connection
   - `history_depth: u32` — current PredictionHistory length
   - `last_stale_age: u32` — current StaleInput.age_ticks

2. **BRP fields to add to PredictedRenderState (register as Reflect):**
   - Already registered. Verify `elapsed`, `interval`, `ready` are visible.

3. **Diagnostic script improvement:**
   - `tmp/brp_snap_diag.sh` should sample `PredictionDiagnostics` fields above
   - Add a `--csv` mode for graphing correction magnitude over time

### Tick timeline diagram

```
Server FixedUpdate ticks:  T0    T1    T2    T3    T4    T5    T6
                           |     |     |     |     |     |     |
Client sends:         seq1 ──┐   seq2 ──┐         seq3 ──┐
                             |          |                 |
Server receives:             ▼          ▼                 ▼
Server applies:           [seq1] [seq2] [stale] [stale] [seq3]
Server acks:              ack(1) ack(2)                 ack(3)
                           pos1   pos2                   pos3
                                                          │
                           Note: pos3 = pos2 + 2 stale ticks + seq3
                           Client mirrors stale ticks locally
                           so prune(3) + replay produces pos3

Client prediction:    [seq1] [seq2] [stale] [stale] [seq3] [seq4] ...
                       ▲                                     ▲
                       history entry                         history entry
                              (stale ticks NOT in history)
```

This diagram shows why stale ticks don't need history entries: they're implicitly accounted for because the ack position already includes their effect, and the client mirrors the same stale behavior locally.
