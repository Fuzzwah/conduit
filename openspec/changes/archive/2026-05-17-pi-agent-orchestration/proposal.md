## Why

Pi Agent is a first-class agent option in Conduit, but it currently lacks orchestration and adversarial review features that Claude Code supports. These features let the main agent delegate sub-tasks (exploration, code review) to specialized sub-agents — improving efficiency and keeping the main context window lean. Adding them makes Pi a fully capable alternative to Claude in Conduit.

## What Changes

- Add `AgentType::Pi` to `is_orchestration_applicable()` so orchestration/adversarial review controls are no longer greyed out in the workspace config dialog
- Pass orchestration and adversarial review configuration to the Pi runner (currently only passed for Claude)
- Create a Pi extension (TypeScript) that registers an `Agent` tool — this tool spawns sub-sessions with configurable models to handle delegated tasks
- Write Pi-native skill files (`.pi/skills/conduit-*/SKILL.md`) so Pi discovers and knows how to use the explore, review, and adversarial-review sub-agents
- Inject orchestration instructions into Pi's system prompt via `--append-system-prompt`
- Update the Pi runner's CLI argument construction to pass `--extension` and `--skill` flags when orchestration is enabled
- Show orchestration badge in the status bar for Pi sessions (currently Claude-only)
- Detect Agent tool calls from Pi in the events handler for the delegation badge in the UI

## Capabilities

### New Capabilities
- `pi-sub-agent-delegation`: The Pi agent can delegate sub-tasks (exploration, review) to sub-sessions running different models via a custom Agent tool
- `pi-orchestration-skills`: Pi discovers conduit-explore, conduit-review, and conduit-adversarial-review skills from `.pi/skills/` directories
- `pi-adversarial-review`: Pi runs an adversarial code review pass using a dedicated skill and configurable model

### Modified Capabilities

- (none — orchestration is new functionality, not modifying existing spec behavior)

## Impact

- **crates/conduit-agent/src/orchestration.rs**: Add `ensure_pi_orchestration_skills()` alongside the existing Claude agent setup
- **crates/conduit-agent/src/pi.rs**: Pass `--extension`, `--skill`, and `--append-system-prompt` flags; inject orchestration and adversarial review config
- **New file: crates/conduit-agent/src/pi-orchestration.ts**: TypeScript extension implementing the `Agent` tool
- **crates/conduit-ui/src/components/workspace_progress_dialog.rs**: Update `is_orchestration_applicable()`
- **crates/conduit-ui/src/app/app_agent_events.rs**: Remove Claude-only guard on config propagation; handle Pi Agent tool calls for delegation badge
- **crates/conduit-ui/src/session.rs**: Show orchestration badge for Pi
