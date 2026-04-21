# Claude Code

[Claude Code](https://docs.anthropic.com/en/docs/claude-code) is Anthropic's official CLI for Claude.

## Features

- **Tool Execution** — Read, write, and execute commands
- **Build/Plan Modes** — Toggle between full execution and read-only
- **Multiple Models** — Opus 4.6, Sonnet 4.6, Opus 4.6 [1m], Sonnet 4.6 [1m], Haiku 4.5
- **200K / 1M Context** — Choose standard or extended context variants

## Models

| Model | Best For |
|-------|----------|
| Opus 4.6 | Most capable for complex work (default) |
| Sonnet 4.6 | Best for everyday tasks |
| Opus 4.6 [1m] | Opus 4.6 with expanded context |
| Sonnet 4.6 [1m] | Sonnet 4.6 with expanded context |
| Haiku 4.5 | Fastest for quick answers |

## Build vs Plan Mode

- **Build Mode** (default) — Full capabilities
- **Plan Mode** — Read-only analysis, no file modifications

Toggle with `Tab` or `Ctrl+4`.

## Default CLI Invocation In Conduit

When you run `conduit` (TUI) or `conduit serve` (web UI) and choose Claude, Conduit sends prompts using Claude's `--input-format stream-json` flow by default.

That means normal day-to-day Conduit Claude sessions are **not** using `claude -p`.

Conduit only uses `-p` in non-stream input cases.

## Tools Available

- `Read` — Read file contents
- `Write` — Create or overwrite files
- `Edit` — Modify existing files
- `Bash` — Execute shell commands
- `Glob` — Find files by pattern
- `Grep` — Search file contents

## Installation

```bash
npm install -g @anthropic-ai/claude-code
```
