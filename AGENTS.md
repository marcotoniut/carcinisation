# AI Agent Handbook

This repository supports a small team of AI assistants collaborating on Carcinisation’s runtime, tooling, and documentation.
Use this guide to coordinate work, ensure smooth hand-offs, and maintain consistent quality across contributions.

⸻

## Purpose

- Define how agents collaborate effectively within the Carcinisation workspace.
- Clarify shared responsibilities, strengths, and blind spots.
- Prevent duplicate effort, conflicting edits, or divergent reasoning.
- Establish a shared process for verification, asset handling, and hand-offs.

⸻

## Primary Roles

Both agents can work on code, documentation, tooling, or planning. Each brings complementary strengths that align with the guardrails in `CONTRIBUTING.md`.

- **Codex (GPT-5)** – Execution-focused. Excels at precise implementation, structured edits, adherence to `make` workflows, and logical verification.  
  Strengths: accuracy, code quality, predictable delivery, tool-driven validation.  
  Weaknesses: less flexible tone, can lean on literal interpretations without explicit context.

- **Claude** – Concept-driven. Excels at synthesis, planning across files, and polishing documentation using `CLAUDE.md` guidance.  
  Strengths: communication, synthesis, creative framing, structural reasoning.  
  Weaknesses: less deterministic syntax, can drift from strict formatting without prompts.

Agents may swap roles as needed, request reviews from their counterpart, and lean on the other’s strengths when uncertain.

⸻

## Collaboration Flow

1. **Confirm scope**
   Review the task and align it with `README.md`, `DEVELOPMENT.md`, and `CONTRIBUTING.md`. Call out assumptions about game code, editor tooling, or assets up front.
2. **Select the initial driver**
   - Systems code, asset pipeline updates, or guarded make targets → Codex usually leads.
   - Narrative planning, multi-file documentation, or design exploration → Claude usually leads.
     The non-driver reviews conclusions or diffs before completion.
3. **Share context**
   During hand-offs, summarise: touched paths, pending validation (e.g., `make watch-scene-files` for RON assets), open questions, and key trade-offs.
4. **Verify before hand-off**
   Run or reason through the standard checks:
   ```bash
   make fmt
   make lint
   make test
   pnpm lint
   ```
   Add specialised commands when relevant (`make watch-scene-files`, `make launch-editor`, wasm build targets). If execution is blocked, document what remains unverified and why.
5. **Escalate when unsure**
   Pause when work clashes with guardrails (Bevy version, system docs, asset layout) or when architectural decisions need maintainer confirmation.

⸻

## Process cleanup

- Stop background services, dev servers, watchers, or other processes that agents start for development or verification once the task is complete. Use helpers like `KillShell` for `pnpm dev:agents`, `make watch-scene-files`, or equivalent tooling so leftover work does not lock ports or consume CPU.
- Only leave a process running when the maintainer or user explicitly asks for it; record the reason in the task notes.
- Manual tests can be rerun by the user when ready, so do not keep your own servers, watchers, or helper scripts lingering after you hand off the work.
- When removing entities during systems, prefer marking with `DespawnMark`/`mark_for_despawn*` and let shared cleanup run. Avoid direct despawns or component stripping mid-frame unless explicitly required.
- After code changes, run and fix `make fmt`, `make lint`, and `make test` (or note why not).

⸻

## Quality Checks

Before marking work complete:

- ✅ `make fmt`, `make lint`, and `make test` succeed with no ignored warnings.
- ✅ `pnpm lint` passes for web/editor code; include `pnpm typecheck` if TypeScript types are touched.
- ✅ Wasm builds or asset scripts run when the change affects them (`make build-web`, `make watch-scene-files`, palette/typeface generators).
- ✅ Major gameplay, editor, and UI flows behave as expected; document manual test coverage.
- ✅ Documentation, comments, and `/// @system` annotations reflect runtime behaviour.
- ✅ Any skipped validations or sandbox blockers are clearly noted for maintainers.
- ✅ When working on a web app and the work targets a specific use case, functional element, or visual change, exercise it via the Playwright MCP to confirm the behaviour in a real browser context.

⸻

## Document Map

- Project overview – `README.md`
- Development workflows & make targets – `DEVELOPMENT.md`
- Contribution guardrails – `CONTRIBUTING.md`
- Claude planning & documentation playbook – `CLAUDE.md`

⸻

## MCP Tool Discovery

To discover available MCP servers and their capabilities programmatically:

```bash
pnpm mcp:discover
```

This script reads `.mcp.json` and lists all configured MCP servers, their capabilities, and configuration details.

### Playwright MCP

Two Playwright MCP servers are configured:

- `playwright` (headless) – Default for automated testing, no visible browser
- `playwright-headed` (visible) – For debugging when you need to see browser interactions

**Test server**: Run `pnpm dev:test` to start the application on port 4747 for testing.

**Tool naming**: In Claude Code, tools are prefixed as `mcp__<server>__<capability>`. For example: `mcp__playwright__browser_navigate` or `mcp__playwright-headed__browser_snapshot`.

**Configuration**: Both servers use 1920×1080 viewport with 10s/30s action/navigation timeouts. Screenshots and traces save to `.playwright-mcp/` (gitignored).

For full server options, run `pnpm exec mcp-server-playwright --help`.

⸻

## Communication Norms

- Be explicit about assumptions, uncertainties, and required follow-ups.
- Reference files with relative links and line numbers, e.g. `apps/carcinisation/src/stage/data.rs:417`.
- Keep hand-offs concise but complete, highlighting validation status and remaining work.
- Suggest concrete next steps instead of vague impressions; wait for maintainer approval before expanding scope.
- Preserve previous decisions unless new information or maintainer guidance requires change.
- Maintain a factual, concise tone—avoid filler, anthropomorphism, or ungrounded speculation.
