# Contributing to Carcinisation

Thank you for helping us evolve Carcinisation. This guide gathers the guardrails, workflows, and quality checks you need to keep the project stable while shipping fast.

## Quick Links

- Project overview & setup – `README.md`
- Command catalogue & tooling – `DEVELOPMENT.md`
- Multi-agent coordination – `AGENTS.md`

## Before You Start

Confirm your environment matches the prerequisites in `README.md` and review the make targets in `DEVELOPMENT.md` for the surfaces your work will touch.

## Project Guardrails

- **Bevy version** – Stay on Bevy 0.17.x with the feature set declared in `Cargo.toml`. Discuss upgrades before introducing new APIs or feature gates.
- **System documentation** – Maintain the `/// @system` / `/// @trigger` annotations and add run-condition notes whenever you modify or create systems.
- **Scheduling** – Use `.run_if(...)`, states, or dedicated schedules for conditional execution (see `src/stage`, `src/game`, `src/main_menu`).
- **Pixel pipeline** – Reuse the `seldom_pixel` bundles (`PxSpriteBundle`, `PxAssets`, `PxSubPosition`, etc.) instead of hand-rolling sprite data.
- **Language** – Use American English in code, commits, and documentation (`color`, `behavior`, `center`).
- **Assets** – Runtime assets belong under `assets/`; editor-only data lives with its owning tool in `tools/`.

## Workflow

See `DEVELOPMENT.md` for the full command catalogue. The short version:

1. **Branch** – Sync `main` and create a feature branch.
2. **Iterate** – Use `make dev` and the tooling targets in `DEVELOPMENT.md`.
3. **Validate** – `make fmt && make lint && make test` before each push; add surface-specific checks (wasm, assets, `pnpm lint`) as needed.
4. **Summarise** – Capture context, validation output, and follow-ups in your pull request description.
5. **Review** – Preserve existing behaviour unless you document the rationale and verification for changes.

## Definition of Done

- Quality gates pass locally with no ignored warnings.
- System docs and run-condition notes remain accurate for every modified gameplay system.
- New systems declare when they execute (states, resources, schedules).
- No new unchecked `TODO`s—open a GitHub issue and link it if the note must stay near the code.
- Ordering constraints include a short comment explaining why the order matters.
- Assets follow the directory conventions above.

## Commit Guidelines

- **Titles** – ≤50 characters, imperative mood, no trailing period.

  ```
  ✅ add hud animation for health pickup
  ✅ fix stage transition timing jitter
  ❌ Added some animation stuff and fixed a few things.
  ```

- **Bodies** – Explain _why_ the change matters. The diff already shows _what_ changed.

  ```
  ✅ gate cutscene input behind active state

  Prevents menus from consuming input once gameplay resumes.

  ❌ gate cutscene input behind active state

  Added run_if to cutscene update and modified GamePlugin schedule...
  ```

## Testing Philosophy

### Prefer End-to-End Coverage

- Spin up real Bevy `App` instances and use production plugins/resources.
- Exercise systems together (spawn entities, run schedules, assert world state).
- Assert observable behaviour instead of implementation details.
- Stub filesystem/network only when unavoidable; prefer in-memory substitutes.

### Examples

```rust
// ✅ Good: runs the actual system over real components (inside crate tests)
#[test]
fn updates_pixel_position_from_targeting_components() {
    use bevy::prelude::*;
    use cween::linear::components::TargetingValueX;
    use seldom_pixel::prelude::PxSubPosition;
    use crate::systems::movement::update_position_x;

    let mut app = App::new();
    app.add_systems(Update, update_position_x);

    let entity = app.world_mut().spawn((
        TargetingValueX(42.0),
        PxSubPosition(Vec2::ZERO),
    )).id();

    app.update();

    let pos = app.world().get::<PxSubPosition>(entity).unwrap();
    assert_eq!(pos.0.x, 42.0);
}

// ❌ Avoid: heavy mocking that never boots Bevy
#[test]
fn update_position_x_with_mock() {
    let mut mock_entity = MockEntity::new();
    mock_entity.expect_set_position_x().with(42.0).times(1);
    // ... brittle expectation setup
}
```

### Known Gaps

- Cutscene sequencing and stage progression lack automated coverage—`App`-level regression tests are welcome.
- Web builds are not exercised in CI; run the wasm targets manually before landing relevant changes.

## Documentation & Logging

- Start new modules with a brief `//!` summary.
- Document bundles with `///` comments that highlight intent and non-obvious fields.
- Keep `/// @system` / `/// @trigger` tags synced with behaviour.
- Inline comments should clarify unusual scheduling, math, or clean-up rules and reference issues when appropriate.
- Use logging levels consistently: `debug!` for verbose diagnostics, `info!` for major state changes, `warn!` for recoverable anomalies, `error!` for required follow-up.

## Questions?

Open an issue or discussion before starting large refactors or architectural changes—we’re happy to plan the work with you.
