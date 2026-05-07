## ADDED Requirements

### Requirement: Config panel appears after successful workspace creation
After workspace creation succeeds, the workspace progress dialog SHALL transition from its "complete" state (bare Continue button) into a configuration panel. The panel SHALL always be shown on success; there is no auto-dismiss path for clean creations.

#### Scenario: Config panel shown after clean creation
- **WHEN** workspace creation completes without error and no remote changes were fetched
- **THEN** the dialog shows the config panel (not a bare Continue button)

#### Scenario: Config panel shown after creation with remote changes
- **WHEN** workspace creation completes without error and git output was streamed
- **THEN** the dialog shows the config panel below the progress log

#### Scenario: No config panel on error
- **WHEN** workspace creation fails with an error
- **THEN** the dialog shows only the red-border error state and a dismiss button; no config rows are displayed

### Requirement: Config panel exposes four rows — Provider, Model, Mode, Orchestration
The config panel SHALL display four focusable rows: Provider, Model, Mode, and Orchestration. The panel SHALL be keyboard-driven: Up/Down arrows move focus between rows; Enter or Space activates the focused row; Tab cycles focus forward.

#### Scenario: Arrow key navigation between rows
- **WHEN** the config panel is visible and the user presses Down
- **THEN** focus moves to the next row (wrapping from last to first)

#### Scenario: Up arrow wraps from first to last row
- **WHEN** focus is on the first row and the user presses Up
- **THEN** focus moves to the last interactive row

### Requirement: Provider row opens the existing provider selector modal
The Provider row SHALL display the currently selected provider name inline. Pressing Enter on the Provider row SHALL open the existing `ProviderSelectorState` modal. When the modal closes, focus SHALL return to the Provider row in the config panel (input mode returns to `InputMode::CreatingWorkspace`).

#### Scenario: Provider row shows current selection
- **WHEN** the config panel is displayed
- **THEN** the Provider row shows the name of the currently resolved provider

#### Scenario: Activating Provider row opens modal
- **WHEN** the user presses Enter on the focused Provider row
- **THEN** the provider selector modal opens

#### Scenario: Provider modal close returns to config panel
- **WHEN** the user dismisses the provider selector modal (Esc or selection)
- **THEN** focus returns to the config panel Provider row

### Requirement: Model row opens the existing model selector modal
The Model row SHALL display the currently selected model name inline. Pressing Enter on the Model row SHALL open the existing `ModelSelectorState` modal filtered to the currently selected provider. When the modal closes, focus SHALL return to the Model row in the config panel.

#### Scenario: Model row shows current selection
- **WHEN** the config panel is displayed
- **THEN** the Model row shows the model ID or name for the resolved provider/model combination

#### Scenario: Activating Model row opens modal
- **WHEN** the user presses Enter on the focused Model row
- **THEN** the model selector modal opens, filtered to the currently selected provider

#### Scenario: Model reverts to provider default when provider changes
- **WHEN** the user changes the provider via the Provider row
- **THEN** the Model row's selection is updated to the default model for the new provider

### Requirement: Mode row toggles Build/Plan inline
The Mode row SHALL show two options — Build and Plan — with the active option highlighted. Pressing Space or Left/Right on the Mode row SHALL toggle between Build and Plan. If the selected provider does not support plan mode (`AgentCapabilities::supports_plan_mode == false`), the Plan option SHALL be visually dimmed and non-interactive (input ignored); the effective mode SHALL remain Build.

#### Scenario: Mode toggles on Space
- **WHEN** the Mode row is focused and the user presses Space
- **THEN** the active mode toggles between Build and Plan

#### Scenario: Plan option dimmed for non-plan-mode providers
- **WHEN** the selected provider does not support plan mode
- **THEN** the Plan option is rendered with muted/dimmed styling

#### Scenario: Plan mode input ignored for non-plan-mode providers
- **WHEN** the selected provider does not support plan mode and the user presses Space on the Mode row
- **THEN** the mode remains Build; no toggle occurs

### Requirement: Orchestration row toggles On/Off inline, greyed when not applicable
The Orchestration row SHALL show two options — Off and On — with the active option highlighted. Pressing Space or Left/Right on the Orchestration row SHALL toggle between Off and On. If the selected provider is not Claude, the entire Orchestration row SHALL be visually dimmed and non-interactive.

#### Scenario: Orchestration toggles on Space for Claude provider
- **WHEN** the selected provider is Claude and the Orchestration row is focused
- **AND** the user presses Space
- **THEN** orchestration toggles between On and Off

#### Scenario: Orchestration row dimmed for non-Claude provider
- **WHEN** the selected provider is not Claude
- **THEN** the Orchestration row is rendered with muted/dimmed styling and input is ignored

### Requirement: Initial config selections resolved from defaults chain
When the config panel is populated, the initial values for provider, model, mode, and orchestration SHALL be resolved in this order: (1) workspace-level overrides (orchestration only, from `workspace.orchestration_enabled`), (2) project-level overrides (provider, model, orchestration from `repository.default_provider`, `repository.default_model`, `repository.orchestration_enabled`), (3) global config (`preferred_provider_for_new_sessions`, `default_model_for(provider)`, `orchestration.enabled_by_default`). Mode always defaults to Build unless overridden in future.

#### Scenario: Project default provider used when set
- **WHEN** the repository record has `default_provider = "Gemini"`
- **THEN** the Provider row shows Gemini on panel open

#### Scenario: Global config used when no project default
- **WHEN** the repository record has no `default_provider`
- **AND** global config has `preferred_provider = "Claude"`
- **THEN** the Provider row shows Claude on panel open

#### Scenario: Workspace orchestration default takes precedence over project
- **WHEN** the workspace record has `orchestration_enabled = true` and the repository has `orchestration_enabled = false`
- **THEN** the Orchestration row shows On on panel open

### Requirement: "Set as project default" checkbox persists provider, model, and orchestration
A "Set as project default" checkbox SHALL appear below the four config rows. When checked and the user confirms with the Continue button, the system SHALL write `default_provider`, `default_model`, and `orchestration_enabled` back to the repository record via `repository_dao.update()`. The checkbox SHALL default to unchecked.

#### Scenario: Defaults not saved when checkbox unchecked
- **WHEN** the user proceeds with the Continue button and the checkbox is unchecked
- **THEN** the repository record's `default_provider`, `default_model`, and `orchestration_enabled` are unchanged

#### Scenario: Defaults saved when checkbox checked
- **WHEN** the user checks "Set as project default" and confirms
- **THEN** the repository record is updated with the selected provider, model, and orchestration values

#### Scenario: Checkbox defaults to unchecked
- **WHEN** the config panel is displayed
- **THEN** the "Set as project default" checkbox is unchecked

### Requirement: Continue button applies config and opens workspace
The Continue button (or pressing Enter when the Continue button is focused) SHALL dismiss the dialog and open the new workspace tab with the chosen provider, model, mode, and orchestration applied to the session. The existing `close_workspace_progress_dialog()` flow (open workspace tab, send initial message) SHALL remain intact. Chosen settings SHALL be stored in `AppState.pending_workspace_session_config` and applied to the newly opened tab's session before the first render.

#### Scenario: Workspace opens with chosen provider
- **WHEN** the user selects Gemini and confirms
- **THEN** the new workspace tab uses the Gemini agent

#### Scenario: Workspace opens with chosen orchestration state
- **WHEN** the user sets Orchestration to On and confirms
- **THEN** the new session has `orchestration_enabled = true`

#### Scenario: Initial message still sent after config
- **WHEN** the workspace was created with an initial message (e.g. from a spec)
- **AND** the user confirms the config panel
- **THEN** the initial message is auto-submitted after the workspace tab opens
