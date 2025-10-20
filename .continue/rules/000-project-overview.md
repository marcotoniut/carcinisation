# Carcinisation — Overview

- Jam origin: GBJam 11 (Game Boy Jam).
- Engine: Bevy currently 0.15; keep the workspace compiling before jumping to 0.17.
- Core loop: Captain Acrab crash → fight aliens → return to asteroid.
- Extra stages exist in code; transitions incomplete (assets present, flow missing).
- Art: GB-style palette constraints.

## Game + Tooling Scope

- Primary game binary lives in `apps/carcinisation` (workspace default).
- Tooling crates (Rust) support builds, editor features, and asset workflows.
- Scripts in `scripts/` drive palette/typeface/gfx generation; they are part of the workspace.
- Web build and dev server logic lives under `tools/editor` (TypeScript) and Make targets.

## Workspace Layout

- `apps/carcinisation/` — main Bevy game crate (entry point, stages, cutscenes).
- `tools/editor/` — Tauri/Vite-style tooling for inspecting game data.
- `tools/scene-file-watcher/`, `tools/assert_assets_path/` — CLI helpers used during iteration.
- `scripts/*` — Rust binaries invoked via Make to preprocess assets.
- `assets/` — RON, TOML, art; keep structure stable unless coordinating with asset pipelines.
- `docs/` — project-specific notes referenced by local documentation tooling.

## Development Guidelines

- Prefer incremental, behavior-preserving changes; stage refactors behind green builds.
- Keep shared components usable by both the game and tools—watch for cross-crate API changes.
- Surface missing documentation as you touch modules, but prioritize functionality first.
- For feature work, outline intent in comments or docs where future maintainers will look.

## Non-goals

- Avoid large refactors during Bevy upgrades. Compile with minimal diffs first, then polish.
