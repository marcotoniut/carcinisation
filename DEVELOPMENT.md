# Development Guide

Commands and workflows for building and testing Carcinisation.

## Toolchain Setup

Install the pinned Node.js, pnpm, Lefthook, Python, and Ruff versions via [`proto`](https://moonrepo.dev/docs/proto):

```bash
proto install
lefthook install    # skip in CI
```

### Bevy CLI

Install the [Bevy CLI](https://thebevyflock.github.io/bevy_cli) so all local commands can run and lint through `bevy` instead of raw `cargo`:

```bash
cargo install bevy_cli --locked
bevy --version
```

`make run`, `make dev`, `make dev-wasm`, `make build`, `make build-release`, and `make lint` all shell out to the CLI. If you prefer calling it directly:

```bash
# Native build
bevy run --bin carcinisation --package carcinisation

# Browser build served on localhost:4000
bevy run --bin carcinisation --package carcinisation web

# Workspace lints across the entire project
bevy lint --workspace --all-targets --all-features
```

## Quick Start

```bash
# Rebuild and rerun automatically when game source or asset files change via `cargo watch --no-restart` + `bevy run` (requires cargo-watch; the watcher remains alive even if a run crashes, so fix the error before the next change triggers a rerun)
make dev

# Run once with the default feature set via `bevy run --bin carcinisation`
make run

# Launch the scene editor utility
make launch-editor

# Validate RON asset changes while editing
make watch-scene-files
```

If `cargo watch` is missing, install it with `cargo install cargo-watch` before running `make dev` or `make test-watch`.

## Make Targets Overview

Use `make help` to see the full catalog. Common targets:

```bash
make dev               # cargo-watch --no-restart watching `apps/carcinisation/src` and `assets`, reruns only on those changes while keeping the watcher alive even after crashes
make run               # bevy run --bin carcinisation (RUST_BACKTRACE=full)
make dev-wasm          # bevy run --bin carcinisation --package carcinisation web
make launch-editor     # Open the Bevy-based scene editor
make watch-scene-files # Watch assets/ for RON parsing errors
make generate-palettes # Rebuild palette/filter assets via Python + Pillow
make generate-typeface # Rebuild font/typeface assets
make process-gfx       # Run graphics processing scripts
make install-web-deps  # Install wasm toolchain dependencies
make build-web         # Produce release wasm build + bindings via make-web.sh
make release-wasm      # Release wasm binary + wasm-opt pass
make fmt               # cargo fmt --all
make lint              # bevy lint --workspace --all-targets --all-features
make test              # cargo test --workspace --all-features
make test-watch        # Re-run tests on change via cargo-watch
```

Override feature flags or runtime arguments when you call make:

```bash
# Run without dynamic linking
make run FEATURES=""

# Pass additional CLI args to Bevy (if/when added)
make run ARGS="--help"
```

## Hot Reloading & Iteration

- **Code + Assets**: `make dev` uses `cargo watch --no-restart` to rerun when files under `apps/carcinisation/src` or `assets/` change. The watcher stays alive even if a build or the game crashes, so you resolve errors before the next save retriggers a run.
- **Assets**: Bevy reloads assets in `assets/` automatically while the game is running. The scene watcher (`make watch-scene-files`) prints parser errors for `.ron` files so you spot mistakes early.
- **Stages & Cutscenes**: Stage data lives under `assets/stages/*.sg.ron`; cutscenes live under `assets/cinematics/*.cs.ron`. Keep Bevy running in `make dev` and run the watcher to validate edits live.

## Tooling

### Scene Editor

Run `make launch-editor` to open the standalone editor in `tools/editor`. It loads assets from the main game's `assets/` directory and lets you inspect or tweak stage and cutscene data with EGUI tools.

### Scene File Watcher

`make watch-scene-files` starts the watcher in `tools/scene-file-watcher`, which continuously parses `.ron` assets under `assets/`. Use it alongside your editor to catch syntax errors before reloading in-game.

### Asset Generators

- `make generate-palettes` rebuilds color palettes (and their `filter/*.px_filter.png` helpers)
- `make generate-typeface` recreates bitmap typefaces in `assets/typeface/`.
- `make process-gfx` runs the graphics processing pipeline for sprites and UI assets.

Palette generation is now pure Python, so it no longer triggers a Rust rebuild or requires the Rust workspace to compile.

## Web Builds

1. Run `make install-web-deps` once to install the wasm targets (`wasm-bindgen-cli`, `wasm-opt`, etc.).
2. Use `make build-web` to create a release wasm build under `web-deploy/` with bindings and assets.
3. Alternatively, `make release-wasm` leaves the optimized wasm binary in `target/wasm32-unknown-unknown/release/` for further packaging.

## Development Workflow

1. Sync main and create a feature branch.
2. Iterate with `make dev`, optionally running the scene editor or watcher tools in parallel.
3. When changes stabilize, run the quality gates:

   ```bash
   make fmt && make lint && make test
   ```

4. Open a pull request following `CONTRIBUTING.md`.

## Debugging Tips

- **Backtraces**: `make run` already sets `RUST_BACKTRACE=full`. Combine with `RUST_LOG=debug` to see verbose logs.

  ```bash
  RUST_LOG=debug make run
  ```

- **Editor overlay**: In debug builds (`make run` / `bevy run --bin carcinisation`), the game injects `bevy_editor_pls`. Use its default shortcut (`Ctrl+Shift+P`) to toggle the inspector overlay.
- **Diagnostics**: The debug plugin draws helper overlays when its state is active. Check `src/debug` for details and extend as needed.
- **Frame pacing**: `FramepacePlugin` is enabled by default; adjust settings in `systems::setup::set_framespace` if you need different frame caps.

## Testing

Even though the game currently has limited automated coverage, keep the suite green:

```bash
make test          # workspace tests with all features
test-single TEST=path::to::case  # focus on a particular test
make test-watch    # rerun tests automatically as you edit
```

Use `cargo test -- --nocapture` to see stdout and `-- --test-threads=1` to debug ordering-sensitive cases.

Refer to `CONTRIBUTING.md` for testing philosophy and expectations when you add new coverage.

## Working with Stage & Cutscene Data

- Stage files (`assets/stages/*.sg.ron`) define enemy spawns, cinematic steps, and timing. Keep structs in sync with `stage::data` definitions.
- Cutscene files (`assets/cinematics/*.cs.ron`) use the same RON format consumed by `cutscene::data`.
- The scene editor automatically reloads assets when the watcher sees valid changes; otherwise, fix parsing errors reported in the terminal.

## Troubleshooting

- **`cargo watch` not found**: `cargo install cargo-watch`.
- **`bevy` CLI not found**: Install it via `cargo install bevy_cli --locked` and make sure `$CARGO_HOME/bin` is on your `PATH`.
- **Asset parse errors**: Run `make watch-scene-files` to see the failing path and RON error context.
- **Wasm build failures**: Ensure `wasm-bindgen-cli`, `wasm-opt`, and the `wasm32-unknown-unknown` target are installed (`make install-web-deps`).
- **Stale artifacts after pulling**: `cargo clean && cargo build` resets the workspace.

## Helpful Paths

- Game entry point: `src/main.rs`
- Shared systems: `src/systems/`
- Stage logic: `src/stage/`
- Cutscene logic: `src/cutscene/`
- Asset directory: `assets/`

Pair this guide with `CONTRIBUTING.md` to align on reviews and quality expectations.
