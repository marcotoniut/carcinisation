# Carcinisation Agent Guide

Use this guide to keep Codex, Claude, and human maintainers aligned while collaborating in the Carcinisation repository.

## Source of Truth

- `README.md` – workspace overview and setup requirements.
- `DEVELOPMENT.md` – day-to-day commands, make targets, and tooling notes.
- `CONTRIBUTING.md` – project guardrails, quality gates, and pull request expectations.
- `CLAUDE.md` – Claude-focused planning and documentation workflow.

Update the authoritative document (not this summary) whenever guidance changes.

## Roles

- **Codex (GPT-5)** – Owns code edits, runs local tooling, keeps the workspace tidy, and defers to the guardrails in `CONTRIBUTING.md`.
- **Claude** – Leads planning, larger design reviews, and documentation polish per `CLAUDE.md`.
- **Maintainers** – Prioritise work and approve bigger decisions; agents surface blockers instead of guessing.

## Collaboration Flow

1. **Clarify scope** – Restate the request, confirm the target surfaces, list assumptions.
2. **Choose the driver** – Code and build changes default to Codex; exploratory planning and cross-file docs default to Claude unless told otherwise.
3. **Share context** – Track touched files, commands, and open questions in session notes or hand-offs.
4. **Verify when possible** – Prefer `make fmt`, `make lint`, `make test`, `pnpm lint`, and asset scripts from `DEVELOPMENT.md`; log any gaps when tooling cannot run.
5. **Escalate early** – Flag conflicts with `CONTRIBUTING.md` guardrails, missing dependencies, or architecture risks before proceeding.

## Guardrails for Agents

- Follow `CONTRIBUTING.md#project-guardrails` for engine version, system documentation, scheduling, and asset layout requirements.
- Use the documented make targets instead of ad-hoc commands; note deviations and sandbox limitations in the session summary.
- Keep scope tight—offer extra ideas as suggestions and wait for maintainer approval before implementing them.
- Coordinate asset changes (`assets/`) with `make watch-scene-files` and mention affected files during hand-offs.
- Minimise new `TODO`s; when unavoidable, include enough context for a maintainer to act.

## Communication Norms

- Reference files with relative paths and line numbers (for example, ``apps/carcinisation/src/main.rs#L25``).
- Communicate with concise, action-oriented language, surfacing risks or trade-offs by priority.
- Preserve previous agent decisions unless new information or maintainer direction necessitates a change.
- Record verification status and remaining follow-ups in the final response so reviewers know what still needs attention.
