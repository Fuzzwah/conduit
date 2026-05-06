## 1. Agent Module

- [x] 1.1 Create `crates/conduit-agent/src/orchestration.rs` with `EXPLORE_AGENT_DEF` and `REVIEW_AGENT_DEF` string constants (frontmatter + system prompt for each)
- [x] 1.2 Implement `pub fn ensure_orchestration_agents() -> std::io::Result<()>` — resolves `~/.claude/agents/`, creates dir if missing, writes each file only if content differs
- [x] 1.3 Implement `pub fn orchestration_instructions() -> &'static str` — returns the instruction block to append to the session prompt
- [x] 1.4 Expose `pub mod orchestration` in `crates/conduit-agent/src/lib.rs`

## 2. AgentStartConfig

- [x] 2.1 Add `pub orchestration_enabled: bool` field to `AgentStartConfig` in `crates/conduit-agent/src/runner.rs`
- [x] 2.2 Add `pub fn with_orchestration(mut self, enabled: bool) -> Self` builder method

## 3. Claude Runner Integration

- [x] 3.1 In `ClaudeCodeRunner::start()` (`crates/conduit-agent/src/claude.rs`): if `config.orchestration_enabled`, call `orchestration::ensure_orchestration_agents()?` and append `orchestration::orchestration_instructions()` to the prompt (separated by `\n\n---\n`)

## 4. Config

- [x] 4.1 Add `OrchestrationConfig { enabled_by_default: bool }` struct to `crates/conduit-config/src/settings.rs` with `#[serde(default)]` on the field and `Default` impl (`enabled_by_default: false`)
- [x] 4.2 Add `pub orchestration: OrchestrationConfig` field to `Config` with `#[serde(default)]`

## 5. Session State

- [x] 5.1 Add `pub orchestration_enabled: bool` to `AgentSession` in `crates/conduit-ui/src/session.rs`
- [x] 5.2 Initialize `orchestration_enabled` from `config.orchestration.enabled_by_default` where `AgentSession` is constructed

## 6. TUI Selector Component

- [x] 6.1 Create `crates/conduit-ui/src/components/orchestration_selector.rs` — modal list with "Enabled" / "Disabled" options, following the `ReasoningSelector` pattern
- [x] 6.2 Expose `orchestration_selector` in `crates/conduit-ui/src/components/mod.rs`
- [x] 6.3 Add `Action::ShowOrchestrationSelector` and `Action::SetOrchestration(bool)` variants (wherever other session actions live)
- [x] 6.4 In `crates/conduit-ui/src/app.rs`: handle `ShowOrchestrationSelector` (show modal, Claude sessions only), handle `SetOrchestration(bool)` (update `AgentSession.orchestration_enabled`), and map `orchestration_enabled` into `AgentStartConfig` when starting a session
- [x] 6.5 Add orchestration toggle entry to the command palette

## 7. Wiring AgentStartConfig

- [x] 7.1 In the session start path (TUI `app.rs` and/or web `ws/handler.rs`), pass `config.with_orchestration(session.orchestration_enabled)` when building `AgentStartConfig` for Claude sessions

## 8. Verification

- [x] 8.1 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and confirm all pass
- [x] 8.2 Manual test: start a Claude session with orchestration ON, verify `~/.claude/agents/conduit-explore.md` and `conduit-review.md` exist and contain the expected content
- [x] 8.3 Manual test: ask Claude to summarize a file; verify it delegates to `conduit-explore` via the Agent tool
- [x] 8.4 Manual test: toggle orchestration OFF, start a new session, verify no orchestration instruction block appears in the prompt
