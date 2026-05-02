# Agents

Conduit orchestrates AI coding assistants called **agents**.

## Supported Agents

| Agent | Provider | Context Window |
|-------|----------|----------------|
| [Codex CLI](./agents/codex.md) | OpenAI | 272K tokens |
| [Claude Code](./agents/claude-code.md) | Anthropic | 200K tokens |
| [Gemini CLI](./agents/gemini.md) | Google | 1M tokens |
| [Dirac CLI](./agents/dirac.md) | Multi-provider | Varies by model |
| CE CLI | Multi-provider | Varies by model |

## Selecting an Agent

The default agent is configured in `~/.conduit/config.toml`:

```toml
default_agent = "codex"  # or "claude", "dirac", "gemini", "opencode", "cecli"
```

## Agent Detection

On startup, Conduit searches for:
- `codex` binary (Codex CLI)
- `claude` binary (Claude Code)
- `gemini` binary (Gemini CLI)
- `dirac` binary (Dirac CLI)
- `cecli` or `aider-ce` binary (CE CLI)

Configure custom paths in settings if needed.

## Agent Capabilities

All agents can:
- Read and write files
- Execute shell commands
- Search codebases
- Analyze code structure

See individual agent pages for specific features.
