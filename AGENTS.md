# Agent Coordination Protocol

This file is the shared operating contract for local agents in this repository.
Agent prompts should stay short and role-specific. Coordination rules live here.

## Mandatory Startup

Before doing any work:

1. Read this file.
2. Read `README.md`, `DEVELOPMENT.md`, and `CONTRIBUTING.md`.
3. Discover versions, tools, workflows, and constraints from repository files. Do not assume them.

## Role Boundaries

Use explicit ownership per task to avoid overlap.

- Runtime agent: gameplay systems, ECS logic, plugin wiring, runtime behavior.
- Pipeline agent: build/web packaging, asset generation, scripts, tooling integration.
- Reviewer agent: correctness, risk, tests, maintainability, boundary enforcement.
- Design agent: mechanics, system mapping, phased delivery plans.

If a task crosses boundaries, split it into stages and assign one owner per stage.

## Single Driver Rule

Only one agent edits code at a time.

- Driver agent: the only agent that writes files for the current stage.
- Support agents: read, analyze, and review only.
- Handoff is required before another agent becomes driver.

## Fast Execution Pattern

For each stage:

1. Scope: define goal, owner, and touched surfaces.
2. Implement: keep diffs narrow and avoid unrelated refactors.
3. Validate: run repository-standard checks relevant to touched surfaces.
4. Handoff: provide the required handoff block.
5. Review: reviewer agent decides whether stage is ready.

Keep stages small enough for focused review and safe rollback.

## Required Handoff Block

Every agent handoff must include:

- Scope touched
- Files changed
- Validation run
- Validation not run (with reason)
- Risks and edge cases
- Next expected owner

No handoff is complete without this block.

## Review Gate

Reviewer output must be structured as:

- Must fix
- Should fix
- Nice to have
- Approval status: Approved or Not approved

Findings must be concrete and actionable, with file references.

## Validation Policy

Run baseline checks plus surface-specific checks from project docs.

Before marking work complete (for code changes), `make fmt`, `make lint`, and `make test` must succeed with no ignored warnings.

Then run additional checks relevant to changed surfaces, as documented in repository docs:

- Runtime behavior changes: run relevant runtime validation and document manual verification.
- Web/editor/tooling changes: run relevant web/tool checks for those paths.
- Asset/format/pipeline changes: run the corresponding generation and validation flows.
- User-visible web behavior changes: validate the changed flow with Playwright MCP.

If checks are blocked, state exactly what is blocked and why.

## Process Hygiene

- Stop watchers, dev servers, and helper processes you started before handoff.
- Do not leave background processes running unless explicitly requested.
- Keep logs concise and focused on actionable failures.

## Escalation Rules

Escalate instead of guessing when:

- Architecture or scheduling changes are ambiguous.
- Runtime/tooling boundaries are unclear.
- A format or pipeline contract may break compatibility.
- Validation cannot be completed with current access.

When escalating, provide options with trade-offs and a recommendation.

## Communication Rules

- Be direct, factual, and concise.
- State assumptions explicitly.
- Ask clarifying questions when requirements are ambiguous or multiple valid approaches would change behavior, architecture, or scope.
- Preserve prior decisions unless new evidence requires change.
- Use precise file references with line numbers when possible.

## Failure Policy

- Prefer explicit failure over silent fallback when required coordination inputs are missing.
- Do not mask missing core files or broken startup assumptions with defensive no-op guards.
- Treat startup hook failures as actionable setup issues and resolve them before proceeding.
