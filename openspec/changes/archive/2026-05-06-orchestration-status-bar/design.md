## Context

Conduit's orchestration mode (added in `fuz/model-orchestration`) lets the main Claude session delegate tasks to cheaper Haiku sub-agents via Claude Code's Agent tool. The main session is the "orchestrator" (Sonnet/Opus); the sub-agents (`conduit-explore`, `conduit-review`) use `claude-haiku-4-5`. Currently nothing in the TUI reflects when delegation is in flight — the status bar continues showing the orchestrator's mode and model name.

The JSONL event pipeline already parses `tool_use` and `tool_result` into `ToolStartedEvent` (carries `tool_name`, `tool_id`, `arguments`) and `ToolCompletedEvent` (carries `tool_id`). The status bar (`StatusBar` component) already takes `agent_mode` and `model` as fields.

## Goals / Non-Goals

**Goals:**
- Show "Explore" or "Review" in the mode chip while the corresponding conduit sub-agent is active
- Show the sub-agent's model name (`haiku`) in the model chip during delegation
- Revert both chips to normal values when the tool result arrives
- Only activate when orchestration mode is enabled for the session

**Non-Goals:**
- Tracking non-conduit Agent tool calls (e.g. Claude delegating to arbitrary agents)
- Persisting delegation history across sessions
- Showing delegation state in the web UI

## Decisions

### 1. Detect delegation from existing `ToolStartedEvent` / `ToolCompletedEvent`

The `ToolStartedEvent` already carries `tool_name` and `tool_id`. When `tool_name == "Agent"`, parse `arguments` (a JSON string) for the `subagent_type` field. If it matches a known conduit sub-agent (`conduit-explore` → "Explore", `conduit-review` → "Review"), set `AgentSession.delegated_agent`. Clear it when `ToolCompletedEvent.tool_id` matches.

**Alternative considered:** Emitting a dedicated new `AgentEvent::DelegationStarted/Stopped`. Rejected — adds event variants when the existing tool events carry all required data. The app already processes `ToolStarted`/`ToolCompleted` and can filter for the Agent tool there.

### 2. Track delegation state on `AgentSession`

Add `delegated_agent: Option<DelegatedAgent>` to `AgentSession` where:

```rust
pub struct DelegatedAgent {
    pub tool_id: String,      // to match the ToolCompleted event
    pub display_label: String, // "Explore" or "Review"
    pub model: String,         // "haiku" (derived from agent name)
}
```

The model name is derived statically from the sub-agent name — we know `conduit-explore` and `conduit-review` use `claude-haiku-4-5`, so `ModelRegistry` can shorten this to "haiku" the same way it does for other model IDs.

**Alternative:** Store the full model ID in the agent definition and read it at runtime. Rejected — the agent definitions are written by conduit itself and we control the model; static derivation is simpler and avoids file I/O in the hot path.

### 3. Override mode label and model in `StatusBar`

Add a `delegated_agent: Option<&DelegatedAgent>` field to `StatusBar`. When set:
- Swap the mode label from `agent_mode.display_name()` to `delegated_agent.display_label`
- Swap the model chip from the session model to `delegated_agent.model`

Both revert to normal when `delegated_agent` is `None`.

## Risks / Trade-offs

- **Missed tool_result** → Mitigation: clear `delegated_agent` on `AgentSession` whenever the session transitions to `Idle` / `Ready` state (already happens on stop/interrupt), so the chip always reverts eventually.
- **arguments parsing failure** → Mitigation: treat parse errors as "not a conduit delegation" and leave `delegated_agent` unset rather than panicking.
- **Multiple concurrent Agent calls** → Claude Code doesn't currently interleave sub-agent calls, but if it did, the last `tool_id` would win. Acceptable for a status indicator.

## Migration Plan

No migration needed — all changes are additive to existing structs and the status bar rendering path. Orchestration must be enabled for the session for any of this to activate (the `delegated_agent` field is only set when `orchestration_enabled` is true).
