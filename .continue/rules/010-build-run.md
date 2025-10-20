# Build & Run (authoritative: Makefile)

## Primary targets

- `make run` — native run (`cargo run`) with env passthrough.
- `make dev` — auto-restart via `cargo watch`.
- `make dev-wasm` — run for `wasm32-unknown-unknown`.
- `make release-wasm` — release build + `wasm-opt`.
- Quality: `make check | fmt | lint | test | test-watch | fix`.
- Tools: `make launch-editor | watch-scene-files`.
- Assets: `make generate-palettes | generate-typeface | process-gfx`.
- Web: `make install-web-deps | build-web`.

## MCP agents (local assistants)

- Primary entry points:
  - `make run-mcp-base`
  - `make run-mcp-bevy`
  - `make run-mcp-rust-docs`
  - `make run-mcp-scribe`
  - Shortcut: `make run-mcp-server` → same as `run-mcp-base`.
- Build the containers with `make build-mcp-base`, `make build-mcp-bevy`, `make build-mcp-rust-docs`, `make build-mcp-scribe`, or `make build-mcp-all`.
- Each target shells out to `./dev/mcp/<server>/run.sh`, which prefers Docker. If Docker is unavailable it will fall back to a local virtualenv inside the same folder; delete the resulting `.venv/` if you want to force Docker-only runs.
- Continue: choose the matching configuration from `.continue/mcpServers/{base,bevy,rust-docs,scribe}.yaml`.

## Env vars (forwarded by the Makefile)

- `RUN_BIN` (default: `carcinisation`)
- `RUN_PACKAGE` (default: `carcinisation`)
- `FEATURES` (default: `bevy/dynamic_linking` for native)
- `WASM_FEATURES` (empty by default; set explicitly for wasm)
- `ARGS` (forwarded after `--` to the game)

### Examples

- Run stage 2:
  `RUN_BIN=carcinisation ARGS="--level stage2" make run`
- Native with no dynamic linking:
  `FEATURES= make run`
- WASM with custom features:
  `WASM_FEATURES="some_feature" make dev-wasm`

## WASM gotcha

`bevy/dynamic_linking` is **not** WASM-safe. Clear `FEATURES` or keep it to native only.

## Workflow tips

- Run `make fmt` before committing Rust changes; the workspace enforces `rustfmt`.
- Use `make check` after larger edits to catch clippy + compile issues in one pass.
- When iterating on tools (`tools/editor`), rely on the existing Make targets instead of ad-hoc scripts so env vars stay consistent.
