## Why

Conduit sessions run a single model for all tasks, meaning expensive orchestrator models (Sonnet/Opus) are used even for cheap read-only operations like file summarization and diff review. Delegating these to a cheaper model (Haiku) via Claude Code's native sub-agent system reduces token cost while keeping the orchestrator's context window lean.

## What Changes

- Two new Claude Code agent definition files are written to `~/.claude/agents/` at session start: `conduit-explore` (file reading, search, codebase summarization) and `conduit-review` (diff/change review), both using `claude-haiku-4-5`
- A new per-session **orchestration mode** toggle is added; when enabled, an instruction block is appended to the session prompt nudging Claude to delegate exploration and review tasks to these sub-agents
- A new `orchestration.enabled_by_default` global config option controls the default state of the toggle
- A new TUI selector component enables toggling orchestration per session, following the existing reasoning/model selector pattern

## Capabilities

### New Capabilities

- `orchestration-mode`: Per-session toggle that enables model orchestration — writing sub-agent definitions and injecting delegation instructions into the Claude session prompt

### Modified Capabilities

_(none — no existing specs change requirements)_

## Impact

- **`crates/conduit-agent`**: New `orchestration.rs` module; `AgentStartConfig` gains `orchestration_enabled` field; `ClaudeCodeRunner::start()` conditionally writes agent files and modifies the prompt
- **`crates/conduit-config`**: New `OrchestrationConfig` struct added to `Config`
- **`crates/conduit-ui`**: New selector component; `AgentSession` gains `orchestration_enabled` field; app wires up toggle action
- **`~/.claude/agents/`**: Two new markdown files written by conduit (idempotent)
- **Dependencies**: No new crate dependencies; uses `dirs` (already transitively available) for home dir resolution
