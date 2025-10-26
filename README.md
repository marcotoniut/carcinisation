# Carcinisation

Carcinisation is a Bevy 0.17 workspace for a pixel-art action game and its supporting toolchain. The repository brings together the runtime, asset generators, editor utilities, and web build scripts under a single cargo workspace.

## Prerequisites

- [Rust toolchain](https://www.rust-lang.org/tools/install) with `cargo` on your `PATH`.
- [`cargo-watch`](https://crates.io/crates/cargo-watch) for hot-reload workflows (`cargo install cargo-watch`).
- [`pnpm`](https://pnpm.io/installation) for Biome linting and web tooling.

## Quick Start

```bash
# rebuilds and reruns the game whenever sources change (requires cargo-watch)
make dev

# run once without file watching
make run
```

Run `make help` to discover additional targets for testing, asset generation, and web builds. Full command descriptions live in `DEVELOPMENT.md`.

## Project Layout

- `apps/carcinisation` – main game executable and gameplay plugins.
- `assets/` – runtime content: stages, cinematics, palettes, and sprites.
- `tools/` – supporting utilities (editor, scene watcher, asset validators).
- `scripts/` – binaries for palette, typeface, and graphics generation.
- `web-deploy/` – output for wasm builds.

## Documentation

- `CONTRIBUTING.md` – project guardrails, quality gates, and pull-request expectations.
- `DEVELOPMENT.md` – make targets, asset tooling, and day-to-day workflows.
- `AGENTS.md` – Codex/Claude collaboration rules and coordination tips.
- `CLAUDE.md` – Claude-specific planning and documentation preferences.

## Planning Work

- Open a GitHub issue or discussion when planning sizeable architecture or content updates.
