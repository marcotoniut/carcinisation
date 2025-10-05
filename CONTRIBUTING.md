# Contributing to Carcinisation

Thanks for helping us grow Carcinisation. These guardrails keep the project stable while letting us move fast—please follow them whenever you add or change code.

## Bevy 0.14 Guardrails

- **API version**: Stick with Bevy 0.14.x (and the features enabled in `Cargo.toml`). Reach out before using APIs that require 0.15+ or extra features.
- **System documentation**: Tag every new or modified system with `/// @system …` (or `/// @trigger …` for event-driven systems). Older modules are still being backfilled—extend the convention when you touch them.
- **State-driven scheduling**: Schedule systems with `.run_if(...)` or plugin states whenever a system should only run in particular phases (see `src/stage`, `src/game`, `src/main_menu`).
- **Pixel pipeline**: Use the existing `seldom_pixel` bundles (`PxSpriteBundle`, `PxAssets`, `PxSubPosition`, etc.) instead of hand-building Bevy sprite bundles.
- **Language**: Use American English in code, comments, commit messages, and documentation (`color`, `behavior`, `center`).

## Definition of Done

Before opening a pull request:

1. **`make fmt && make lint && make test`** succeed locally. Fix every clippy warning instead of suppressing it.
2. **Preserve or extend system docs**. Keep existing `/// @system` / `/// @trigger` comments, and add them for any new systems.
3. **Declare run conditions**. New systems must specify when they run (state/resource checks, schedules, etc.).
4. **No new TODOs**. Prefer filing an issue if work must be deferred.
5. **Document ordering**. If system ordering matters, add a one-line comment explaining why.
6. **Assets stay organized**. Place new runtime assets in `assets/` and editor-only data under the relevant tool directory.

## Code Quality Standards

### Formatting

```bash
make fmt
```

### Linting

```bash
make lint
```

### Testing

```bash
make test
```

### Full Check

```bash
make fmt && make lint && make test
```

## Workflow

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/descriptive-name`).
3. Build your change following the guardrails above.
4. Run `make fmt && make lint && make test` before every push.
5. Commit with short, descriptive messages (see below).
6. Open a pull request and fill out the checklist.

Use `make help` for an overview of the available developer commands (`make dev`, `make dev-wasm`, asset generators, etc.).

### Commit Guidelines

**Titles**: ≤50 chars, imperative mood, no trailing period.

```
✅ add hud animation for health pickup
✅ fix stage transition timing jitter
❌ Added some animation stuff and fixed a few things.
```

**Messages**: Explain *why* the change matters—the diff already shows *what* changed.

```
✅ gate cutscene input behind active state

Prevents menus from consuming input once gameplay resumes.

❌ gate cutscene input behind active state

Added run_if to cutscene update and modified GamePlugin schedule...
```

## Testing Philosophy

### Prefer End-to-End Coverage

- **Avoid mocks/stubs**: Spin up Bevy `App` instances and use real plugins/resources.
- **Test full flows**: Exercise systems together (e.g., spawning entities, running schedules, asserting world state).
- **Assert observable behavior**: Validate component/resource changes instead of implementation details.
- **Fake only I/O**: Stub filesystem/network only when unavoidable; prefer using in-memory resources.

### Examples

```rust
// ✅ Good: runs the actual system over real components (inside crate tests)
#[test]
fn updates_pixel_position_from_targeting_components() {
    use bevy::prelude::*;
    use crate::plugins::movement::linear::components::TargetingPositionX;
    use crate::systems::movement::update_position_x;
    use seldom_pixel::prelude::PxSubPosition;

    let mut app = App::new();
    app.add_systems(Update, update_position_x);

    let entity = app.world_mut().spawn((
        TargetingPositionX(42.0),
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

### Known Coverage Gaps

- No automated regression tests exist yet for cutscene sequencing or stage progression. Adding `App`-based tests around these flows is highly encouraged.
- wasm/web builds are not exercised in CI; please test manually before landing changes that touch wasm-specific code.

## Documentation Conventions

### Module Documentation

Start new modules with a brief `//!` summary of their purpose.

```rust
//! Cutscene progression logic and supporting resources.
```

### Bundle Documentation

Document bundles with `///` comments highlighting their intent and any non-obvious fields.

```rust
/// Player avatar data for the pixel-rendered stage.
#[derive(Bundle)]
pub struct PlayerBundle {
    /// Marks the entity for cleanup when leaving a stage.
    pub on_stage_scene: OnStageScene,
    /// Required by seldom_pixel to pin the sprite to the pixel grid.
    pub sprite: PxSpriteBundle<Layer>,
    // ...
}
```

### System Documentation

Use the tags introduced above for systems and triggers.

```rust
/// @system Syncs the pixel camera with the tracked player position.
pub fn move_camera(/* ... */) { }

/// @trigger Resets volume settings when the stage restarts.
pub fn reset_volume_on_stage_restart(/* ... */) { }
```

### Inline Comments

- Clarify complex component relationships or scheduling requirements.
- Justify non-obvious math, pixel-grid adjustments, or resource lifetimes.
- Reference issues for intentional deviations from conventions.

### Logging

- `log::debug!` for verbose development information (disabled in release).
- `log::info!` for major state changes (loading assets, transitions, achievements).
- `log::warn!` for recoverable anomalies.
- `log::error!` for critical failures that require follow-up.

## Questions?

Open an issue before starting large refactors or architectural changes—we’re happy to discuss approaches and split work.
