# Agent Protocol

## Startup

Discover tools/versions/workflows from repo files. Do not assume.
Before choosing validation, read `.direnv/cache/check-agent-capabilities.json`.
If it is missing or stale, regenerate it with `pnpm check:agent:generate`.

## Ownership

- One driver edits files at a time.
- Support agents read/review only.
- Handoff is required before changing driver.

## Stage Loop

1. Scope: goal, owner, touched surfaces.
2. Implement: small, focused diffs.
3. Validate: run required checks for touched surfaces.
4. Handoff: include the required block below.
5. Review: reviewer decides approval.

## Required Handoff Block

- Scope touched
- Files changed
- Validation run
- Validation not run (with reason)
- Risks and edge cases
- Next expected owner

## Validation

Use `pnpm check:agent` flags from `.direnv/cache/check-agent-capabilities.json`.
For broad code changes, run the default flags from that file.
If a check fails, open the matching focus file in `reports/agent/` first.
Re-run `pnpm check:agent` with the same flags after fixing issues.
Run any additional surface-specific checks from project docs when needed.
If something is blocked, state exactly what and why.

## Review Output Format

- Must fix
- Should fix
- Nice to have
- Approval status: Approved or Not approved

## Commits

- Do not add Co-Authored-By lines to commits.

## Artifacts

- Planning docs, investigation notes, and build reports go in `tmp/`, never in tracked source directories.
- Do not copy pipeline-only outputs (e.g. `analysis.json`) into `assets/`.
- `tmp/` is gitignored; treat it as ephemeral.

## Hygiene

- Stop any watchers/dev servers/helper processes you started.
- Do not leave background processes running unless requested.

## Escalate When Unclear

Escalate instead of guessing when behaviour, architecture, boundaries, compatibility, or validation expectations are unclear.
Provide options, trade-offs, and a recommendation.

## Communication

- Default to `caveman`.
- Use normal style when brevity risks misunderstanding.
- State assumptions explicitly.
- Ask clarifying questions when ambiguity would change behaviour or scope.
- Use precise file references when possible.

## Failure

- Prefer explicit failure over silent fallback.
- Do not hide missing required inputs with no-op guards.
