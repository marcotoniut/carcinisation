# Changelog

## Unreleased

### Added

- Sprite atlas assets (`.px_atlas.ron`) with region-based sprite sheets and animation.
- Composite sprites (`CxCompositeSprite`) for multi-part characters with per-part animation.
- Frame view/control API (`CxFrameView`, `CxFrameControl`, `CxFrameSelector`,
  `CxFrameTransition`, `CxFrame`) for manual or externally-driven frame selection.
- `CxAnimationPlugin` — animation systems are now opt-in.
- Pixel-perfect sprite picking with `CxPick`.
- `gpu_palette` feature with `CxGpuSprite` GPU sprite pass.
- `CxHeadlessPlugin` for integration tests and server builds.
- `Reflect` derives and feature-gated type registration (`reflect` feature).
- `brp_extras` feature for Bevy Remote Protocol integration.
- `profiling_spans` feature for tracing instrumentation.

### Changed

- Upgraded to Bevy 0.18.
- Two-phase filter extraction: `CxFilterLayers::Range` now resolves against the complete
  layer set instead of only layers collected so far. Range resolution uses binary search
  (`O(log L + K)`) instead of linear scan (`O(L)`).
- Rendering reuses screen buffers across frames.
- `CxFilterLayers` gained helper constructors (`single_clip`, `single_over`, `range_clip`,
  `many_clip`).
- Screen and UI code split into submodules.

### Fixed

- Image trimming no longer risks shrinking to zero width.
- Palette loading reports conversion errors and rejects palettes with more than 255 colors.
- Render-entity remapping for maps and picking.
- Input/event handling hardened for particle emitters.

### Performance

- Two-phase filter extraction with indexed range resolution (3-4x for range-heavy scenes).
- Per-call allocations eliminated in image slice iteration.
- `CxFilterRect` and `CxLine` drawing iterates only affected pixels.
- UI layout/caret/key/scroll systems gated with run conditions.
- GPU sprite node preallocates frame buffers.
- Map tile change detection with render-entity remap.
- Named `LayerContents` struct replaces anonymous tuples.
