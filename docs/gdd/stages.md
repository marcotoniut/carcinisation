# Stage Structure and Progression

## Stage Model ✅

Each stage is a self-contained level defined as a `.sg.ron` file in `assets/stages/`. Stages orchestrate camera movement, enemy spawning, and encounter pacing through a sequential step system.

### Data Structure

```
StageData
├── name, background_path, music_path
├── skybox (animation frames, timing)
├── start_coordinates (camera spawn)
├── spawns[] — entities present at load
└── steps[] — sequential progression
```

Source: `apps/carcinisation/src/stage/data.rs`

### Spawns

| Type | Purpose |
|------|---------|
| Object | Static props (visual only) |
| Pickup | Collectibles (health, bombs) |
| Enemy | Hostile entities with movement step sequences |
| Destructible | Breakable objects containing drops |

Each spawn carries: position, depth, type-specific data (health, speed, drops, movement steps).

### Steps

Executed sequentially to form the stage timeline:

| Step | Behaviour | Purpose |
|------|-----------|---------|
| **Tween** | Camera scrolls to target; timed spawns fire during movement | Advance through stage, introduce enemies gradually |
| **Stop** | Camera holds; waits for enemy defeat or timer | Arena encounters, pacing gates |
| **Cinematic** | Trigger cutscene sequence | Narrative beats (basic implementation) |

Tween steps support per-depth floor layout overrides and projection profile shifts. Floor layout defines gameplay surfaces; projection defines how those lanes are rendered.

### Encounter Pattern

```
Tween (scroll + spawn) -> Stop (arena hold) -> Tween -> Stop -> ... -> Stage End
```

Tween sections introduce enemies at authored intervals. Stop sections create focused combat arenas — player must clear the wave or survive a timer before the stage advances.

## Campaign 🚧

### Structure

```
GameData
└── steps[]
    ├── CinematicAssetGameStep — cutscene
    └── StageAssetGameStep — stage (optional checkpoint flag)
```

Source: `apps/carcinisation/src/progression/`

### Current State

- Active path: Intro cinematic -> Park stage
- Spaceship and Asteroid backgrounds exist; no stage files authored
- Checkpoint flags defined in data; resume logic incomplete

### Player Lifecycle ✅

| Event | Result |
|-------|--------|
| Death | Death screen -> continue from checkpoint (if available) or game over |
| Game over | Restart from beginning; high score recorded |
| Stage cleared | Advance to next GameStep |

- Lives: 3 default
- Difficulty enum (Easy/Normal/Hard) defined but **not wired** — intended as rank baseline (see [Rank](gameplay.md#rank--dynamic-difficulty-))

## Environments

### Park ✅

First and only playable stage. Outdoor setting with benches, trees, signs across multiple depths. Encounters: mosquitos, mosquitons, destructible lamps. Stage file: `assets/stages/park.sg.ron`.

### Spaceship 💡

Background assets only (`spaceship/`). Intended setting for the [vacuum window mechanic](specs/vaccum_window_mechanic.md).

### Asteroid 💡

Background assets only (`asteroid/`).

## Tooling ✅

- **Stage editor**: `tools/editor/` — visual placement and editing
- **Live reload**: `tools/scene-file-watcher/` — hot-reload `.ron` without rebuild
- **Asset validation**: `tools/assert_assets_path/` — compile-time path checks

## Open Questions

- Stage count and ordering for full campaign
- How difficulty baseline + rank should scale encounters (see [Rank](gameplay.md#rank--dynamic-difficulty-))
- Linear progression vs. branching paths
