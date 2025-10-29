# Contributing to Level Editor

This document focuses on how we collaborate and keep the project healthy. The same expectations apply whether you are contributing manually or through an automation/agent workflow.

## Definition of Done

Before opening a pull request:

1. `pnpm lint`, `pnpm test`, and `pnpm build` succeed locally. Apply Biome fixes with `pnpm lint:fix` or `pnpm format` when needed.
2. Update or add documentation and comments affected by your change (component docs, contributor guidance, etc.).
3. Avoid introducing new `TODO` comments—file an issue instead if follow-up work is needed.
4. Smoke test the editor in `pnpm dev` (or `make dev-stage-editor`) when you touch load/save flows, viewport rendering, or other interactive features.

## Local Quality Checks

### Format

```bash
pnpm format
```

Runs Biome in write mode across `src/`. Use this when you expect formatting changes.

### Lint

```bash
pnpm lint
```

Biome static analysis. Auto-fix style issues with `pnpm lint:fix`.

### Tests

```bash
pnpm test
```

Runs the Vitest suite.

### Build / Type Safety

```bash
pnpm build
```

Runs `tsc` followed by the Vite production build. This catches type errors that linting will not.

### Full Suite

```bash
pnpm lint && pnpm test && pnpm build
```

`make ci-stage-editor` runs the same checks from the repository root.

## Workflow

1. Fork the repository and clone your fork.
2. Create a focused feature branch (`git checkout -b feature/descriptive-name`).
3. Implement and self-review the change, running the quality checks above.
4. Keep commits cohesive and reference related issues when possible.
5. Open a pull request and include context, screenshots, or repro steps that help reviewers verify the change quickly.

Automation and AI agents should follow the same workflow—ensure the branch history, PR description, and test evidence remain human-reviewable.

### Commit Guidelines

**Titles**: ≤50 chars, imperative mood, no trailing period.

```
✅ add hud animation for health pickup
✅ fix stage transition timing jitter
❌ Added some animation stuff and fixed a few things.
```

**Messages**: Explain _why_ the change matters—the diff already shows _what_ changed.

```
✅ gate cutscene input behind active state

Prevents menus from consuming input once gameplay resumes.

❌ gate cutscene input behind active state

Added run_if to cutscene update and modified GamePlugin schedule...
```

### Inline Comments

- Clarify complex data flow, styling trade-offs, or interactions with third-party APIs.
- Justify non-obvious math, map projections, or timing tweaks.
- Reference issues or documentation for intentional deviations from conventions.

### Logging

- `console.debug` for verbose development-only details (disabled in production builds).
- `console.info` for major lifecycle events (loading assets, navigation transitions).
- `console.warn` for recoverable anomalies.
- `console.error` for failures that need follow-up or bug reports.

### Data & Type Definitions

- `src/types/generated.d.ts` is generated from the Rust data model and is not tracked in git. Run the Rust codegen pipeline (currently `cargo run --bin codegen` from the sidecar workspace) before development so the file exists, and regenerate it whenever schemas change.
- `src/state/store.ts` and `src/utils/fileSystem.ts` are the source of truth for loaded file content—keep their contracts aligned when adding new features.
- Sample stage files live under `assets/stages/` in the repo root. Use them for manual testing but do not commit edited copies.

## Questions?

Open an issue or start a discussion before large refactors or architectural changes. We are happy to help scope the work and share context.
