# LLM Configuration (Normalized)

This directory contains normalized LLM system configuration.

## Structure

```
config/
├── agent.jsonc         # Agent profiles (dev, reviewer)
├── continue.jsonc      # Workspace defaults (models, actions, docs)
├── mcp/                # MCP server manifests
│   ├── base.jsonc      # Base toolkit (shell, env)
│   ├── bevy.jsonc      # Bevy-specific analysis
│   ├── rust-docs.jsonc # Rust doc generation
│   ├── scribe.jsonc    # AI-assisted writing
│   └── playwright.jsonc # Web automation
└── README.md           # This file
```

## Usage

### Continue IDE

Point Continue to these configs:

1. Workspace config: `config/continue.jsonc`
2. Agent config: `config/agent.jsonc`
3. MCP manifests: `config/mcp/*.jsonc`

### Scripts

- **Lint prompts:** `pnpm llm:lint`
- **Test contracts:** `pnpm test:llm`

## Documentation

- [docs/llm/audit.md](../docs/llm/audit.md) - Configuration audit
- [docs/llm/policy.min.md](../docs/llm/policy.min.md) - Unified policy (~1200 tokens)
- [prompts/system.min.md](../prompts/system.min.md) - System prompt (~800 tokens)
- [docs/llm/migration.md](../docs/llm/migration.md) - Migration guide

## See Also

- Old config (pre-refactor): `.continue/` (kept for compatibility)
- MCP implementations: `dev/mcp/*/server.py`
