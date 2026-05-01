## Why

When a user creates a workspace by selecting an OpenSpec or Specify spec, the spec is used only for naming — the agent starts with no context about the work ahead. The user must manually prompt the agent to read the spec, which is friction that defeats the purpose of selecting a spec at creation time.

## What Changes

- When a workspace is created with an OpenSpec change selected, the agent automatically receives an initial message asking it to read `openspec/changes/{change_id}/` (proposal.md, design.md, tasks.md) and summarize remaining work.
- When a workspace is created with a Specify spec selected, the agent automatically receives an initial message asking it to read `.specify/specs/{spec_id}/tasks.md` and summarize remaining work.
- Workspaces created without a spec are unaffected — no initial message is sent.

## Capabilities

### New Capabilities

- `workspace-spec-context-load`: Automatically sends a context-load message to the agent when a workspace is opened for the first time and was created from a spec, priming it with the spec content.

### Modified Capabilities

<!-- none -->

## Impact

- `src/ui/events.rs` — `WorkspaceCreated` struct gains an `initial_message: Option<String>` field
- `src/ui/app_state.rs` — `AppState` gains a `pending_created_workspace_initial_message: Option<String>` field
- `src/ui/app.rs` — workspace creation effect composes the message; `AppEvent::WorkspaceCreated` handler stores it; `close_workspace_progress_dialog` auto-sends it via `submit_prompt`
- No new dependencies, no API changes, no breaking changes
