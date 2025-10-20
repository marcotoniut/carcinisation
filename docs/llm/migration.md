# LLM Configuration Migration Guide

**Version:** 1.0.0 → 2.0.0 (Minimal Config)
**Date:** 2025-10-20

---

## Summary of Changes

### What Changed

1. **Unified Policy & System Prompts:**
   - Created `docs/llm/policy.min.md` (unified rules, ~1200 tokens)
   - Created `prompts/system.min.md` (minimal system prompt, ~800 tokens)
   - **Replaces:** Scattered rules across 9 files (~1530 tokens) + verbose agent system messages (~1100 tokens)

2. **Normalized Config Structure:**
   - Created `config/agent.jsonc` (agent profiles)
   - Created `config/mcp/*.jsonc` (one file per MCP server with full schemas)
   - Created `config/continue.jsonc` (workspace defaults, model aliases)
   - **Replaces:** `.continue/config.yaml`, `.continue/agents/*.yaml`, `.continue/mcpServers/*.yaml`

3. **Tool Contracts Added:**
   - All MCP configs now include preconditions, postconditions, error models, retry policies
   - **Benefit:** Clear tool invocation contracts for reliable LLM behavior

4. **Lint & Test Infrastructure:**
   - Added `scripts/lint-prompts.js` (token limits, banned phrases, RFC 2119 enforcement)
   - Added `tests/llm/contracts.spec.md` (golden test cases)
   - **Benefit:** Automated quality gates for prompt engineering

### Token Reduction

| Component | Before | After | Reduction |
|-----------|--------|-------|-----------|
| Dev agent system message | ~1100 tokens | ~800 tokens (system.min.md) | -27% |
| Rules (9 files) | ~1530 tokens | ~0 tokens (moved to policy.min.md) | -100% |
| Policy | ~0 tokens (implicit) | ~1200 tokens (explicit) | n/a |
| **Total per session** | **~2630 tokens** | **~2000 tokens** | **-24%** |

**Note:** Policy is loaded once; system prompt per session. Net savings depend on session length.

---

## Migration Steps

### 1. Backup Old Config (Optional)

```bash
mkdir -p .continue.backup
cp -r .continue .continue.backup/
```

### 2. Update Package Scripts

Add to `package.json`:

```json
{
  "scripts": {
    "llm:lint": "node scripts/lint-prompts.js --audit",
    "test:llm": "echo 'LLM contract tests (spec-only; runner TBD)' && cat tests/llm/contracts.spec.md"
  }
}
```

### 3. Update Continue IDE Settings

**Option A: Use New Config (Recommended)**

Point Continue to new configs:

1. Open Continue settings (IDE-specific)
2. Set workspace config path: `config/continue.jsonc`
3. Set agent config path: `config/agent.jsonc`
4. Set MCP manifest directory: `config/mcp/`

**Option B: Symlink (Compatibility)**

Keep old paths but link to new configs:

```bash
# Backup old files
mv .continue/config.yaml .continue/config.yaml.backup

# Symlink new configs
ln -s ../config/continue.jsonc .continue/config.yaml
ln -s ../config/agent.jsonc .continue/agents/dev.yaml
# (Repeat for reviewer, MCP servers)
```

**Option C: Hybrid (Migration Period)**

Use old configs but reference new prompts:

Edit `.continue/agents/dev.yaml`:

```yaml
systemMessage: |
  {{file:prompts/system.min.md}}

  Policy: {{file:docs/llm/policy.min.md}}
```

### 4. Update MCP Server Manifests

If using new configs, update `.continue/mcpServers/*.yaml` to load from `config/mcp/`:

```yaml
# .continue/mcpServers/base.yaml
name: Toolkit Base MCP
version: 0.0.2
schema: v1

include: ../../config/mcp/base.jsonc
```

Or delete old manifests if Continue supports `config/mcp/` directly.

### 5. Verify Lint & Tests

```bash
# Lint prompts
pnpm llm:lint

# Expected output:
# === LLM Prompt Audit ===
# Checking docs/llm/policy.min.md...
#   Words: ~900, Tokens (est): ~1200
#   ✅ No issues
# Checking prompts/system.min.md...
#   Words: ~600, Tokens (est): ~800
#   ✅ No issues
# Total: 0 error(s), 0 warning(s)
```

### 6. Test Agent Behavior

1. Open Continue IDE with dev agent
2. Ask: "Find all Bevy systems in the project"
3. Verify:
   - Uses `find_bevy_system_like_fns` (Bevy MCP)
   - Returns file:line matches
   - No errors

4. Ask: "Run `make check`"
5. Verify:
   - Uses `make("check")` (Rust Docs MCP)
   - Surfaces stderr on failure
   - Follows policy (minimal diffs, etc.)

### 7. Commit Changes

```bash
git add config/ docs/llm/ prompts/ scripts/ tests/llm/ package.json
git commit -m "refactor(llm): normalize config, add policy/system prompts, +24% token reduction

- Consolidate .continue/ configs to config/
- Add policy.min.md (1200 tokens) and system.min.md (800 tokens)
- Add MCP tool contracts (preconditions, errors, retry)
- Add lint-prompts.js and contracts.spec.md
- Net token reduction: ~2630 → ~2000 tokens per session

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Rollback Instructions

### Quick Rollback (Restore Backup)

```bash
rm -rf .continue
mv .continue.backup .continue
git checkout HEAD -- config/ docs/llm/ prompts/ scripts/ tests/llm/ package.json
```

### Selective Rollback (Keep New Docs, Revert Config)

```bash
# Keep policy.min.md and system.min.md
git checkout HEAD -- config/ .continue/

# Update agents to reference new prompts
# (Edit .continue/agents/dev.yaml manually)
```

---

## Per-Model Overrides

### Override System Prompt for Specific Agent

Edit `config/agent.jsonc`:

```jsonc
{
  "agents": {
    "dev": {
      "systemPromptFile": "prompts/system.min.md",  // Default
      "systemPromptOverride": "Use this inline override instead of file"
    }
  }
}
```

### Override Policy for Environment

Create `config/agent.local.jsonc` (gitignored):

```jsonc
{
  "agents": {
    "dev": {
      "policyFile": "docs/llm/policy.custom.md"  // Override for this env
    }
  }
}
```

Then update Continue settings to prefer `config/agent.local.jsonc` over `config/agent.jsonc`.

### Override MCP Server for Development

Edit `config/mcp/base.jsonc`:

```jsonc
{
  "enabled": true,  // Set to false to disable
  "env": {
    "TOOLKIT_ROOT": "/custom/path"  // Override env vars
  }
}
```

Or create `config/mcp/base.local.jsonc` with overrides.

---

## Per-Environment Configuration

### Development Environment

Use local Ollama models (default):

```jsonc
// config/continue.jsonc
{
  "providers": {
    "ollama": {
      "endpoint": "http://localhost:11434"
    }
  }
}
```

### CI/CD Environment

Use cloud models (OpenAI/Anthropic):

```jsonc
// config/continue.ci.jsonc
{
  "providers": {
    "anthropic": {
      "endpoint": "https://api.anthropic.com/v1",
      "apiKey": "${ANTHROPIC_API_KEY}"
    }
  },
  "models": {
    "default": "claude",
    "aliases": {
      "claude": {
        "name": "Claude 3.5 Sonnet",
        "provider": "anthropic",
        "model": "claude-3-5-sonnet-20241022",
        "roles": ["chat", "apply", "edit"]
      }
    }
  }
}
```

### Production/Review Environment

Disable write operations:

```jsonc
// config/mcp/rust-docs.prod.jsonc
{
  "env": {
    "ALLOW_WRITE": "0"  // Enforce read-only
  }
}
```

---

## Troubleshooting

### Issue: Lint fails with "Banned phrase found"

**Cause:** New policy enforces RFC 2119 keywords (MUST/MAY instead of try/should).

**Fix:** Update prompt files to use RFC 2119 keywords:

```diff
- You should check the return code
+ MUST check the return code
```

### Issue: Agent doesn't load new system prompt

**Cause:** Continue may cache old config.

**Fix:**
1. Restart Continue IDE
2. Clear Continue cache (IDE-specific)
3. Verify `systemPromptFile` path in `config/agent.jsonc`

### Issue: MCP tools return "precondition unmet"

**Cause:** New tool contracts enforce preconditions (e.g., ripgrep installed).

**Fix:**
1. Check error message for missing dependency
2. Install dependency (e.g., `brew install ripgrep`)
3. Retry tool call

### Issue: Token count still high

**Cause:** Old rules still loaded alongside new policy.

**Fix:** Remove or disable old rules:

```jsonc
// config/agent.jsonc
{
  "agents": {
    "dev": {
      "rules": []  // Disable all old rules; policy.min.md covers them
    }
  }
}
```

### Issue: MCP server fails to start

**Cause:** Manifest path changed or Docker not running.

**Fix:**
1. Verify Docker running: `docker ps`
2. Check manifest path in `config/mcp/*.jsonc`
3. Run manually: `./dev/mcp/base/run.sh` (check logs)

---

## Feature Comparison

### Old Config (.continue/)

**Pros:**
- Simple YAML format
- Direct integration with Continue
- Inline system messages

**Cons:**
- Redundancy (duplicated actions, models)
- No token budgets
- No tool contracts
- Verbose system messages (~1100 tokens)
- No lint/test gates

### New Config (config/)

**Pros:**
- DRY (shared config, no duplication)
- Explicit tool contracts (preconditions, errors, retry)
- Token budgets enforced (lint)
- Model aliases (fast/deep/code)
- Lint & test infrastructure
- 24% token reduction

**Cons:**
- JSONC format (more verbose than YAML for simple cases)
- Requires migration effort
- Continue may need manual config path update

---

## Next Steps

1. ✅ Migrate config (follow steps above)
2. ⏭ Run lint: `pnpm llm:lint`
3. ⏭ Test agents with sample requests
4. ⏭ Merge PR (see PR description for checklist)
5. ⏭ Monitor agent behavior for regressions
6. ⏭ Iterate on policy.min.md based on usage patterns

---

## Support

**Issues:** Report problems at [repo]/issues
**Questions:** Ask in team chat or tag @llm-admin
**Feedback:** Suggest improvements to policy.min.md via PR

---

## Appendix: File Mapping

| Old File | New File | Notes |
|----------|----------|-------|
| `.continue/config.yaml` | `config/continue.jsonc` | Workspace defaults |
| `.continue/agents/dev.yaml` | `config/agent.jsonc` (agents.dev) | Dev agent profile |
| `.continue/agents/reviewer.yaml` | `config/agent.jsonc` (agents.reviewer) | Reviewer agent profile |
| `.continue/mcpServers/base.yaml` | `config/mcp/base.jsonc` | Base MCP manifest |
| `.continue/mcpServers/bevy.yaml` | `config/mcp/bevy.jsonc` | Bevy MCP manifest |
| `.continue/mcpServers/rust-docs.yaml` | `config/mcp/rust-docs.jsonc` | Rust Docs MCP manifest |
| `.continue/mcpServers/scribe.yaml` | `config/mcp/scribe.jsonc` | Scribe MCP manifest |
| `.continue/mcpServers/playwright.yaml` | `config/mcp/playwright.jsonc` | Playwright MCP manifest |
| `.continue/rules/*.md` (9 files) | `docs/llm/policy.min.md` | Consolidated rules |
| (none) | `prompts/system.min.md` | Minimal system prompt |
| (none) | `scripts/lint-prompts.js` | Lint script |
| (none) | `tests/llm/contracts.spec.md` | Golden test cases |
| (none) | `docs/llm/audit.md` | Config audit report |
| (none) | `docs/llm/migration.md` | This file |

**Old files:** Keep for compatibility during migration period; delete after validation.
