# Agent Protocol

## Startup

Discover tools/versions/workflows from repo files. Do not assume.

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

For code changes, pass all of:

- `make fmt`
- `make lint`
- `make test`

Then run any additional surface-specific checks from project docs.
If something is blocked, state exactly what and why.

## Review Output Format

- Must fix
- Should fix
- Nice to have
- Approval status: Approved or Not approved

## Hygiene

- Stop any watchers/dev servers/helper processes you started.
- Do not leave background processes running unless requested.

## Escalate When Unclear

Escalate instead of guessing when behaviour, architecture, boundaries, compatibility, or validation expectations are unclear.
Provide options, trade-offs, and a recommendation.

## Communication

- Be direct and concise.
- State assumptions explicitly.
- Ask clarifying questions when ambiguity would change behaviour or scope.
- Use precise file references when possible.

## Failure

- Prefer explicit failure over silent fallback.
- Do not hide missing required inputs with no-op guards.
