# Changelog

## Unreleased

### Added

- Sprite atlas assets (`.px_atlas.ron`) with region-based sprite sheets and animation.
- Composite sprites (`PxCompositeSprite`) for multi-part characters with per-part animation.
- Frame view/control API (`PxFrameView`, `PxFrameControl`, `PxFrameSelector`,
  `PxFrameTransition`, `PxFrame`) for manual or externally-driven frame selection.
- `PxAnimationPlugin` — animation systems are now opt-in.
- Pixel-perfect sprite picking with `PxPixelPick`.
- `gpu_palette` feature with `PxGpuSprite` GPU sprite pass.
- `PxHeadlessPlugin` for integration tests and server builds.
- `Reflect` derives and feature-gated type registration (`reflect` feature).
- `brp_extras` feature for Bevy Remote Protocol integration.
- `profiling_spans` feature for tracing instrumentation.

### Changed

- Upgraded to Bevy 0.18.
- Two-phase filter extraction: `PxFilterLayers::Range` now resolves against the complete
  layer set instead of only layers collected so far. Range resolution uses binary search
  (`O(log L + K)`) instead of linear scan (`O(L)`).
- Rendering reuses screen buffers across frames.
- `PxFilterLayers` gained helper constructors (`single_clip`, `single_over`, `range_clip`,
  `many_clip`).
- Screen and UI code split into submodules.

### Fixed

- `PxImage::trim_right` no longer risks shrinking to zero width.
- Palette loading reports conversion errors and rejects palettes with more than 255 colors.
- Render-entity remapping for maps and picking.
- Input/event handling hardened for particle emitters.

### Performance

- Two-phase filter extraction with indexed range resolution (3-4x for range-heavy scenes).
- Per-call allocations eliminated in `PxImageSliceMut::for_each_mut`.
- `PxRect` and `PxLine` drawing iterates only affected pixels.
- UI layout/caret/key/scroll systems gated with run conditions.
- GPU sprite node preallocates frame buffers.
- Map tile change detection with render-entity remap.
- Named `LayerContents` struct replaces anonymous tuples.
