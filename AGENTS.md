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

- Do not add Co-Authored-By lines to commits.

## Artifacts

- Planning docs, investigation notes, build reports go in `tmp/`, never tracked source dirs.
- Do not copy pipeline-only outputs (for example `analysis.json`) into `assets/`.
- `tmp/` is gitignored and ephemeral.

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

## Failure

- Prefer explicit failure over silent fallback.
- Do not hide missing required inputs with no-op guards.
