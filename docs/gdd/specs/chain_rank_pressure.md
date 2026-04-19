# Chain, Rank, and Pressure — First-Pass Implementation Spec

Status: 💡 Proposed

Covers the minimum implementation needed to make the chain -> rank -> pressure loop playable and testable. Not a final design — tuning, UI, and per-enemy exceptions are deferred.

Parent design: [Gameplay Systems](../gameplay.md#scoring-)

---

## 1. Chain Tracking

### Purpose

Track continuous successful threat resolution. Chain is the primary input signal that drives rank.

### State

```
Resource: ChainState
  count: u32              — current chain length (consecutive kills)
  last_kill_time: Duration — timestamp of most recent kill (stage time)
  active: bool             — whether chain is live
```

Scoped to stage lifetime. Reset on stage entry.

### Inputs

| Event | Effect |
|-------|--------|
| Enemy killed | Increment `count`, update `last_kill_time`, set `active = true` |
| Chain timeout (no kill within window) | Set `active = false`, begin rank decay |
| Player takes damage | Reset `count` to 0, set `active = false` |

### Behaviour

- Chain activates on first kill
- Each subsequent kill within the timeout window extends the chain
- Timeout window is a single tunable duration (exact value deferred)
- Damage immediately breaks the chain — punishes trading hits for kills
- Chain count feeds into rank calculation (see below)

### Integration Point

Consume the existing `Dead` marker insertion. When an enemy entity receives `Dead`, the chain system updates `ChainState`. Use an observer on enemy death or check in a system that queries `Added<Dead>` with `Enemy` marker.

---

## 2. Rank State

### Purpose

Derive a pressure level from player performance. Rank modulates encounter intensity.

### State

```
Resource: RankState
  value: f32          — continuous rank value, clamped to [0.0, 1.0]
  baseline: f32       — derived from Difficulty enum at stage start
```

First pass uses a continuous normalised value. Simpler to interpolate pressure axes than discrete steps. Can be bucketed into named tiers later if needed for design communication.

### Baseline from Difficulty

| Difficulty | Baseline |
|------------|----------|
| Easy | 0.0 |
| Normal | 0.2 |
| Hard | 0.4 |

Baseline is the floor — rank cannot decay below it during a stage.

### Inputs

| Signal | Effect on `value` |
|--------|-------------------|
| Chain active + growing | Increment toward 1.0 (rate proportional to chain length) |
| Chain timeout (broken naturally) | Slow decay toward baseline |
| Chain broken by damage | Faster decay toward baseline |
| Extended inactivity (no kills, no damage) | Slow decay toward baseline |

### Behaviour

- Rank rises while chain is active and sustained
- Longer chains accelerate rank gain — a chain of 15 pushes rank harder than a chain of 3
- Rank never drops below baseline
- Rank never exceeds 1.0
- Exact rise/decay rates are tuning values — defer to playtesting

### Integration Point

`RankState` is a stage-scoped resource. Updated each frame (or on chain state change) by a dedicated system. Read by spawn/pressure systems.

---

## 3. Pressure Application

Rank modulates five axes. First-pass implementation should start with the two simplest (enemy count, spawn timing) and layer the rest incrementally.

### Axis: Enemy Count

- **How**: multiply authored spawn count per stop step by a rank-derived factor
- **Effect**: more enemies on screen simultaneously at higher rank
- **First pass**: `extra_spawns = floor(base_count * rank_value * scale_factor)` appended to `StageStepSpawner.spawns`
- **Constraint**: cap total simultaneous enemies to preserve readability

### Axis: Spawn Timing

- **How**: compress `elapsed` delays on `EnemySpawn` entries in tween steps
- **Effect**: enemies appear in quicker succession, tighter threat waves
- **First pass**: scale elapsed times by `1.0 - (rank_value * compression_factor)`, clamped to a minimum gap
- **Constraint**: minimum gap prevents enemies overlapping at spawn

### Axis: Depth Pressure (Deferred to Second Pass)

- **How**: bias spawned enemy depths toward broader lane spread at higher rank
- **Effect**: player must manage threats across more lanes simultaneously
- **Notes**: requires modifying `EnemySpawn.depth` before queue insertion; more invasive than count/timing

### Axis: Threat Composition (Deferred to Second Pass)

- **How**: substitute enemy types in spawn lists (e.g. mosquito -> mosquiton at higher rank)
- **Effect**: harder enemies appear earlier in the stage
- **Notes**: requires a substitution table mapping rank thresholds to enemy type upgrades

### Axis: Pursuer Frequency (Deferred)

- **How**: inject pursuer-behaviour enemies into spawn queues at higher rank
- **Effect**: continuous pressure forcing prioritisation
- **Notes**: blocked until at least one enemy type uses `PursueMovementPlugin`

---

## 4. System Responsibilities

Minimum moving parts for first pass:

### Resources (Stage-Scoped)

| Resource | Purpose |
|----------|---------|
| `ChainState` | Chain count, last kill time, active flag |
| `RankState` | Current rank value, difficulty baseline |

Both inserted on stage entry, removed on stage exit (follow `StageEntity` cleanup pattern).

### Systems

| System | Schedule | Responsibility |
|--------|----------|----------------|
| `update_chain` | `Update` | Watch for enemy deaths (`Added<Dead>` + `Enemy`), update `ChainState`. Check timeout each frame. |
| `update_rank` | `Update`, after `update_chain` | Read `ChainState`, adjust `RankState.value` toward target. Apply decay when chain inactive. |
| `modulate_spawns` | Runs once per step initialisation | Read `RankState`, modify `StageStepSpawner` contents before spawns begin draining. |

### Observer (Optional)

If preferred over query-based death detection:

```
on_enemy_killed(_trigger: On<EnemyKilledEvent>, chain: ResMut<ChainState>, ...)
```

This requires emitting an `EnemyKilledEvent` from the existing death pipeline. Either approach works — choose based on whether other systems also need a kill signal.

### Integration with Existing Code

- `modulate_spawns` hooks into step initialisation — runs after `initialise_stop_step` / `initialise_movement_step` populates `StageStepSpawner`, before `check_step_spawn` begins draining it
- `ChainState` reads stage time via `Time<StageTimeDomain>` for timeout tracking
- `RankState.baseline` set from `DifficultySelected` resource at stage entry

---

## 5. First Playable Slice

The smallest testable implementation that proves the loop:

1. **ChainState resource** — increments on enemy kill, resets on damage, times out on inactivity
2. **RankState resource** — rises while chain active, decays when chain breaks, floors at difficulty baseline
3. **Spawn timing modulation only** — compress `elapsed` delays in `StageStepSpawner` based on `RankState.value`
4. **One stage (Park)** — test with existing enemy spawns; observe that sustained chain play produces faster spawn waves

This is testable with zero new enemies, zero UI changes, and zero asset work. The loop is felt entirely through encounter pacing.

### Success Criteria

- Chain visibly accumulates during sustained kills (verify via debug log or inspector)
- Rank rises during chain, decays after break
- Spawn timing compresses noticeably at high rank vs. low rank
- Taking damage resets chain and triggers rank decay
- Difficulty baseline is respected (Hard starts with tighter spawns than Easy)

---

## 6. Explicit Deferrals

The following are intentionally out of scope for first implementation:

| Deferred | Reason |
|----------|--------|
| Exact tuning values (timeout window, rise/decay rates, compression factor) | Requires playtesting |
| Chain/rank UI display | Design direction says rank manifests through intensity, not UI |
| Score multiplier from chain | Scoring works without it; add after loop is validated |
| Depth pressure modulation | More invasive spawn modification; layer after count/timing work |
| Threat composition substitution | Requires enemy substitution tables; layer after base loop |
| Pursuer injection | Blocked until an enemy type uses `PursueMovementPlugin` |
| Per-enemy chain value weighting | All kills worth equal chain credit initially |
| Rank tier naming / bucketing | Continuous float is sufficient for first pass |
| Cross-stage rank persistence | Rank resets per stage initially |
| Bomb/destructible kills and chain | Unclear if these should count; defer decision |
