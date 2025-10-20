# Carcinisation — Project Guide

**Origin:** GBJam 11 | **Engine:** Bevy 0.15 → 0.17 | **Primary:** `apps/carcinisation`

## Workspace

- **Game:** `apps/carcinisation/` (main binary)
- **Tools:** `tools/editor/`, `tools/scene-file-watcher/`
- **Assets:** `assets/` (RON, TOML, art)
- **Scripts:** `scripts/` (Rust bins for asset preprocessing)

## Build & Run

**Primary:** `make run | dev | dev-wasm | check | lint | fmt | test`

**Env vars:** `RUN_BIN`, `RUN_PACKAGE`, `FEATURES` (native: `bevy/dynamic_linking`), `WASM_FEATURES` (empty), `ARGS`

**WASM:** NEVER use `bevy/dynamic_linking` for `wasm32-unknown-unknown`

## MCP Servers (Local Assistants)

Run: `make run-mcp-{base|bevy|rust-docs|scribe}` | Build: `make build-mcp-all`

- **Base:** Sandboxed shell (`run_shell`, `env_info`)
- **Bevy:** `bevy_version`, `find_bevy_system_like_fns`
- **Rust Docs:** `list_rust_files`, `find_module_doc_gaps`, `cargo_doc`, `make`
- **Scribe:** `draft_commit`, `draft_pr`, `summarize`
- **Browser:** Playwright automation (pnpm)

## Bevy Upgrades

**Strategy:** Compile first (minimal diffs) → behavior → style
**Order:** Update `Cargo.toml` → `cargo update -p bevy` → `make check` → triage → fix → re-check
**Replicon:** Keep in lock-step with Bevy version

## Dependencies

Use Cargo (`cargo add|update|tree`), NEVER pip. Reference exact names from manifest. Keep workspace `Cargo.lock` consistent.

## Testing

`make test` (workspace) | `make test-single TEST=path::to::case` | Keep tests headless (no GPU/window)
