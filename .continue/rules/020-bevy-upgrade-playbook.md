# Bevy Upgrade Playbook

Lean on this guide whenever Bevy needs a version bump (for example, 0.15 → 0.16 or 0.17). For day-to-day feature work, follow the general guardrails instead.

## Principles

1. Keep the workspace compiling before chasing idiomatic cleanups.
2. Preserve behavior first; refactors come after the build is green.
3. Prefer scoped diffs—tackle one subsystem or crate at a time.

## Pre-flight

- Review Bevy release notes and migration docs for the target version.
- Check `Cargo.toml` and `Cargo.lock` for direct and indirect Bevy dependencies (plugins, editor tooling, replicon).
- Note custom features (`dynamic_linking`, `wayland`, wasm-related flags) so you can confirm they still exist.

## Execution Checklist

- Update the Bevy version in `[workspace.dependencies]` and any crate-specific overrides.
- Run `cargo update -p bevy` (or targeted packages) to refresh the lock file.
- Build once (`make check`) and triage errors by crate or subsystem.
- For every error:
  - Locate the new API path or helper.
  - Apply the smallest viable edit (rename import, tweak signature, adjust feature gate).
  - Keep temporary compatibility shims close to the call site with comments.
- Re-run `make check` regularly; keep the diffs reviewable.

## Post-pass

- `make test` to exercise the workspace (game, tools, scripts).
- Sanity run the game (`make run`) and critical tooling (`make launch-editor`) to spot runtime regressions.
- Document notable API changes in commit messages or follow-up docs so future upgrades have a breadcrumb trail.
