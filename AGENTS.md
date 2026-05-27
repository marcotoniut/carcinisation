# Agent Protocol

## Startup

Read repo files for tools, versions, workflows. No guesses.
Read `.direnv/cache/check-agent-capabilities.json` before validation.
If missing or stale, run `pnpm check:agent:generate`.

## Ownership

- One driver edits at a time.
- Support agents read/review only.
- Handoff before driver change.

## Stage Loop

1. Scope: goal, owner, touched surfaces.
2. Implement: small focused diffs.
3. Validate: required checks for touched surfaces.
4. Handoff: include the required block below.
5. Review: reviewer approves or rejects.

## Required Handoff Block

- Scope touched
- Files changed
- Validation run
- Validation not run (with reason)
- Risks and edge cases
- Next expected owner

## Validation

Use `pnpm check:agent` flags from `.direnv/cache/check-agent-capabilities.json`.
`defaultFlags` = baseline broad checks, not every check.
Iteration: prefer `surfaceProfiles.<surface>.quick`.
Before handoff/review: prefer `surfaceProfiles.<surface>.full`.
Run `surfaceProfiles.<surface>.advisory` only when requested or paying down quality debt.
CLI also supports `--surface <surface> --profile <quick|full|advisory>`.
Use `--fail-fast` during iteration when first failure is enough.
For machine output, use `pnpm --silent check:agent ... --json`.
Without `--silent`, `pnpm` adds wrapper lines around JSON.
Use `--list` to inspect checks, defaults, and surface/profile mappings.
If a check fails, open matching `reports/agent/*.focus.txt` first.
After fixes, rerun same flags.
Run any additional surface-specific checks from project docs when needed.
If blocked, state what and why.

## Review Output Format

- Must fix
- Should fix
- Nice to have
- Approval status: Approved or Not approved

## Commits

- Generate docs, comments, commit messages, and PR text with intent: terse, domain-focused, no filler. Expand only when extra context materially improves clarity.
- Do not add Co-Authored-By lines to commits.

## Artifacts

- Planning docs, investigation notes, build reports go in `tmp/`, never tracked source dirs.
- Do not copy pipeline-only outputs (for example `analysis.json`) into `assets/`.
- `tmp/` is gitignored and ephemeral.

## Testing

- Default: `just test` runs `cargo nextest run --workspace --all-features --locked`.
- Fallback: `just test-cargo` runs `cargo test --workspace --all-features`.
- Server integration tests use per-PID port ranges for parallel safety (`.config/nextest.toml`).
- `just test-single <name>` and `just test-watch` use `cargo test` (no nextest equivalent).

### Test Organisation

Where tests live:

- **Inline `#[cfg(test)] mod tests`** is the default for small unit tests that exercise private logic or tight module invariants. Keep these near the implementation.
- **Crate-level `tests/*.rs`** for integration, public API, ECS/app-based, networking, asset-loading, parity, and end-to-end behaviour tests.
- **Shared helpers** go in `tests/common/mod.rs` when used by multiple integration test files in the same crate. Avoid duplicating helpers across files.

Oversized inline test modules (>~150 lines) can be extracted to a sibling file:

```rust
// in src/foo.rs
#[cfg(test)]
mod tests;
// tests live in src/foo/tests.rs
```

Only extract when the module is already being materially edited and the move improves readability. Do not move tests purely for style.

Naming: use descriptive `snake_case` test names. Avoid `_spec.rs` or JS-style layout.

Moving forward:

- New tests follow this convention.
- Existing tests are reorganised only opportunistically when already being materially edited.
- Do not do repo-wide test reshuffles.

### Server Integration Test Conventions

- Use helpers from `tests/common/mod.rs` for player/enemy spawning, health queries, intent injection, and tick loops.
- Prefer `wait_for_server_condition()` with a max-tick guard over blind `for _ in 0..N` loops. Early exit reduces wall time and clarifies intent.
- Comment tick-rate assumptions on non-trivial loops: `// 50 ticks at 2 ms ≈ 3 FixedUpdate cycles at 30 Hz`.
- For client+server tests requiring real networking, use `tick_with_sleep()`. For server-only tests, use `tick_server()`.
- Deterministic `Time<Fixed>` advancement (see `pickup.rs`) is preferred when real networking is not needed.

## Dependency Management

- `cargo add` / `cargo remove` built-in. `cargo upgrade` / `cargo set-version` need `just install-cargo-edit`.
- Add deps to `[workspace.dependencies]` first, then `cargo add --package <pkg>`. Pin versions; avoid wildcards.
- `just check-deny` runs `cargo deny check` for license + advisory CI gating. Install with `just install-cargo-deny`.

### Optional Profiling & Hygiene

- `just install-cargo-machete` — detects unused deps. Use for periodic cleanup. False positives possible with proc macros / build scripts / features.
- `just install-cargo-bloat` — analyses binary/WASM size. Use for optimisation/profiling sessions.
- `just install-cargo-llvm-lines` — analyses per-function LLVM IR line count for compile-time bloat.
- Not required for: `cargo check`, `cargo test`, gameplay iteration, CI build correctness.
- Do not add to `.envrc` auto-install. Do not add to default CI jobs.

## Hygiene

- Stop any watchers/dev servers/helper processes you started.
- Do not leave background processes running unless requested.

## Escalate When Unclear

Escalate instead of guessing on behavior, architecture, boundaries, compatibility, or validation expectations.
Give options, trade-offs, and recommendation.

## Communication

- Default to `caveman`.
- Use normal style when brevity risks confusion.
- State assumptions explicitly.
- Ask when ambiguity changes behavior or scope.
- Use precise file refs when possible.

## Correctness

- Fix root cause, not symptom. Do not patch around design bugs.
- Prefer principled fix over expedient hack.
- Prefer compile-time type safety over runtime validation. Make illegal states unrepresentable.

## Failure

- Prefer explicit failure over silent fallback.
- Do not hide missing required inputs with no-op guards.

## Glossary

See `.agents/glossary.md` for canonical project vocabulary, acronyms, and domain terms.
