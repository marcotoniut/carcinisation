# Gameplay Systems

## Player Movement ✅

- 4-directional input (Game Boy d-pad), constant 125.0 px/s
- Confined to screen bounds (0-160 X, HUD height to 144 Y)
- Fixed at Depth::Zero — player never occupies enemy depth lanes
- Purpose: position for melee range and dodge incoming attacks
- Source: `apps/carcinisation/src/player/`

## Depth and Perspective ✅

The defining mechanical system and core skill axis. Enemies exist on discrete depth lanes (1-9), creating a pseudo-3D battlefield the player attacks into from the foreground. Optimal play is defined by resolving threats across multiple lanes efficiently, not raw kill speed alone.

### Depth Lanes

- 10 discrete layers: Depth::Zero (player/UI) through Depth::Nine (furthest)
- Enemies occupy lanes 1-9; player fixed at Zero
- Attacks resolve across depths — collision is depth-aware
- Enemies can transition between lanes during movement steps

### Projection

Maps depth to screen-Y position via cubic curve:

```
floor_y = horizon_y + (normalised_depth ^ bias_power) * (floor_base_y - horizon_y)
normalised_depth = (9 - depth) / 8
```

Defaults: horizon ~50% screen height, bias power 3.0. Stages can override per-step for camera lens shifts.

### Depth Scaling

Geometric progression — Depth 1 at 1.0x, Depth 9 at 0.04x (~0.67 ratio per step). Visual only; does not affect collision. Config: `assets/config/depth_scale.ron`.

### Authored Depths

Key enemies have hand-drawn sprites per depth (e.g. mosquito: depths 1-7). System selects nearest authored variant and applies ratio scaling for unauthored depths.

### Directional Parallax Feedback 💡

- Background parallax shifts dynamically with player movement to reinforce spatial perception and a subtle sense of rotation/orbital motion
- Visual feedback only — does not affect gameplay, collision, or depth mechanics
- Must remain subtle and used sparingly
- **Visual priority**: enemies and projectiles always take precedence over background effects — no visual layer may reduce gameplay readability

### Open Questions

- Player depth movement (dodge into/out of lanes)?
- Depth interaction with environmental hazards?

## Combat ✅

Dual-button system: A = melee, B = ranged. Melee is primarily a close-range stabilisation tool — it controls nearby threats under pressure but is not the intended optimisation path. Ranged reaches into depth and drives efficient chain play.

### Player Attacks

| Attack | Type | Damage | Input | Collision | Notes |
|--------|------|--------|-------|-----------|-------|
| Pincer | Melee | 70 | Release | Sprite mask | Repeat hits (0.18s, 35/tick) |
| Pistol | Ranged | 30 | Release | Point | Instant, single target |
| Machine Gun | Ranged | 20 | Hold | Point | 0.18s warmup, 0.08s interval, 2 deg spread |
| Bomb | Ranged | 0 (det: 60) | Release | Point | Arc trajectory, screen shake |

Bomb is a standard combat option (area denial, grouped targets), not a panic/reset mechanic. Bomb depth interaction and effectiveness scaling are intentionally undefined.

Source: `apps/carcinisation/src/player/attacks.rs`

### Damage Resolution

1. Attack entity spawns with collision mode (point or sprite mask)
2. Hit -> `DamageMessage` observer -> `Health` decremented + `Flickerer` feedback
3. Health 0 -> `Dead` marker -> cleanup pipeline
4. Optional `SpawnDrop` on death (pickups, secondary spawns)
5. Critical hits trigger when enemy defence <= 0.5 (awards bonus score)

### Enemy Attacks ✅

| Attack | Source | Constraint |
|--------|--------|------------|
| Blood shot | Mosquiton (ranged) | Depth <= 7 |
| Boulder throw | Tardigrade (ranged) | — |
| Spider shot | Spidey (ranged) | — |
| Hovering damage | Area-of-effect | Proximity-based |
| Contact damage | Any (melee) | On collision |

Source: `apps/carcinisation/src/stage/attack/`

### Open Questions

- Weapon unlock/upgrade progression (all attacks currently available from start)
- Depth-distance affecting attack effectiveness?

## Scoring ✅

Points accumulate during a run. Score penalised on death and health pickup use.

### Score Sources

| Event | Points |
|-------|--------|
| Ranged hit | +1 |
| Ranged critical hit | +4 |
| Melee hit | +3 |
| Melee critical hit | +10 |
| Enemy kill | +7 to +10 (per type) |
| Player death | -150 |
| Health pickup | -2 x health restored |

- Melee is higher-risk, higher-reward than ranged
- Health usage trades score for survival
- Top 5 high scores persisted
- Source: `apps/carcinisation/src/game/score/`

**Intended feedback loop** 💡: efficient play (kills, chains) -> higher rank -> increased encounter pressure -> demands continued efficiency or forces trade-offs (health pickups, safer ranged play). Scoring, chaining, and rank are facets of one system, not independent features.

### Kill Chain 💡

Not implemented. Chain represents continuous successful threat resolution — the primary input signal for rank.

- Consecutive kills within a time window sustain and extend the chain
- Chain rewards fast decision-making, multi-target prioritisation, and engagement across depth lanes
- Breaking chain (long gap between kills, taking damage) stalls or decays rank
- Chain is the main driver of rank acceleration — not raw score alone
- Design constraint: must remain readable on 160x144 display

## Rank / Dynamic Difficulty 💡

Not implemented. Proposed direction:

- **Rank** is driven by player performance — primarily chain efficiency, secondarily damage taken and score rate
- Difficulty enum (Easy/Normal/Hard) sets the rank baseline; rank modulates within that range
- Rank is elastic: rises through sustained chain, decays through damage, death, passivity, or broken chains — players can always reduce pressure through safer play or failure
- Depends on: scoring system, chain system, enemy spawn system

### Pressure Manifestation

Rank does not display as UI. The player feels rank through what the game throws at them:

| Aspect | Effect at Higher Rank |
|--------|-----------------------|
| Enemy count | More simultaneous enemies on screen |
| Spawn timing | Shorter gaps between threat waves |
| Depth pressure | Attacks arrive across multiple lanes simultaneously |
| Threat composition | More complex enemy types and movement patterns appear earlier |
| Pursuer frequency | Pursuing enemies introduced sooner (see [Pursuers](#pursuers-)) |

Lower rank eases all of the above — self-balancing difficulty curve.

**Escalation constraint**: pressure must escalate through coordinated adjustment across multiple axes, not isolated spikes in a single dimension. At every rank level, threats must remain readable and solvable through player action.

### Loop Example

Player maintains chain -> rank rises -> enemies spawn faster across multiple depths -> player must resolve threats more efficiently to sustain chain -> chain breaks or player adapts and pressure holds.

## Enemies 🚧

### ✅ Implemented

**Mosquito** — baseline fodder
- Simple sprite, multi-depth variants (depths 1-7)
- Movement: linear tween, circle orbit, idle, jump
- Attack: contact damage only

**Mosquiton** — composed multi-part (wings + body)
- Authored at Depth::Three; part-based animation system
- Ranged (blood shot) + melee; per-part health tracking
- BrokenWings state: falling animation, disables flight
- Source: `apps/carcinisation/src/stage/enemy/mosquiton/`

**Tardigrade** — heavy tank
- Slow, high health; boulder throw + contact damage

**Spidey** — composed multi-part
- Animation loaded, behaviour partially stubbed
- Source: `apps/carcinisation/src/stage/enemy/spidey/`

### Pursuers 💡

Enemies that apply continuous pressure by tracking the player position. Purpose: force prioritisation and punish passive play. If left alive, pursuers disrupt chaining by demanding attention away from optimal kill order. Movement infrastructure exists (`PursueMovementPlugin` in cween, registered in stage plugin) but no enemy uses it yet.

### 💡 Not Implemented

- **Marauder** — enum defined, no assets or behaviour
- **Spidomonsta** — implied boss variant, undefined
- **Kyle** — enum defined, undefined

### Enemy Movement ✅

Scripted step sequences defined in stage data:

| Step | Behaviour |
|------|-----------|
| LinearTween | Move along X/Y vector at base speed |
| CircleAround | Orbit point (direction, radius, duration) |
| Jump | Parabolic arc via Z-tween |
| Idle | Hold position for duration |
| Attack | Execute attack for duration |

Steps can include `depth_movement` to shift between lanes mid-motion. All movement is currently authored (scripted), not reactive. Pursue-target movement infrastructure exists in cween but is unused — intended for pursuer-type enemies.

### Open Questions

- Which existing enemy types gain pursuer behaviour at higher rank?
- Boss encounter design
- Per-rank enemy count targets

## Pickups ✅

- Health packs (restore HP, costs score) and bomb pickups (replenish ammo)
- Sources: enemy drops (`SpawnDrop`), stage data placement, destructible contents
- Per-depth sprite variants

## Destructibles ✅

- Breakable stage objects (lamps, containers) with own health pool
- On destruction: spawn contained items (pickups, enemies)
- Purpose: optional risk/reward encounters, pacing variation

## HUD ✅

- 14px strip at screen bottom; displays health, lives, score
- Source: `apps/carcinisation/src/stage/ui/`
