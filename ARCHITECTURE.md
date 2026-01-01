# Architecture Review

This document captures issues, improvement opportunities, and redundancies noticed in the Rust
codebase (app, editor, and libraries). It is a backlog of findings, not a prioritized roadmap.

## App (apps/carcinisation)

### Issues / Risks
- `check_staged_cleared` runs every frame and triggers `StageClearedEvent` whenever progress
  reaches the final step; without gating, cleanup and music spawn can re-run after the stage
  is already cleared. Consider guarding with `StageProgressState` or a one-shot marker.
- `StageProgressState`, `StageProgress`, and `StageActionTimer` represent overlapping pieces of
  stage state (`apps/carcinisation/src/stage/systems.rs`, `apps/carcinisation/src/stage/resources.rs`);
  multiple sources of truth increase the risk of desyncs when new transitions are added.
- `StageStepSpawner` mutates a `Vec<StageSpawn>` via `retain_mut` and clones spawns on trigger
  (`apps/carcinisation/src/stage/systems/spawn.rs`); for long sequences, a queue (`VecDeque`)
  or index cursor would avoid repeated scans and cloning.
- `Depth` defaults and TODOs in `apps/carcinisation/src/stage/data.rs` currently use depth
  values known to be incorrect (bench objects set to `Depth::Eight`); these implicit defaults
  can hide rendering bugs in content.
- `PxAssets` writes metadata under a hard-coded `assets/` path
  (`apps/carcinisation/src/pixel/assets.rs`); this does not follow the `AssetPlugin` root, so
  running from alternate CWDs or tooling could generate meta files in the wrong location.

### Improvements / Redundancies
- UI overlays (cleared, death, game over, pause, main menu) repeat identical background/text
  spawn code. A shared "screen overlay" helper or data-driven UI descriptors would cut duplication
  (`apps/carcinisation/src/stage/ui/*`, `apps/carcinisation/src/main_menu/systems/layout.rs`).
- `globals.rs` mixes constants, helpers, and asset paths; splitting into focused modules
  (screen metrics, despawn helpers, asset paths) would reduce the global grab-bag.
- `PixelPlugin` is currently a no-op wrapper (`apps/carcinisation/src/pixel/mod.rs`); consider
  removing it or reintroducing a clear purpose (registration of pixel-specific systems).
- Deprecated `RailPosition` still exists in `apps/carcinisation/src/stage/components/placement.rs`;
  either migrate remaining users to `PxSubPosition` or remove to avoid split semantics.

## Editor (tools/editor)

### Issues / Risks
- `SceneData`, `ScenePath`, and `StageSpawnLabel` are tagged as both `Component` and `Resource`
  (`tools/editor/src/components.rs`), which blurs ownership and can lead to accidental entity
  storage or unintentional queries.
- `on_scene_change` despawns all `SceneItem` entities and rebuilds the scene whenever
  `StageControlsUI` or `SceneData` changes (`tools/editor/src/systems/mod.rs`). This is simple,
  but scales poorly for large scenes and makes incremental edits harder.
- Asset load handlers clone full `StageData`/`CutsceneData` into `SceneData`
  (`tools/editor/src/systems/mod.rs`), doubling memory and making edits diverge from the source
  assets. Prefer storing handles and reading from `Assets<T>` on demand.
- Selection/picking scans and sorts all `Draggable` entities on every click
  (`tools/editor/src/systems/input.rs`); a picking plugin or cached z-order list would reduce
  O(n log n) cost in large scenes.

### Improvements / Redundancies
- Cutscene and stage asset loading logic is nearly identical; a generic helper would reduce
  repeated state/print handling (`tools/editor/src/systems/mod.rs`).
- Logging uses `println!` instead of `info!`/`warn!`, which makes editor output inconsistent with
  the rest of the app.
- Selection outline entities are spawned separately and cleaned up manually. Parenting the
  outline to the selection or tracking it with a resource would simplify cleanup paths.
- `assets_root()` and `ASSETS_PATH` are hard-coded; align with `assert_assets_path!` or reuse
  the same asset root configuration used by the game to reduce drift.

## Libraries (crates/* and tools/*)

### Issues / Risks
- `carcinisation_collision::pixel_mask` uses reflection into `PxSpriteAsset` internals and caches
  results without invalidation (`crates/carcinisation_collision/src/pixel_mask.rs`). Upstream
  changes or hot-reloads can silently desync cached masks.
- `ColliderData::point_collides_with` clones colliders into a new `Vec` each call
  (`crates/carcinisation_collision/src/shapes.rs`); returning indices or references would avoid
  per-call allocations.
- `cween` exposes per-axis targeting types (`TargetingValueX/Y/Z`) and multiple bundles that
  mirror each other (`crates/cween/src/linear/components/mod.rs`); consider a generic `Vec2/Vec3`
  tween or a shared macro to reduce redundancy and potential divergence.

### Improvements / Redundancies
- `VolumeSettings` bundles master/music/sfx into one resource
  (`crates/carcinisation_core/src/components.rs`); splitting into separate resources or a
  structured config would make fine-grained updates more explicit.
- `scene-file-watcher` only validates `CutsceneData` RON files
  (`tools/scene-file-watcher/src/main.rs`); stage RONs are not checked despite similar risk.
- `activable` has a TODO to support nested plugin activation; clarifying intended usage and
  standardizing activation patterns would reduce copy-paste in app plugins.
