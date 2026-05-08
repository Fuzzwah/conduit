## Why

Agent-generated code ships without independent review — the same agent that wrote the changes is also the one deciding they're correct. An adversarial reviewer with a separate context and a mandate to find problems provides a genuine second opinion before changes are committed or merged.

## What Changes

- New `conduit-adversarial-review` orchestration sub-agent definition written to `~/.claude/agents/` alongside the existing `conduit-explore` and `conduit-review` agents
- New `adversarial_review_enabled` and `adversarial_review_model` settings on both workspace and repository records (database migration)
- New toggle + model input rows in the Workspace Ready config panel (the dialog shown after workspace creation)
- New `AdversarialReview` variant on `SuggestedAction`; appears in the Work Complete action list when enabled and changes exist
- New `/adversarial-review` slash command (Claude only) that injects the review prompt into the active session at any time
- Review is performed by the sub-agent against either the open PR diff (`gh pr diff`) or the local branch diff (`git diff`); findings are returned to the primary agent, which can then act on them

## Capabilities

### New Capabilities

- `adversarial-review-config`: Per-workspace and per-project settings controlling whether adversarial review is enabled and which model the reviewer uses; configured in the workspace ready dialog and persisted to the database
- `adversarial-review-trigger`: The two entry points for initiating a review — the `/adversarial-review` slash command and the Work Complete dialog action — and the prompt-injection flow that feeds the review task to the primary agent

### Modified Capabilities

- `workspace-ready-config`: Add adversarial review toggle and model input rows to the existing workspace creation config panel
- `orchestration-mode`: Extend orchestration agent management to write/update the `conduit-adversarial-review` agent definition when a session starts with the feature enabled

## Impact

- **Database**: new migration adding two nullable columns to `workspaces` and `repositories` tables
- **`conduit-data`**: `Workspace` and `Repository` structs, workspace/repository SQL queries
- **`conduit-agent`**: `orchestration.rs` (new agent def), `runner.rs` (new `AgentStartConfig` field), `claude.rs` (pass config to `ensure_orchestration_agents`)
- **`conduit-git`**: `SuggestedAction` enum, `suggested_actions_for()` builder
- **`conduit-ui`**: session state, workspace ready config dialog component, Work Complete state machine and dialog component, `app.rs` prompt building and command dispatch
- **`conduit-resolver`**: new `ConduitCommand::AdversarialReview` variant and builtin registration
- **`conduit-web`**: Work Complete preflight endpoint threads workspace settings into suggested actions
