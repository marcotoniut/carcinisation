# Changelog

## 0.10.0-dev

### Added

- `carapace::image::CxImage` made public with accessor API (`new`, `empty`, `width`, `height`, `size`, `area`, `get_pixel`, `data`, `data_mut`, `clear`).
- `carapace::raycaster` module with `draw_wall_column` helper for vertical texture strip rendering.
- `carapace::bundles` module with convenience spawn bundles: `CxSpriteBundle<L>`, `CxTextBundle<L>`, `CxLineBundle<L>`, `CxFilterRectBundle<L>`, `CxAnimationBundle`. All re-exported from prelude.
- `CxSpriteAsset::from_raw(data, width)` constructor for creating sprite assets from raw palette-indexed data.
- `CxSpriteAsset::extract_frame(n)` to pull a single animation frame as a standalone `CxImage`.
- `CxSpriteAsset::frame_width()` and `frame_height()` accessors.

## 0.9.0-dev

### Added

- Per-entity presentation transforms (`CxPresentationTransform`): rotation, scale, flip, visual/collision offsets.
- Per-part render-time transforms for composite sprites (rotation, scale, pivot).
- Primitive drawing system (`CxPrimitive` with Rect, Circle, Polygon shapes; Solid, Checker, OrderedDither fills).
- Line drawing module (feature-gated `line`).
- `CxBlink` component for entity blinking.
- Debug draw helpers (`draw_circle_collider_2d`, `draw_rect_collider_2d`, `draw_world_mask_outline_2d`).
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
