# Testing & CI

- `make test` runs the full workspace suite (game + tools + scripts).
- Single test: `make test-single TEST=path::to::case` (agent may substitute).
- Add or update tests alongside feature work; favor integration coverage for Bevy systems.
- Keep tests runnable in headless envs (no GPU/window required).
