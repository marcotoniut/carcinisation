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
# Watch and rebuild automatically via bacon (faster and more reliable than cargo-watch)
make dev

# Run once with the default feature set via `bevy run --bin carcinisation`
make run

# Run pre-built debug binary directly (useful for IDE debuggers)
make debug-binary

# Skip menus/cutscenes and boot directly into a configured stage
make dev-stage

# Launch the scene editor utility
make launch-editor

# Validate RON asset changes while editing
make watch-scene-files
```

This project uses [bacon](https://dystroy.org/bacon/) instead of cargo-watch for all watch workflows. Install it with:

```bash
cargo install bacon --locked
```

## Make Targets Overview

Use `make help` to see the full catalog. Common targets:

```bash
make dev               # Watch and rebuild via bacon (watches apps/carcinisation/src and assets)
make run               # bevy run --bin carcinisation (RUST_BACKTRACE=full)
make debug-binary      # Run pre-built target/debug/carcinisation with proper DYLD_LIBRARY_PATH
make build-and-run     # Build once, then run via wrapper script (faster for repeated runs)
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
make test-watch        # Re-run tests on change via bacon
```

Override feature flags or runtime arguments when you call make:

```bash
# Run without dynamic linking
make run FEATURES=""

# Pass additional CLI args to Bevy (if/when added)
make run ARGS="--help"
```

## Hot Reloading & Iteration

- **Code + Assets**: `make dev` uses `bacon` to watch and rerun when files under `apps/carcinisation/src` or `assets/` change. Bacon is faster and more reliable than cargo-watch, with better error reporting.
- **Assets**: Bevy reloads assets in `assets/` automatically while the game is running. The scene watcher (`make watch-scene-files`) prints parser errors for `.ron` files so you spot mistakes early.
- **Stages & Cutscenes**: Stage data lives under `assets/stages/*.sg.ron`; cutscenes live under `assets/cinematics/*.cs.ron`. Keep Bevy running in `make dev` and run the watcher to validate edits live.
- **Single-stage debug flow**: `make dev-stage` copies `apps/carcinisation/single-stage.config.default.ron` to `apps/carcinisation/single-stage.config.ron` (if needed), then launches the dedicated `single_stage` binary. Edit that file to point at any `.sg.ron` you want to boot directly, skipping menus and cutscenes entirely.

### Local Documentation

- API docs for every crate in the workspace (and their dependencies) are generated via `scripts/generate-docs.sh`. The output lives under `target/doc` (already gitignored). Optional flags:

  ```bash
  scripts/generate-docs.sh           # build docs
  DOCS_OFFLINE=1 scripts/generate-docs.sh  # reuse cached deps, no network
  scripts/generate-docs.sh --serve   # build then serve at http://localhost:7878
  ```

- For the standard Rust documentation run `rustup doc`.

### Dynamic Linking on macOS

The project uses Bevy's `dynamic_linking` feature for faster compile times. On macOS, the dynamic linker (`dyld`) needs help finding the shared libraries:

- Rust's standard library dylibs (from `rustc --print target-libdir`)
- Bevy's dylibs (from `target/debug/deps`)

**When to use the wrapper script:**

- **IDE debuggers (Zed, VS Code)**: Use `scripts/debug-carcinisation.sh` or `make debug-binary`.
- **Manual testing**: Use `make build-and-run` to build once and reuse the binary without paying cargo's startup overhead.
- **Via bacon/cargo**: `make dev` and `make run` handle `DYLD_LIBRARY_PATH` automatically, so no extra steps.

The wrapper script (`scripts/debug-carcinisation.sh`) is the canonical way to run the debug binary outside of cargo/bevy. It’s referenced by `.zed/debug.json` so Zed users get correct env vars without hardcoding the platform dynamic-loader path.

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

   Agents can also run `pnpm check:agent --all` to execute the standard checks with captured logs under `reports/agent/`.

4. Open a pull request following `CONTRIBUTING.md`.

## Debugging Tips

- **Backtraces**: `make run` already sets `RUST_BACKTRACE=full`. Combine with `RUST_LOG=debug` to see verbose logs.

  ```bash
  RUST_LOG=debug make run
  ```

- **Editor overlay**: In debug builds (`make run` / `bevy run --bin carcinisation`), the game injects `bevy-inspector-egui`. The inspector overlay is available through the EGUI context when running in debug mode.
- **Diagnostics**: The debug plugin draws helper overlays when its state is active. Check `src/debug` for details and extend as needed.
- **Frame pacing**: `FramepacePlugin` is enabled by default; adjust settings in `apps/carcinisation/src/systems/setup.rs` (`set_framespace`) if you need different frame caps.

## Testing

Even though the game currently has limited automated coverage, keep the suite green:

```bash
make test          # workspace tests with all features
make test-single TEST=path::to::case  # focus on a particular test
make test-watch    # rerun tests automatically as you edit
```

Use `cargo test -- --nocapture` to see stdout and `-- --test-threads=1` to debug ordering-sensitive cases.

Refer to `CONTRIBUTING.md` for testing philosophy and expectations when you add new coverage.

## Working with Stage & Cutscene Data

- Stage files (`assets/stages/*.sg.ron`) define enemy spawns, cinematic steps, and timing. Keep structs in sync with `stage::data` definitions.
- Cutscene files (`assets/cinematics/*.cs.ron`) use the same RON format consumed by `cutscene::data`.
- The scene editor automatically reloads assets when the watcher sees valid changes; otherwise, fix parsing errors reported in the terminal.

## Troubleshooting

- **`bacon` not found**: `cargo install bacon --locked`. This project uses bacon instead of cargo-watch.
- **`bevy` CLI not found**: Install it via `cargo install bevy_cli --locked` and make sure `$CARGO_HOME/bin` is on your `PATH`.
- **dyld errors on macOS**: Use `make debug-binary` (or `scripts/debug-carcinisation.sh`) so `DYLD_LIBRARY_PATH` includes Rust + Bevy dylibs.
- **Asset parse errors**: Run `make watch-scene-files` to see the failing path and RON error context.
- **Wasm build failures**: Ensure `wasm-bindgen-cli`, `wasm-opt`, and the `wasm32-unknown-unknown` target are installed (`make install-web-deps`).
- **Stale artifacts after pulling**: `cargo clean && cargo build` resets the workspace.

## Helpful Paths

- Game entry point: `apps/carcinisation/src/main.rs`
- Shared systems: `apps/carcinisation/src/systems/`
- Stage logic: `apps/carcinisation/src/stage/`
- Cutscene logic: `apps/carcinisation/src/cutscene/`
- Asset directory: `assets/`

Pair this guide with `CONTRIBUTING.md` to align on reviews and quality expectations.
