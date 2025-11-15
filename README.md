# Carcinisation

Carcinisation is a Bevy 0.17 workspace for a pixel-art action game and its supporting toolchain. The repository brings together the runtime, asset generators, editor utilities, and web build scripts under a single cargo workspace.

## Prerequisites

- [Rust toolchain](https://www.rust-lang.org/tools/install) with `cargo` on your `PATH`.
- [`cargo-watch`](https://crates.io/crates/cargo-watch) for hot-reload workflows (`cargo install cargo-watch`).
- [Bevy CLI](https://thebevyflock.github.io/bevy_cli) for `bevy run`, `bevy lint`, and browser builds. Install it with `cargo install bevy_cli --locked` and verify with `bevy --version`.
- [`proto`](https://moonrepo.dev/docs/proto) to provision the pinned Node.js, pnpm, Python, and Ruff versions (`proto install`).
- [`pnpm`](https://pnpm.io/installation) for Biome linting and web tooling (installed via `proto install` above).

## Quick Start

```bash
# rebuilds and reruns the game via `bevy run` whenever sources change (requires cargo-watch)
make dev

# run once without file watching
make run

# run the web target explicitly (same command make dev-wasm wraps)
bevy run --bin carcinisation --package carcinisation web

# lint everything Bevy knows about
bevy lint --workspace --all-targets --all-features
```

Run `make help` to discover additional targets for testing, asset generation, and web builds. Full command descriptions live in `DEVELOPMENT.md`.

## Python & Palette Tooling

The palette generator relies on the proto-managed Python toolchain so editors and CI stay in sync.

1. Install proto’s toolchain version pin: `proto install`.
2. Create or refresh the workspace virtualenv: `make py-setup`. This runs `proto run python -- -m venv .venv` and installs `scripts/generate-palettes/requirements.txt` into `.venv/`.
3. In VS Code, open the command palette and run `Python: Select Interpreter`, then pick `${workspaceFolder}/.venv/bin/python` (macOS/Linux) or `${workspaceFolder}\\.venv\\Scripts\\python.exe` on Windows. If IntelliSense lags, run `Python: Restart Language Server`.

Running `make generate-palettes` (or `proto run python -- scripts/generate-palettes/run.py`) uses proto directly, so CI and headless invocations never need to activate `.venv`. If you upgrade proto’s pinned Python, delete `.venv` and rerun `make py-setup`.

Troubleshooting: If VS Code reports `Import "PIL" could not be resolved`, re-run `make py-setup` and confirm the interpreter path above is selected.

## Project Layout

- `apps/carcinisation` – main game executable and gameplay plugins.
- `assets/` – runtime content: stages, cinematics, palettes, and sprites.
- `tools/` – supporting utilities (editor, scene watcher, palette generator, asset validators).
- `scripts/` – Rust binaries for typeface and graphics generation.
- `web-deploy/` – output for wasm builds.

## Documentation

- `CONTRIBUTING.md` – project guardrails, quality gates, and pull-request expectations.
- `DEVELOPMENT.md` – make targets, asset tooling, palette generation workflow, and day-to-day processes.
- `AGENTS.md` – Codex/Claude collaboration rules and coordination tips.
- `CLAUDE.md` – Claude-specific planning and documentation preferences.

## Planning Work

- Open a GitHub issue or discussion when planning sizeable architecture or content updates.
