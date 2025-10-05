# Development Guide

Commands and workflows for building and testing Carcinisation.

## Quick Start

```bash
# Rebuild and rerun automatically on code changes (requires cargo-watch)
make dev

# Run once with the default feature set
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
make dev             # cargo watch -x run with dynamic linking enabled
make run             # cargo run with RUST_BACKTRACE=full
make dev-wasm        # cargo run --target wasm32-unknown-unknown
make launch-editor   # Open the Bevy-based scene editor
make watch-scene-files # Watch assets/ for RON parsing errors
make generate-palettes # Rebuild palette assets
make generate-typeface # Rebuild font/typeface assets
make process-gfx     # Run graphics processing scripts
make install-web-deps # Install wasm toolchain dependencies
make build-web       # Produce release wasm build + bindings via make-web.sh
make release-wasm    # Release wasm binary + wasm-opt pass
make fmt             # cargo fmt --all
make lint            # cargo clippy with -D warnings
make test            # cargo test --workspace --all-features
make test-watch      # Re-run tests on change via cargo-watch
```

Override feature flags or runtime arguments when you call make:

```bash
# Run without dynamic linking
make run FEATURES=""

# Pass additional CLI args to Bevy (if/when added)
make run ARGS="--help"
```

## Hot Reloading & Iteration

- **Code**: `make dev` recompiles and relaunches whenever source files change. The dynamic linking feature keeps rebuild times low in debug.
- **Assets**: Bevy reloads assets in `assets/` automatically while the game is running. The scene watcher (`make watch-scene-files`) prints parser errors for `.ron` files so you spot mistakes early.
- **Stages & Cutscenes**: Stage data lives under `assets/stages/*.sg.ron`; cutscenes live under `assets/cinematics/*.cs.ron`. Keep Bevy running in `make dev` and run the watcher to validate edits live.

## Tooling

### Scene Editor

Run `make launch-editor` to open the standalone editor in `tools/editor`. It loads assets from the main game's `assets/` directory and lets you inspect or tweak stage and cutscene data with EGUI tools.

### Scene File Watcher

`make watch-scene-files` starts the watcher in `tools/scene-file-watcher`, which continuously parses `.ron` assets under `assets/`. Use it alongside your editor to catch syntax errors before reloading in-game.

### Asset Generators

- `make generate-palettes` rebuilds color palettes under `assets/palette/`.
- `make generate-typeface` recreates bitmap typefaces in `assets/typeface/`.
- `make process-gfx` runs the graphics processing pipeline for sprites and UI assets.

These scripts expect the Rust toolchain; install dependencies where each README indicates.

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

- **Editor overlay**: In debug builds (`cargo run`), the game injects `bevy_editor_pls`. Use its default shortcut (`Ctrl+Shift+P`) to toggle the inspector overlay.
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
