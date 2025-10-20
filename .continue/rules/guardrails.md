# Guardrails

## General

- Target minimal files; include new ones only when necessary
- Preserve behavior unless requested; document intentional changes
- Surface plan before large refactors; prefer iterative, testable steps
- Add `//!` module docs when touching files that lack them (opportunistic)

## Bevy-Specific

- Keep WASM-safe: `bevy/dynamic_linking` stays native-only
- Scoped diffs during upgrades: one subsystem at a time
- Compile before chasing idioms

## Paths

- MUST use relative paths (`src/main.rs`), NEVER absolute (`/src/main.rs`)
- NEVER use leading slashes (causes errors)

## Discovery Workflow

1. Find: `file_glob_search` or `grep_search`
2. Verify: non-empty results
3. Read: `read_file` on candidates
4. Analyze: Bevy/Rust Docs MCP if needed
5. Propose: minimal diffs

## MCP Tool Priority

- **Web tasks:** ALWAYS use Browser MCP (never built-in web tools)
- **Repo shell:** Prefer Base MCP (sandboxed) over built-in Bash
- **File ops:** Continue built-ins (`read_file`, `grep_search`) for quick access
