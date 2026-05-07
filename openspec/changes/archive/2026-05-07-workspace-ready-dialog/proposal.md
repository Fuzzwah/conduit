## Why

When a new workspace is created, the user is immediately dropped into a session with whatever provider, model, mode, and orchestration settings happen to be active — there is no moment to configure the session before it starts. This dialog gives users a natural pause point to make those choices before the agent begins work, using the same defaults resolution chain already established by PR #231.

## What Changes

- The workspace creation progress dialog, on successful completion, transitions into a "Workspace Ready" configuration panel instead of showing a bare "Continue" button.
- The config panel exposes four inline rows: **Provider**, **Model**, **Mode** (Build / Plan), and **Orchestration** (On / Off). The Orchestration row is greyed out when the selected provider does not support it.
- Initial selections are resolved from the existing chain: workspace default → project (repository) default → global config.
- A **"Set as project default"** toggle at the bottom of the panel saves the chosen provider, model, and orchestration setting back to the repository record before opening the workspace.
- On workspace creation **error**, no config panel is shown — the dialog retains its existing red-border error state with a dismiss button only.
- The existing `close_workspace_progress_dialog()` flow (open workspace tab, send initial message) is unchanged; the config panel merely feeds settings into it.

## Capabilities

### New Capabilities

- `workspace-ready-config`: Inline post-creation configuration panel embedded in the workspace progress dialog, allowing provider, model, mode, and orchestration to be chosen before the first session starts, with an option to persist choices as project defaults.

### Modified Capabilities

- `workspace-creation-prelude`: The creation prelude now always ends at the config panel (not a bare dismiss) on success; navigation and keyboard handling during `InputMode::CreatingWorkspace` must account for the new focusable rows.
- `orchestration-mode`: Orchestration selection now surfaces inside the workspace-ready config panel in addition to the existing `M-o` session-level selector; the project-level save path added here extends the per-project default established in PR #231.

## Impact

- `crates/conduit-ui/src/components/workspace_progress_dialog.rs` — extend `WorkspaceProgressDialogState` with config-panel state (focused row, provider choice, model choice, mode choice, orchestration choice, save-as-default flag); extend `WorkspaceProgressDialog` widget to render the panel when complete and not errored.
- `crates/conduit-ui/src/app/app_input.rs` — extend `InputMode::CreatingWorkspace` input handling for arrow-key row navigation and Enter/Space row activation in the config panel.
- `crates/conduit-ui/src/app.rs` — populate config panel from resolved defaults on workspace creation success; thread chosen provider/model/mode/orchestration into `open_workspace_with_options` and session creation; save repo defaults when "Set as project default" is checked.
- `crates/conduit-data/src/repository.rs` / `models.rs` — add `default_provider: Option<String>` and `default_model: Option<String>` fields and migration (orchestration column already exists from PR #231).
- `crates/conduit-data/src/database.rs` — migration 22 for the two new repository columns.
- No breaking changes to existing keybindings, CLI, or external APIs.
