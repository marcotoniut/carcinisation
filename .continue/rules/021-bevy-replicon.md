# Bevy Replicon Notes

Only apply if you’re touching networking code.

- Keep Replicon versions in lock-step with the target Bevy release.
- Verify compatibility flags (`server`, `client`, `transport`) before enabling new features.
- Prefer opt-in feature flags so single-player builds stay lean.
- Run `cargo tree -p bevy_replicon` if dependency moves are required; keep the lockfile in sync with the rest of the workspace.
