## 1. Session State

- [x] 1.1 Add `DelegatedAgent { tool_id: String, display_label: String, model: String }` struct to `crates/conduit-ui/src/session.rs`
- [x] 1.2 Add `pub delegated_agent: Option<DelegatedAgent>` field to `AgentSession`, initialized to `None`

## 2. Delegation Detection in App

- [x] 2.1 In `crates/conduit-ui/src/app.rs`, in the `AgentEvent::ToolStarted` handler: when `tool_name == "Agent"` and `orchestration_enabled`, parse `arguments` JSON for `subagent_type`; map `"conduit-explore"` → `DelegatedAgent { display_label: "Explore", model: "claude-haiku-4-5", tool_id }` and `"conduit-review"` → `DelegatedAgent { display_label: "Review", model: "claude-haiku-4-5", tool_id }`; set `session.delegated_agent`
- [x] 2.2 In the `AgentEvent::ToolCompleted` handler: if `tool_id` matches `session.delegated_agent.as_ref().map(|d| &d.tool_id)`, clear `session.delegated_agent`
- [x] 2.3 Clear `session.delegated_agent` wherever the session is set to idle/ready/stopped state (stop, interrupt, error transitions) so the chip always reverts

## 3. Status Bar Rendering

- [x] 3.1 Add `delegated_agent: Option<&DelegatedAgent>` field to `StatusBar` in `crates/conduit-ui/src/components/status_bar.rs`
- [x] 3.2 In the mode label rendering path: when `delegated_agent` is `Some`, display `delegated_agent.display_label` instead of `agent_mode.display_name()`
- [x] 3.3 In the model chip rendering path: when `delegated_agent` is `Some`, display the short model name for `delegated_agent.model` (via `ModelRegistry::find_model()`) instead of the session model
- [x] 3.4 Pass `session.delegated_agent.as_ref()` when constructing `StatusBar` in `app.rs`

## 4. Verification

- [x] 4.1 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and confirm all pass
- [x] 4.2 Manual test: enable orchestration, ask Claude to "summarize session.rs", verify status bar shows "Explore" / "haiku" while the Agent tool is running, then reverts to "Build" / orchestrator model when done
- [x] 4.3 Manual test: disable orchestration, confirm Agent tool calls (if any) do not change the status bar
