## 1. Data Model — Repository Provider/Model Defaults

- [x] 1.1 Add `default_provider: Option<String>` and `default_model: Option<String>` fields to `Repository` struct in `crates/conduit-data/src/models.rs`; initialise both to `None` in `Repository::new()` and `Repository::new_with_url()`
- [x] 1.2 Add migration 22 in `crates/conduit-data/src/database.rs`: idempotent `ALTER TABLE repositories ADD COLUMN default_provider TEXT` and `ALTER TABLE repositories ADD COLUMN default_model TEXT`
- [x] 1.3 In `crates/conduit-data/src/repository.rs`: include `default_provider` and `default_model` in INSERT, SELECT, and UPDATE SQL; read both back from rows as `Option<String>` (columns 12 and 13)

## 2. Config Panel State

- [x] 2.1 Add `WorkspaceReadyConfigState` struct to `crates/conduit-ui/src/components/workspace_progress_dialog.rs` with fields: `focused_row: usize`, `provider: AgentType`, `model_id: String`, `mode: AgentMode`, `orchestration_enabled: bool`, `save_as_project_default: bool`
- [x] 2.2 Add `config: Option<WorkspaceReadyConfigState>` field to `WorkspaceProgressDialogState`; ensure `show()` clears it and `hide()` drops it
- [x] 2.3 Update `WorkspaceProgressDialogState::finish()` to accept initial config values `(provider, model_id, orchestration_enabled)` and populate `self.config = Some(WorkspaceReadyConfigState { ... })`; mode defaults to `AgentMode::Build`
- [x] 2.4 Add helper methods to `WorkspaceProgressDialogState`: `config_focused_row()`, `config_row_count()`, `move_focus_up()`, `move_focus_down()`, `toggle_mode()`, `toggle_orchestration()`, `toggle_save_default()`, `is_orchestration_applicable()` (returns true only when provider is Claude), `is_plan_mode_applicable()` (delegates to `AgentCapabilities`)

## 3. Config Panel Rendering

- [x] 3.1 Update `WorkspaceProgressDialog::dialog_height()` to return ~27 when `state.config.is_some()` (borders + log + separator + 4 rows + save-default row + gaps + button)
- [x] 3.2 In `WorkspaceProgressDialog::render()`, after the status line, when `state.config.is_some()` render: a horizontal separator line, then four rows (Provider, Model, Mode, Orchestration), then the "Set as project default" checkbox row, then a gap, then the Continue button
- [x] 3.3 Each row renders as: left-aligned label (10 chars wide) + value area; focused row uses `accent_primary()` highlight on the label; dimmed rows use `text_muted()` for both label and value
- [x] 3.4 Provider row: shows provider name inline (e.g. `Claude`, `Gemini`)
- [x] 3.5 Model row: shows model ID truncated to fit dialog width
- [x] 3.6 Mode row: shows `[ Build ]  [ Plan ]` with active option highlighted; Plan option text is muted when `!is_plan_mode_applicable()`
- [x] 3.7 Orchestration row: shows `[ Off ]  [ On ]` with active option highlighted; entire row (label + values) is muted when `!is_orchestration_applicable()`
- [x] 3.8 Save-default row: shows `[ ] Set as project default` (checkbox glyph `[x]` when checked); uses `text_muted()` styling
- [x] 3.9 Update instructions footer in the config panel state: `vec![("Enter", "Continue"), ("Esc", "Continue"), ("↑↓", "Navigate")]`

## 4. App State — Pending Session Config

- [x] 4.1 Add `PendingSessionConfig` struct to `crates/conduit-ui/src/app_state.rs` with fields: `provider: AgentType`, `model_id: String`, `mode: AgentMode`, `orchestration_enabled: bool`
- [x] 4.2 Add `pending_workspace_session_config: Option<PendingSessionConfig>` field to `AppState`; initialise to `None`

## 5. Populating Config Panel on Workspace Created

- [x] 5.1 In `crates/conduit-ui/src/app.rs`, in the `AppEvent::WorkspaceCreated { result: Ok(created) }` branch: after storing `pending_created_workspace_id`, load the workspace and repository records from DAO and resolve provider (repo `default_provider` → global), model (repo `default_model` → global default for provider), and orchestration (workspace `orchestration_enabled` → repo `orchestration_enabled` → global config)
- [x] 5.2 Call `self.state.workspace_progress_dialog_state.finish(provider, model_id, orchestration_enabled)` (updated signature from task 2.3) instead of the old `finish()`

## 6. Input Handling for Config Panel

- [x] 6.1 In `crates/conduit-ui/src/app/app_input.rs`, in the `InputMode::CreatingWorkspace` branch: when `dialog_state.config.is_some()` and `dialog_state.complete`, handle `Up`/`Down` → `move_focus_up()`/`move_focus_down()`; `Space` or `Left`/`Right` → toggle the focused row (mode, orchestration, save-default); `Enter` → if focused row is Provider open provider selector, else if Model open model selector, else call `close_workspace_progress_dialog()`; `Esc` → call `close_workspace_progress_dialog()`
- [x] 6.2 Add a new `ModelPickerContext::WorkspaceReadyConfig` variant in `crates/conduit-ui/src/app_state.rs` (alongside the existing `SessionSelection` variant)
- [x] 6.3 When opening the provider selector from the config panel, set `input_mode = InputMode::SelectingProviders` and store context so the close path returns to `InputMode::CreatingWorkspace`
- [x] 6.4 When opening the model selector from the config panel, set `input_mode = InputMode::SelectingModel` with `model_picker_context = ModelPickerContext::WorkspaceReadyConfig` and `allowed_providers` filtered to the currently selected provider
- [x] 6.5 In the provider selector close path (in `app.rs`): when `model_picker_context == WorkspaceReadyConfig`, update `dialog_state.config.provider` with the new selection, reset `dialog_state.config.model_id` to the default model for the new provider, return `input_mode = InputMode::CreatingWorkspace`
- [x] 6.6 In the model selector close path (in `app.rs`): when `model_picker_context == WorkspaceReadyConfig`, update `dialog_state.config.model_id` with the selected model, return `input_mode = InputMode::CreatingWorkspace`

## 7. Applying Config on Dialog Close

- [x] 7.1 In `close_workspace_progress_dialog()` in `crates/conduit-ui/src/app.rs`: if `workspace_progress_dialog_state.config` has a value, extract `(provider, model_id, mode, orchestration_enabled, save_as_project_default)` before hiding the dialog
- [x] 7.2 Store the extracted values as `self.state.pending_workspace_session_config = Some(PendingSessionConfig { provider, model_id, mode, orchestration_enabled })`
- [x] 7.3 After `open_workspace_with_options(workspace_id, close_sidebar)`, apply `pending_workspace_session_config` to the active session: set `session.agent_type = provider`, `session.model = Some(model_id)`, `session.agent_mode = mode`, `session.orchestration_enabled = orchestration_enabled`; clear `pending_workspace_session_config`
- [x] 7.4 If `save_as_project_default` is true: load the repository record for the created workspace, set `repo.default_provider = Some(provider.to_string())`, `repo.default_model = Some(model_id)`, `repo.orchestration_enabled = Some(orchestration_enabled)`, call `repository_dao.update(&repo)` (log and swallow errors)

## 8. Revert Auto-Dismiss (Undo PR #234 Logic)

- [x] 8.1 In `crates/conduit-ui/src/app.rs`, in the `AppEvent::WorkspaceCreated { result: Ok(_) }` branch: remove the `has_meaningful_content` branch that called `close_workspace_progress_dialog()` automatically — `finish()` is now always called on success (config panel always shown)
- [x] 8.2 Remove or update the `has_meaningful_content` field in `WorkspaceProgressDialogState` if it is no longer needed; update `push()` accordingly

## 9. Verification

- [x] 9.1 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and confirm all pass
- [x] 9.2 Manual test: create a new workspace; confirm config panel appears with correct defaults; change provider and confirm model resets; confirm Continue opens workspace with chosen settings
- [x] 9.3 Manual test: check "Set as project default", confirm; create a second workspace in same project; confirm config panel pre-fills with saved defaults
- [x] 9.4 Manual test: select a non-Claude provider; confirm Orchestration row is dimmed and non-interactive
- [x] 9.5 Manual test: select a provider without plan mode support; confirm Plan option is dimmed
- [x] 9.6 Manual test: trigger a workspace creation failure; confirm error dialog has no config rows
