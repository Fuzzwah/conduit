## Context

Conduit wraps the Claude Code CLI as a subprocess and parses its JSONL output stream — it does **not** call the Anthropic API directly. Claude Code natively supports sub-agent delegation via agent definition markdown files placed in `.claude/agents/` (project-local) or `~/.claude/agents/` (user-global). When the main Claude session calls the `Agent` tool with a `subagent_type` matching an agent definition name, Claude Code spins up that sub-agent with its declared model and system prompt, then returns the result to the parent context.

WozCode exploits this pattern with a `woz:explore` agent (Haiku) that the main agent delegates cheap exploration work to. Conduit can implement the same pattern without any direct API calls or new infrastructure — just agent definition files and a nudge in the session prompt.

Current state: `AgentStartConfig` (runner.rs) drives session startup; `ClaudeCodeRunner::start()` builds the `claude` CLI command; `AgentSession` (session.rs) holds per-session UI state; `Config` (settings.rs) holds persisted global config.

## Goals / Non-Goals

**Goals:**
- Write `conduit-explore` and `conduit-review` agent definitions to `~/.claude/agents/` when a Claude session with orchestration enabled starts
- Inject a concise instruction block into the session prompt when orchestration is on
- Provide a per-session toggle in the TUI (following the reasoning/model selector pattern)
- Persist the default state in `conduit.toml` as `orchestration.enabled_by_default`

**Non-Goals:**
- Directly calling the Anthropic API from conduit (agent delegation stays inside Claude Code)
- Orchestration for non-Claude agents (Codex, Gemini, Pi, etc.)
- Automatic tool interception or custom MCP servers
- Tracking token savings or cost attribution for sub-agent calls
- Web UI toggle (TUI only for initial implementation)

## Decisions

### Write agent definitions to `~/.claude/agents/` (not project `.claude/agents/`)

**Rationale:** Conduit runs in user-owned worktrees of arbitrary projects. Writing to a project's `.claude/agents/` would pollute the project's version control and potentially conflict with existing agent definitions. Global user-level agents in `~/.claude/agents/` are available to all Claude sessions and are entirely owned by the user's tooling, not the project.

**Alternative considered:** Write to the worktree's `.claude/agents/` per session and clean up on stop. Rejected: adds lifecycle complexity, risks leaving stale files on crash.

### Idempotent file writes (only update if content differs)

**Rationale:** Reading a hash before writing avoids unnecessary filesystem chatter on every session start. Agent definition content is static and versioned in conduit, so a simple equality check suffices.

**Alternative considered:** Always overwrite. Rejected: unnecessary I/O, and could cause confusing behavior if a user has customized the files.

### Append orchestration instructions to the session prompt (not a separate `--system-prompt` flag)

**Rationale:** The Claude CLI does not expose a `--system-prompt` flag for standalone (`-p`) invocations. Appending to the user prompt is the only reliable injection point. The instructions are clearly demarcated with a `---` separator so they read as metadata, not task content.

**Alternative considered:** Write a temporary `CLAUDE.md` override. Rejected: CLAUDE.md affects all sessions in the working directory, not just the current one.

### Per-session toggle stored in `AgentSession`, default in `Config`

**Rationale:** Matches the existing `reasoning_effort` pattern exactly. Per-session state lives in `AgentSession`; the global default is persisted to `conduit.toml`. A new `OrchestrationConfig { enabled_by_default: bool }` sub-struct follows the `QueueConfig`/`SteerConfig` pattern.

### New `orchestration.rs` module in `conduit-agent`

**Rationale:** Agent definitions and the instruction template are specific to agent startup concerns. Co-locating them in `conduit-agent` keeps the logic close to `claude.rs` where it's consumed, avoids coupling `conduit-config` to file I/O, and makes the module easy to unit test in isolation.

## Risks / Trade-offs

- **User customized the agent files** → Conduit checks content before writing and only updates if content differs. Users who want to customize can do so; conduit won't clobber unless the definition changes in a new conduit release.
- **`~/.claude/agents/` doesn't exist yet** → `ensure_orchestration_agents()` creates the directory with `std::fs::create_dir_all`.
- **Sub-agents available globally (not just in orchestration sessions)** → The agent definition files are visible in any Claude session the user runs, even outside conduit. This is acceptable — the agents are harmless read-only helpers and follow the same pattern as conduit's own rust-reviewer/web-ui-reviewer agents.
- **Claude may not use sub-agents even when instructed** → The instruction block is a nudge, not a guarantee. Claude's actual behavior depends on its judgment. This is inherent to the prompt-based approach and matches WozCode's pattern.
- **`dirs` crate for home dir resolution** → `dirs` is already used transitively in the workspace; no new dependency needed.

## Migration Plan

- No database migrations required
- The new `orchestration` config section is optional with `#[serde(default)]`; existing `conduit.toml` files without it continue to work (defaults to `enabled_by_default: false`)
- Agent definition files in `~/.claude/agents/` are new additions; no existing files are modified
- Rollback: remove the two agent definition files from `~/.claude/agents/` and revert conduit binary

## Open Questions

_(none — design is fully determined by the constraints above)_
