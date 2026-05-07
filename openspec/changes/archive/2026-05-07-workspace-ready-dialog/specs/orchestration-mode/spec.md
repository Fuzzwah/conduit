## ADDED Requirements

### Requirement: Orchestration default surfaced in workspace-ready config panel
When a new workspace is created successfully, the orchestration state for the first session SHALL be configurable via the workspace-ready config panel's Orchestration row, before the workspace tab is opened. The initial value SHALL be resolved from the defaults chain (workspace override → project override → global config). The Orchestration row SHALL be non-interactive when the selected provider is not Claude.

#### Scenario: Orchestration row reflects workspace override
- **WHEN** the workspace record has `orchestration_enabled = true`
- **THEN** the Orchestration row shows On when the config panel opens

#### Scenario: Orchestration row reflects project default
- **WHEN** the workspace has no `orchestration_enabled` override and the repository has `orchestration_enabled = false`
- **THEN** the Orchestration row shows Off when the config panel opens

#### Scenario: Orchestration row reflects global config
- **WHEN** neither workspace nor repository has an `orchestration_enabled` override
- **AND** `conduit.toml` has `orchestration.enabled_by_default = true`
- **THEN** the Orchestration row shows On when the config panel opens

#### Scenario: Orchestration row non-interactive for non-Claude provider
- **WHEN** the selected provider in the config panel is not Claude
- **THEN** the Orchestration row is dimmed and Space/Left/Right have no effect on it

### Requirement: Project-level provider and model defaults stored in repository record
The `repositories` table SHALL store `default_provider` (TEXT, nullable) and `default_model` (TEXT, nullable) columns. These represent per-project overrides for the provider and model used when opening new sessions in that project's workspaces. NULL means "inherit global config".

#### Scenario: Repository with no provider default uses global config
- **WHEN** a repository has `default_provider = NULL`
- **THEN** the workspace-ready config panel resolves provider from global config

#### Scenario: Repository with provider default uses it
- **WHEN** a repository has `default_provider = "Gemini"`
- **THEN** the workspace-ready config panel shows Gemini as the initial provider selection

#### Scenario: Saving project defaults writes provider and model columns
- **WHEN** the user checks "Set as project default" and confirms the config panel with provider=Gemini, model=gemini-2.5-pro
- **THEN** the repository record has `default_provider = "Gemini"` and `default_model = "gemini-2.5-pro"` after the update
