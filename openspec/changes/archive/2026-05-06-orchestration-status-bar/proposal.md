## Why

When orchestration mode is active, the main Claude session delegates file exploration and code review to cheaper Haiku sub-agents via the Agent tool. Currently there is no visual indication that delegation is happening — the status bar continues to show the orchestrator's model and "Build"/"Plan" mode, leaving the user unable to tell which model is doing the work at any given moment.

## What Changes

- The session mode chip in the footer status bar ("Build" / "Plan") is replaced with the delegated sub-agent name ("Explore" / "Review") while an Agent tool call is in flight, reverting to the normal mode label when the tool result arrives.
- The model name shown in the footer status bar updates to reflect the sub-agent's model (e.g. `haiku`) during delegation, reverting to the orchestrator's model name afterward.
- A new `delegated_agent` field is tracked on `AgentSession` — set when a `tool_use` event with `name == "Agent"` is parsed for a known conduit sub-agent, cleared when the matching `tool_result` arrives.

## Capabilities

### New Capabilities

- `delegation-status-display`: Real-time status bar reflection of which agent (orchestrator or sub-agent) is active during a Claude session with orchestration mode enabled.

### Modified Capabilities

<!-- none -->

## Impact

- `crates/conduit-agent/src/claude.rs` — JSONL event parsing must detect Agent tool_use / tool_result events and emit a new session event carrying the delegated agent name and tool_use_id.
- `crates/conduit-ui/src/session.rs` — `AgentSession` gains `delegated_agent: Option<DelegatedAgent>` tracking the active sub-agent and its model.
- `crates/conduit-ui/src/app.rs` — handle the new delegation event to update session state.
- `crates/conduit-ui/src/components/` — footer / status bar rendering reads `delegated_agent` to swap the mode label and model chip.
