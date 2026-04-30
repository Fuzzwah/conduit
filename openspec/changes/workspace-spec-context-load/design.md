## Context

When a workspace is created with a spec selected, conduit uses the spec ID only for naming the workspace and branch. After creation, `close_workspace_progress_dialog` opens the workspace and the agent starts with a blank slate.

The submit path (`submit_prompt` → `submit_prompt_for_tab`) already handles first-message agent launch — constructing `AgentStartConfig` and emitting `Effect::StartAgent`. We can hook into `close_workspace_progress_dialog` to call this immediately after workspace open, passing the composed context message.

The relevant state transition: `Effect::CreateWorkspace` (async task) → `AppEvent::WorkspaceCreated` → stored in `AppState` → `close_workspace_progress_dialog` → `open_workspace_with_options` → auto-send.

## Goals / Non-Goals

**Goals:**
- Auto-send a context-load message when a workspace is opened for the first time and was created from an OpenSpec or Specify spec
- Message asks the agent to read spec files and summarize remaining work
- No change to workspaces created without a spec

**Non-Goals:**
- Auto-sending on subsequent re-opens of the same workspace (message fires only at creation time)
- Auto-applying the spec (not jumping to `/opsx:apply`)
- Any changes to the web UI workspace creation flow (TUI only for now)

## Decisions

### Thread `initial_message` through `WorkspaceCreated` rather than storing in `AppState` at effect-dispatch time

The spec info is available in the `Effect::CreateWorkspace` handler, and it could be stored in `AppState` there (before the async task). However, storing it in the `WorkspaceCreated` result struct keeps the causal chain explicit: the message is associated with the workspace that was created, not speculatively with a workspace that might fail to create. If workspace creation fails, no initial message is stored.

Alternative: store in `AppState` when the effect is dispatched, clear on failure. Rejected because it requires two more state mutations and splits the logic across success/failure paths.

### Use `submit_prompt` (not a new Effect variant)

`submit_prompt` already handles the full agent-start path for fresh sessions. Adding a new `Effect::SendInitialMessage` variant would be redundant. Calling `submit_prompt` directly in `close_workspace_progress_dialog` after `open_workspace_with_options` is the minimal path.

### Message format: plain read-and-summarize, not a skill invocation

`/opsx:explore` locks the agent into explore mode (no implementation). `/opsx:apply` jumps straight to implementation. A plain message like "read these files and summarize remaining work" gives the agent context without constraining what comes next — the user can pivot immediately to any direction.

### Store pending message in `AppState` (not pass as function argument)

`open_workspace_with_options` is called from several places (`close_workspace_progress_dialog`, direct key handler). Adding an `initial_message` parameter would require updating all call sites. Storing it in `AppState` as `pending_created_workspace_initial_message` keeps `open_workspace_with_options` unchanged and the auto-send logic local to `close_workspace_progress_dialog`, which is the only creation-time code path.

## Risks / Trade-offs

- **Agent not yet running when message is sent** → `submit_prompt` already handles this; it starts the agent with the message as the first prompt. No additional handling needed.
- **Message fires if user immediately closes and reopens workspace** → The pending message is consumed (`take()`) from `AppState` in `close_workspace_progress_dialog`, so it only fires once — on first open after creation. Subsequent opens use the saved session path and will not re-send.
- **Spec files may not exist yet** → The message is a read request, not a hard assertion. If the files are missing, the agent will say so and the user can redirect. Not a crash risk.
- **Long message visible in chat** → The message is a normal user turn and will appear in chat history. This is intentional — it's transparent about what was auto-sent.
