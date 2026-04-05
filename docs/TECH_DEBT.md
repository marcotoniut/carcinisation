# Technical Debt

Known issues, improvement opportunities, and redundancies in the Rust codebase (app, editor, and libraries). This is a backlog of findings, not a prioritized roadmap.

## App (apps/carcinisation)

### Issues / Risks
- `StageProgressState`, `StageProgress`, and `StageActionTimer` represent overlapping pieces of
  stage state. Reset ordering is now consistent via `reset_stage_progression()`, but the three
  resources still form an implicit dependency chain that could desync if new transition paths
  are added without using the helper.

### Improvements / Redundancies
- UI overlays (cleared, death, game over, pause) share identical spawn structure (background rect
  + 3 text lines at Y=90/60/50). Main blockers to full unification: Bevy requires distinct marker
  component types per screen, and the death screen title is dynamic. Typeface loading is now
  shared via `load_inverted_typeface()`, but the spawn bundles remain duplicated (~4 instances).
- `globals.rs` mixes constants and helpers across ~5 categories. Could be split into focused
  modules (viewport, asset_paths, lifecycle, config) for discoverability, though at ~70 lines
  it's manageable as-is.

## Editor (tools/editor)

### Notes (investigated, acceptable trade-offs)
- Asset load handlers clone full data into `SceneData` at load time (~20-35 KB). The editor
  mutates data in-place from ~15 locations, making a handle-based approach impractical.
- Selection/picking sorts all `Draggable` entities per click (O(n log n)). At typical stage
  sizes (20-50 entities) this costs ~1-5 microseconds. A spatial index breaks even at ~60-80
  entities.

## Libraries (crates/* and tools/*)

### Notes (investigated, acceptable trade-offs)
- `carcinisation_collision::pixel_mask` caches pixel data via reflection into `PxSpriteAsset`
  internals. Cache invalidation IS implemented (listens to `AssetEvent` and clears on
  modification), but the reflection-based extraction is fragile against upstream field changes.
- `cween` per-axis targeting types (`TargetingValueX/Y/Z`) appear redundant but are
  architecturally required: Bevy's `Changed<T>` filter needs distinct types for per-axis
  change detection.
- `VolumeSettings` bundles master/music/sfx into one resource but is never mutated during
  gameplay. Splitting would add boilerplate without benefit.
- `activable` nested plugin TODO: current explicit pattern (~33 lines across 11 relationships)
  provides valuable clarity. A macro would save ~2 lines per relationship.
