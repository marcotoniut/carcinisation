# LLM Systems Audit Report

**Date:** 2025-10-20
**Scope:** Agent configuration, MCP servers/clients, Continue IDE integration
**Objective:** Evaluate, refactor, and minimize configuration for improved model reliability and reduced token overhead

---

## Executive Summary

The repository uses **Continue IDE** with **5 custom MCP servers** and **2 agent profiles** (dev, reviewer). Configuration is split across `.continue/` with **9 rule files** (~1150 words of rules) and **2 agent configs** (~1050 words). The primary issues are:

1. **Redundancy:** Agent configs duplicate actions and model lists; rules repeat context discovery patterns
2. **Token overhead:** ~2200 words of prompt material loaded per agent session (est. 2900+ tokens)
3. **Precedence ambiguity:** No explicit priority ladder; conflicts between rules not documented
4. **Tool contracts:** MCP tool schemas exist but lack preconditions, error models, and retry guidance
5. **Model coupling:** Hardcoded Ollama model names with no abstraction layer

**Recommendation:** Consolidate to unified policy + minimal system prompts, normalize config structure, add lint/test gates.

---

## File Inventory

### Continue Configuration Files

| File | Purpose | Consumer | Load Order | Word Count | Approx Tokens |
|------|---------|----------|------------|------------|---------------|
| `.continue/config.yaml` | Workspace config (models, actions, docs) | Continue | 1 | ~400 | ~530 |
| `.continue/agents/dev.yaml` | Dev agent (system message, MCP routing) | Continue (agent mode) | 2 | ~830 | ~1100 |
| `.continue/agents/reviewer.yaml` | Review agent (auditing focus) | Continue (agent mode) | 2 | ~216 | ~290 |

### Rules (Loaded by Agents)

| File | Purpose | Applies To | Word Count | Approx Tokens |
|------|---------|------------|------------|---------------|
| `.continue/rules/000-project-overview.md` | Project context, scope, layout | All | 245 | ~325 |
| `.continue/rules/010-build-run.md` | Build targets, env vars, MCP commands | All | 300 | ~400 |
| `.continue/rules/015-rust-deps.md` | Rust dependency management | All | 84 | ~112 |
| `.continue/rules/020-bevy-upgrade-playbook.md` | Bevy version migration strategy | All | 239 | ~320 |
| `.continue/rules/021-bevy-replicon.md` | Networking plugin notes | Conditional | 65 | ~87 |
| `.continue/rules/030-testing-and-ci.md` | Test commands and practices | All | 50 | ~67 |
| `.continue/rules/040-local-docs.md` | Local documentation paths | All | 25 | ~33 |
| `.continue/rules/addmoduledocs.md` | Module doc encouragement | Conditional | 45 | ~60 |
| `.continue/rules/099-apply-guardrails.md` | General guardrails and refactor policy | All | 94 | ~125 |

**Total rules:** ~1150 words, ~1530 tokens

### MCP Server Manifests

| File | Server Name | Transport | Tools | Purpose |
|------|-------------|-----------|-------|---------|
| `.continue/mcpServers/base.yaml` | Toolkit Base MCP | STDIO (Docker/venv) | 2 | Sandboxed shell utilities |
| `.continue/mcpServers/bevy.yaml` | Bevy MCP | STDIO (Docker/venv) | 2 | Bevy-specific analysis |
| `.continue/mcpServers/rust-docs.yaml` | Rust Docs MCP | STDIO (Docker/venv) | 7 | Rust doc analysis/generation |
| `.continue/mcpServers/scribe.yaml` | Scribe MCP | STDIO (Docker/venv) | 3 | AI-assisted writing (commit, PR) |
| `.continue/mcpServers/playwright.yaml` | Browser (Playwright) | STDIO (pnpm) | 8 | Web automation |

### MCP Server Implementations

| File | Server | Lines | Tools Provided | Dependencies |
|------|--------|-------|----------------|--------------|
| `dev/mcp/base/server.py` | Toolkit Base MCP | 101 | `run_shell`, `env_info` | fastmcp |
| `dev/mcp/bevy/server.py` | Bevy MCP | 91 | `bevy_version`, `find_bevy_system_like_fns` | fastmcp, ripgrep |
| `dev/mcp/rust-docs/server.py` | Rust Docs MCP | 198 | `list_rust_files`, `find_module_doc_gaps`, `find_public_item_doc_gaps`, `cargo_doc`, `docs_index_path`, `make`, `insert_module_header` | fastmcp, ripgrep, cargo, make |
| `dev/mcp/scribe/server.py` | Scribe MCP | 93 | `draft_commit`, `draft_pr`, `summarize` | fastmcp, ollama (HTTP) |

### Package Configuration

| File | Purpose | Key Dependencies |
|------|---------|------------------|
| `package.json` | Top-level workspace config | `@playwright/mcp@0.0.43`, pnpm workspaces |

---

## Dependency & Precedence Map

### Load Order & Precedence

```
1. Continue Core Config (.continue/config.yaml)
   ├─ Models, actions, docs (workspace-wide)
   └─ No system message (defers to agent)

2. Agent Selection (.continue/agents/{dev,reviewer}.yaml)
   ├─ Agent-specific models & tool routing
   ├─ systemMessage (OVERRIDES workspace default)
   └─ Loads rules via Continue's rule engine

3. Rules Directory (.continue/rules/*.md)
   ├─ Numbered files suggest load order (000, 010, 015, etc.)
   ├─ 099-apply-guardrails.md is "meta" but loads last
   └─ No explicit priority among rules if conflicts exist

4. MCP Manifests (.continue/mcpServers/*.yaml)
   ├─ Server launch configs (command, args, env)
   └─ Referenced by agent `tools` section

5. MCP Server Code (dev/mcp/*/server.py)
   ├─ Tool schemas generated at runtime
   └─ Tool descriptions embedded in decorator
```

### Who Reads What

| Consumer | Reads | Priority |
|----------|-------|----------|
| Continue Workspace | `config.yaml` | Base |
| Continue Dev Agent | `agents/dev.yaml` → all rules → 5 MCP servers | Agent overrides workspace |
| Continue Reviewer Agent | `agents/reviewer.yaml` → all rules → 2 MCP servers | Agent overrides workspace |
| MCP Servers | Launch scripts (`run.sh`) → env vars → server.py | Independent |

### Conflict Scenarios

1. **Model selection:** `config.yaml` lists models; agents override with their own lists → **Agent wins**
2. **Actions:** Duplicated in config.yaml and agents → **Both loaded** (redundant)
3. **System message:** Only agents define it → **No workspace fallback**
4. **Rules precedence:** No documented order → **Last-loaded wins (099-apply-guardrails.md?)**

---

## Token Cost Analysis

### Per-Session Overhead (Estimated)

| Component | Words | Approx Tokens | Frequency |
|-----------|-------|---------------|-----------|
| Agent system message (dev) | ~830 | ~1100 | Every session |
| All rules (9 files) | ~1150 | ~1530 | Every session |
| MCP tool schemas (22 tools) | ~600 | ~800 | On-demand |
| **Total (dev agent)** | **~2580** | **~3430** | **Per session** |
| Reviewer agent system message | ~216 | ~290 | Every session |
| All rules (9 files, same) | ~1150 | ~1530 | Every session |
| MCP tool schemas (9 tools) | ~270 | ~360 | On-demand |
| **Total (reviewer agent)** | **~1636** | **~2180** | **Per session** |

### Largest Token Offenders

1. **dev.yaml systemMessage (830 words):** Verbose MCP routing table, redundant discovery workflow
2. **.continue/rules/010-build-run.md (300 words):** Extensive Make target listing
3. **.continue/rules/000-project-overview.md (245 words):** Redundant project context
4. **.continue/rules/020-bevy-upgrade-playbook.md (239 words):** Conditional playbook loaded always

### Optimization Opportunities

- **Merge discovery patterns:** Single concise workflow instead of per-agent duplication
- **Consolidate Make targets:** Reference Makefile instead of enumerating all targets
- **Conditional rules:** Load 020-bevy-upgrade-playbook.md only when needed
- **Tool routing:** Replace prose table with structured JSON schema
- **Remove soft language:** "try", "should", "kindly" → MUST/MAY

---

## Conflict Analysis

### Identified Conflicts

1. **Path format (critical):**
   - `dev.yaml` (line 138): "All paths MUST be relative to repo root"
   - `reviewer.yaml` (line 48): "All paths MUST be relative to repo root"
   - **Redundant but consistent** (no conflict)

2. **Tool selection priority:**
   - `dev.yaml` (line 133): "ALWAYS use Browser MCP tools" for web tasks
   - `dev.yaml` (line 141): "Reach for the Base MCP when you specifically need its sandboxed shell"
   - **Ambiguous:** What if web scraping requires shell access? No tie-breaker.

3. **Discovery workflow:**
   - `dev.yaml` (lines 87-91): 5-step context discovery
   - `dev.yaml` (lines 143-150): Overlapping 5-step discovery workflow
   - **Duplicated** with minor wording changes

4. **Editing strategy:**
   - `099-apply-guardrails.md`: "minimal set of files" but "include new files when they serve the feature"
   - `020-bevy-upgrade-playbook.md`: "scoped diffs—tackle one subsystem or crate at a time"
   - **Potentially conflicting** for large features

5. **Documentation coupling:**
   - `099-apply-guardrails.md`: "Couple docs/tests with the code they describe; avoid doc-only diffs"
   - `addmoduledocs.md`: "add a short summary" when touching modules
   - **Implicit conflict:** Does adding module docs violate "avoid doc-only diffs"?

### Redundancies

1. **Actions list:** Duplicated in `config.yaml` (lines 43-103) and `agents/dev.yaml` (lines 32-80)
2. **Models list:** Partially duplicated across config.yaml and agent files with different role assignments
3. **MCP tool routing:** Prose table in `dev.yaml` (lines 93-135) vs implicit from tool schemas
4. **Build/test commands:** Repeated across 010-build-run.md and systemMessage examples

### Leaky Abstractions

1. **Docker/venv fallback:** Mentioned in 010-build-run.md but implemented in `run.sh` scripts
2. **Ollama model names:** Hardcoded in scribe/server.py; no config abstraction
3. **Repo root path:** Calculated in each MCP server independently (DRY violation)

---

## Risk Analysis

### Brittle Patterns

1. **Undefined tool contracts:**
   - No preconditions (when is `find_bevy_system_like_fns` safe to call?)
   - No error models (what does `run_shell` return on failure?)
   - No retry guidance (should `cargo_doc` be retried on timeout?)
   - No rate limits (scribe uses Ollama with 60s timeout, no backoff)

2. **Model-specific assumptions:**
   - Ollama-only setup (no OpenAI/Anthropic/etc. provider flexibility)
   - Hardcoded model names in multiple files (drift risk)
   - No model capability flags (which models support tool calling?)

3. **Ambiguous role instructions:**
   - "Use built-in Continue tools for quick file reads" vs "Reach for Base MCP" — no clear decision tree
   - "Favor intent-driven searching" — subjective, no examples
   - "Phase your work: TOOLS → PLAN → (optional) PATCH" — not enforced

4. **Path safety:**
   - `_resolve_path()` guards against escapes but no validation for symlinks or special files
   - No mention of .gitignore or ignored paths in tool routing

5. **Write operations:**
   - `insert_module_header` requires `ALLOW_WRITE=1` but rust-docs.yaml sets `ALLOW_WRITE=0` by default
   - Conflict between intent (enable writes) and safety (disable by default)

### Breaking Change Risks

1. **Bevy version assumptions:** Rules assume 0.14 → 0.15 → 0.17; will break if jumping versions
2. **Makefile dependencies:** Rules reference Make targets without fallback if Makefile changes
3. **File structure:** Hard-coded paths (src/, tools/, scripts/) will break if workspace layout changes
4. **MCP transport:** All use STDIO; switching to HTTP would require manifest rewrites

---

## Tool Contract Gaps

### Missing Elements (Per Tool)

#### Base MCP

- **run_shell:**
  - ❌ Precondition: Max command length? Allowed binaries?
  - ❌ Postcondition: returncode semantics (0 = success, non-zero = ?)
  - ❌ Error model: What errors are retryable?
  - ❌ Idempotency: Is command safe to retry?
  - ❌ Rate limits: No throttling mechanism

- **env_info:**
  - ✅ Simple, deterministic
  - ❌ No schema version

#### Bevy MCP

- **bevy_version:**
  - ❌ Precondition: Cargo.toml must exist in ROOT
  - ✅ Returns "(not found)" on failure (explicit)
  - ❌ No guidance on what to do if not found

- **find_bevy_system_like_fns:**
  - ❌ Precondition: ripgrep must be installed
  - ❌ Postcondition: Format of output not documented
  - ❌ Error model: Silent failure if rg missing?

#### Rust Docs MCP

- **list_rust_files, find_module_doc_gaps, find_public_item_doc_gaps:**
  - ❌ Precondition: ripgrep required
  - ✅ Returns list/string (clear)
  - ❌ Error handling: Silent empty list on rg failure?

- **cargo_doc:**
  - ❌ Precondition: Cargo.toml must exist, cargo toolchain installed
  - ❌ Timeout: Can hang on large workspaces
  - ❌ Retry: Should failures be retried?
  - ❌ Side effects: Creates target/doc/ directory

- **make:**
  - ❌ Precondition: Makefile must exist, Make installed
  - ❌ Error model: Non-zero exit not surfaced clearly
  - ❌ Timeout: No limit on long-running targets
  - ⚠️ Ambiguous: env param parsing ("KEY=VAL KEY2=VAL2")

- **insert_module_header:**
  - ✅ Precondition: Requires ALLOW_WRITE=1 (enforced)
  - ❌ Postcondition: File encoding assumption (utf-8 hardcoded)
  - ❌ Idempotency: Will duplicate header if called twice
  - ❌ Undo: No rollback mechanism

#### Scribe MCP

- **draft_commit, draft_pr, summarize:**
  - ❌ Precondition: Ollama must be running and accessible
  - ⚠️ Error model: Returns empty string on connection failure (silent)
  - ❌ Timeout: Hardcoded 60s, no retry
  - ❌ Rate limit: No backoff for Ollama overload
  - ❌ Fallback: No alternative if JSON parsing fails

#### Browser MCP (Playwright)

- **All tools:**
  - ❌ Precondition: Browser must be launched (state dependency)
  - ❌ Idempotency: click/type not idempotent
  - ❌ Error model: Selector not found, timeout behavior?
  - ❌ Rate limit: No guidance on wait times

---

## Model-Specific Notes

### Current Models (Ollama Local)

| Model | Role | Size | Capabilities | Notes |
|-------|------|------|--------------|-------|
| `llama3.1:8b` | chat, apply | 8B | Tool calling | Primary for workspace config |
| `qwen2.5-coder:14b` | edit | 14B | Code completion | |
| `qwen3-coder:30b` | chat, apply, rerank | 30B | Tool calling, ranking | Dev agent primary |
| `codellama:7b-code` | autocomplete | 7B | Fast completion | |
| `nomic-embed-text:latest` | embed | ~140M | Embeddings | |
| `llama3.1:8b-instruct-q8_0` | (Scribe internal) | 8B | JSON mode | Hardcoded in scribe/server.py |

### Issues

1. **No provider abstraction:** All hardcoded to Ollama
2. **Model ID drift:** `llama3.1:8b` in config, `llama3.1:8b-instruct-q8_0` in scribe
3. **No capability flags:** Which models support tool calling? JSON mode?
4. **No fallback chain:** If qwen3-coder:30b unavailable, no degradation strategy

---

## Recommendations

### High Priority

1. **Create unified policy (docs/llm/policy.min.md):**
   - Explicit priority ladder (Safety → User Intent → Tool Contracts → Style)
   - Tie-breakers for known conflicts
   - ≤1200 tokens

2. **Create minimal system prompt (prompts/system.min.md):**
   - Tool Use Contract (when/inputs/outputs/retry/backoff)
   - Discovery workflow (single, concise version)
   - ≤800 tokens

3. **Normalize configs:**
   - `config/agent.jsonc` (merge dev/reviewer common elements)
   - `config/mcp/*.jsonc` (one file per server with full schema)
   - `config/continue.jsonc` (workspace defaults, model aliases)

4. **Add tool contracts:**
   - Preconditions, postconditions, error models in MCP server docstrings
   - Explicit idempotency and retry guidance

5. **Consolidate rules:**
   - Merge 000-project-overview + 010-build-run → single "project.md"
   - Make conditional rules (020-bevy-upgrade-playbook) opt-in

### Medium Priority

6. **Lint script (scripts/lint-prompts.js):**
   - Token count enforcement
   - Banned phrases (try/should → MUST/MAY)
   - Double-negative detection
   - TODO/FIXME in prompts

7. **Test suite (tests/llm/contracts.spec.md):**
   - Golden cases: "user request X → tool calls Y"
   - Error handling: "missing precondition Z → clear error message"

8. **Migration guide (docs/llm/migration.md):**
   - Old → new config mapping
   - Rollback instructions
   - Per-model/per-environment overrides

### Low Priority

9. **Model abstraction:**
   - `models.aliases` in config (fast/deep/code)
   - Provider registry (ollama/openai/anthropic)

10. **MCP transport flexibility:**
    - Support STDIO and HTTP in manifests
    - Environment-based transport selection

---

## Deletion Candidates

### Files to Remove (Superseded)

- None currently (all files actively used)

### Consolidation Opportunities

1. **Merge into config/continue.jsonc:**
   - `.continue/config.yaml` (actions, docs, base models)

2. **Merge into config/agent.jsonc:**
   - `.continue/agents/dev.yaml` (remove redundant actions)
   - `.continue/agents/reviewer.yaml` (DRY with dev)

3. **Merge rules:**
   - `000-project-overview.md` + `010-build-run.md` → `project.md`
   - `015-rust-deps.md` + `030-testing-and-ci.md` → `development.md`
   - `020-bevy-upgrade-playbook.md` → move to `docs/bevy-upgrade.md` (out of always-loaded rules)
   - `021-bevy-replicon.md` → merge with 020 or remove (65 words, very narrow)
   - `040-local-docs.md` → merge into project.md (25 words)
   - `addmoduledocs.md` → merge into development.md or policy.md
   - `099-apply-guardrails.md` → move to policy.min.md

**Result:** 9 rules → 3 rules (~600 words, ~800 tokens, -48% reduction)

### MCP Manifest Consolidation

- Move to `config/mcp/` with schema validation
- Add `enabled: true/false` flag to selectively load servers

---

## Next Steps

1. ✅ Audit complete
2. ⏭ Write policy.min.md and system.min.md
3. ⏭ Refactor configs to config/
4. ⏭ Add MCP tool contracts (docstring updates)
5. ⏭ Create lint-prompts.js + contracts.spec.md
6. ⏭ Write migration.md
7. ⏭ Open PR with checklist

---

## Appendix: Token Estimation Method

- **Words to tokens:** ~1.33 multiplier (average for technical English)
- **YAML/JSON overhead:** ~1.2x for structured content
- **Code blocks:** ~1.1x (mostly literal tokens)

**Formula:** `tokens ≈ (words × 1.33) + (yaml_lines × 0.5)`
