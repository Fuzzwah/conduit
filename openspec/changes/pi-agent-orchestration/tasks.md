## 1. Un-gate orchestration controls for Pi

- [ ] 1.1 In `crates/conduit-ui/src/components/workspace_progress_dialog.rs`, change `is_orchestration_applicable()` to return `true` for `AgentType::Pi` (alongside Claude)
- [ ] 1.2 In `crates/conduit-ui/src/app/app_agent_events.rs`, remove the `agent_type == AgentType::Claude` guard around orchestration/adversarial config propagation so Pi sessions also pass `orchestration_enabled` and `adversarial_review` config to `AgentStartConfig`
- [ ] 1.3 In `crates/conduit-ui/src/session.rs`, update the status bar orchestration badge logic to show for Pi sessions (not just Claude)

## 2. Create the Pi Agent tool extension

- [ ] 2.1 Design the extension file `crates/conduit-agent/src/agent-tool.ts` that registers an `Agent` tool using `pi.registerTool()`. The tool accepts `subagent_type` (string) and optional `task` (string) parameters.
- [ ] 2.2 Implement sub-agent spawning in the extension using `createAgentSession()` from `@mariozechner/pi-coding-agent`. The sub-agent model is determined by the skill definition.
- [ ] 2.3 Add error handling: SDK/model errors, timeout, empty results. Return clear error messages to the calling agent.
- [ ] 2.4 Ensure the tool result matches the format Claude's `Agent` tool uses (text content block) so Conduit's existing event handling works.

## 3. Write Pi-native orchestration skill files

- [ ] 3.1 In `crates/conduit-agent/src/orchestration.rs`, add `ensure_pi_orchestration_skills()` that writes three skill directories under `~/.pi/agent/skills/`:
  - `conduit-explore/SKILL.md`: Fast codebase exploration skill
  - `conduit-review/SKILL.md`: Quick diff/code review skill
  - `conduit-adversarial-review/SKILL.md`: Adversarial review skill (with configurable model in frontmatter)
- [ ] 3.2 Each skill SHALL use the Agent Skills standard frontmatter (`name`, `description`, and a `model` metadata field for the sub-agent model)

## 4. Update the Pi runner for orchestration

- [x] 4.1 In `crates/conduit-agent/src/pi.rs`, update `build_command()` to accept the extension path and skill paths:
  - Write the extension file to `~/.conduit/pi-agent-extensions/agent-tool.ts`
  - Add `--extension <path>` for the extension
  - Add `--skill <path>` for each orchestration skill directory
  - Add `--append-system-prompt <text>` for orchestration instructions
- [x] 4.2 In `crates/conduit-agent/src/pi.rs`, update `start()` to call `ensure_pi_orchestration_skills()` when `config.orchestration_enabled` is true
- [x] 4.3 Inject orchestration instructions via `--append-system-prompt` when orchestration is enabled

## 5. Update agent event handling for Pi delegation

- [x] 5.1 In `crates/conduit-ui/src/app/app_agent_events.rs`, extend the sub-agent delegation detection (currently at line ~293) to also handle `Agent` tool calls from Pi sessions (same tool name, same subagent_type values)
- [x] 5.2 In `crates/conduit-ui/src/app/app_agent_events.rs`, replace the hardcoded model (`"claude-haiku-4-5"`) in the delegation display with the actual configured model from the session's `adversarial_review_model` field (or a new per-skill model lookup). For Claude sessions, fall back to `"claude-haiku-4-5"` when no model is explicitly configured. For Pi sessions, fall back to `"gemini-2.5-flash"` when no model is explicitly configured.

## 6. Tests

- [x] 6.1 Add/update tests in `crates/conduit-agent/src/models.rs` if any static model list assertions change
- [x] 6.2 Verify existing capabilities tests in `crates/conduit-ui/src/session.rs` still pass
- [x] 6.3 Verify the workspace_progress_dialog rendering tests handle Pi correctly