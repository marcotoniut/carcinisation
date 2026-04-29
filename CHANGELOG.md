# Changelog — Carcinisation

## 0.3.0

### Added — First-Person Shooter Mode
- New `carcinisation_fps` crate: Wolf3D-style DDA raycasting, grid maps, billboard sprites, enemy AI (idle/chase/attack), hitscan combat, distance fog, enemy projectiles, death screen.
- `FpPlugin<L>` for integrating FP mode into any Bevy app with Carapace.
- RON-driven FP map format with procedural wall textures, entity spawns, and fog config.
- Mosquiton billboard sprites exported from Aseprite source.
- `fp_test` dev binary for standalone FP testing with GBInput + leafwing.
- Per-column z-buffer for wall/billboard depth occlusion.
- Bayer-dithered distance fog and wall side shading.

### Added — Splash Screen
- Boot-time Bevy splash screen driven by RON config (`bevy.ron`).
- Splash delegates to CutscenePlugin with per-track rotation keyframes.
- Follower/relative-offset rotation for multi-layer bird animation.
- `cutscene_loop` dev binary for splash tuning.

### Added — Composition & Atlas System
- Composed sprite pipeline: multi-part characters with per-part animation, health pools, and pixel-authoritative collision.
- Compact RON atlas metadata (`atlas.composed.ron`) with sprite packing, semantic parts, and per-frame poses.
- PXI compact indexed atlas image format.
- Fragment splitting for self-symmetric sprite canonicalisation.
- Per-part render-time transforms (rotation, scale, flip) in composite sprites.
- Per-animation part overrides in atlas metadata (wing flap, leg overrides).
- `AnimationComplete` cue when finite animations exhaust repeats.
- Encoder capacity warnings for compact composed atlas.

### Added — Attack System
- Atlas-based player attacks with visual-accurate origins and depth scaling.
- Flamethrower weapon with particle chain, drain mechanics, and collider scaling.
- Spidey spiderweb projectile with Webbed debuff.
- Spider_shot, boulder_throw, blood_shot attack types with data-driven animations.
- Bomb and machinegun attacks.
- Pixel-mask collisions for sprite-accurate hit detection.
- Radial attack collision mode.
- Attack tuning externalised to RON config files.

### Added — Enemy System
- Mosquiton composed enemy with breakable wings, depth-specific art, and behaviour patterns.
- Spidey composed enemy with jump traversal, semantic part targeting, and lounge animations.
- Tardigrade enemy type.
- Depth-aware enemy placement and behaviour scripting.
- Composed health pools with per-part durability and damage routing.
- Continuous depth tracking for smooth enemy Z-movement.

### Added — Pickup System
- Composed atlas visuals for pickups with depth scaling.
- Pickup drop physics (gravity, velocity, floor clamping).
- Health recovery feedback arc with parallax-aware start position.
- Glitter filter toggle and HUD flash on pickup.

### Added — Stage System
- Parallax system with depth-weighted lateral displacement.
- Floor surface model with per-depth overrides and gap/solid semantics.
- Projection system: quadratic depth-to-Y mapping with lerp between step profiles.
- Perspective grid visualisation (shared between runtime debug overlay and editor).
- Stage primitives and primitive band configuration.
- Checkpoint continue system with mid-stage resume.
- Data-driven stage step progression (tween, stop, cinematic).

### Added — Cutscene System
- Shared rotation keyframe infrastructure (Easing with DampedSpring, RotationKeyframe, evaluate_rotation_keyframes).
- Timeline curve followers with per-element time scaling.
- Rotation followers (relative offset from leader entity).
- CutsceneAppearAt for delayed element visibility.
- Background primitive spawning in cutscene acts.
- AnyGameplayKey skip mode.

### Added — Editor
- Full depth selection, scaled preview, pose selector, and depth hotkeys.
- Spawn creation, deletion, and camera navigation.
- Projection gizmos with drag-based authoring.
- Stage metadata inspector and resource inspector.
- Timeline interactions with act labels and path tracing.

### Added — Debug & Dev Tools
- Depth debug overlays (perspective grid, entity anchors) with env var toggles.
- Player attack collision overlay with projectile color tuning.
- Debug god mode, debug spawn system.
- `depth_traverse` example binary.
- `carapace_mosquiton_stress` stress test binary.
- Character gallery (`--features gallery`).

### Added — Input
- Player intent resolution with Select+A melee chord (80ms grace window).
- B button as slow movement modifier.
- Remapped B to ShiftLeft, Select to KeyZ.
- Context-sensitive input on game_over and cleared screens.

### Added — Infrastructure
- Agent validation script (`check:agent`) with surface profiles.
- Playwright MCP integration for visual testing.
- Aseprite MCP for sprite operations.
- Lefthook git hooks (rustfmt, clippy, biome, ruff).

### Refactored — Crate Extraction
- `carcinisation_input`: GBInput enum, key mappings, init_gb_input.
- `carcinisation_animation`: Easing, RotationKeyframe, RotationKeyframes, evaluate_rotation_keyframes.
- `carcinisation_cutscene`: CutsceneLayer, cutscene components, CutsceneTimeDomain, CutsceneProgress.
- `carcinisation_base`: Layer enum (Ors/Fps/Cutscene/Menu/Shared sub-layers), GameProgressState, Score, Lives, CameraPos.
- `carcinisation_ors`: stage/, data/, assets/ (on-rails shooter gameplay, ~30K lines).
- Eliminated `pixel/` module — bundles upstreamed to `carapace::bundles`, assets moved to crate root.
- Moved SCREEN_RESOLUTION, HUD_HEIGHT, mark_for_despawn_by_query to `carcinisation_core::globals`.

### Refactored — Architecture
- Layer enum restructured: `Ors(OrsLayer)`, `Fps(FpsLayer)`, `Cutscene(CutsceneLayer)`, `Menu(MenuLayer)`, `Shared(SharedLayer)`.
- State-based anchor system with auto-derivation and per-animation overrides.
- Split composed enemy visuals into Update gameplay + PostUpdate render.
- Derive depth from continuous state (replaces discrete-only).
- Spawn anchor model formalised with editor/runtime parity.
- Unified collision scaling across depth levels.

### Fixed
- Pickup feedback arc parallax offset.
- Composed mask rect Y-axis off-by-one.
- Spidey lounge→idle leg override frame desync.
- Attack race condition and position sync.
- ECS scheduling, position authority, dead-entity guards, camera shake lifecycle.
- Projection gizmo drag clamping.
- HUD text visibility, attacks during pause, end-of-level crash.

### Performance
- Two-phase filter extraction with indexed range resolution.
- Per-call allocations eliminated in image slice iteration.
- GPU sprite node frame buffer preallocation.
- Map tile change detection with render-entity remap.
- UI layout/caret/key/scroll systems gated with run conditions.
- Skip composite metric rescans.

### Upgraded
- Bevy 0.12 → 0.13 → 0.14 → 0.15 → 0.17 → 0.18.
- Rust edition 2024, toolchain 1.93.1.
- seldom_pixel vendored and rebranded as Carapace.
