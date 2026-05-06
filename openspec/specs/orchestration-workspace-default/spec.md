## ADDED Requirements

### Requirement: Workspace and project store an orchestration default
Each workspace and each project (repository) SHALL store an optional orchestration default (`None` = inherit, `true` = force on, `false` = force off). This value SHALL be persisted in the database and survive restarts.

#### Scenario: Default is inherit on creation
- **WHEN** a workspace or project is created
- **THEN** its orchestration default SHALL be `None` (inherit)

#### Scenario: Value persists across restarts
- **WHEN** a workspace orchestration default is set to `true`
- **AND** conduit is restarted
- **THEN** the workspace orchestration default SHALL still be `true`

### Requirement: New Claude sessions inherit orchestration default from workspace, then project, then global config
When a new Claude session is created for a workspace, the session's `orchestration_enabled` SHALL be resolved in priority order: workspace override → project override → global config default.

#### Scenario: Workspace override wins
- **WHEN** a workspace has `orchestration_enabled = true`
- **AND** the global config default is `false`
- **AND** a new Claude session is created for that workspace
- **THEN** `session.orchestration_enabled` SHALL be `true`

#### Scenario: Project override used when workspace inherits
- **WHEN** a workspace has `orchestration_enabled = None`
- **AND** its parent project has `orchestration_enabled = false`
- **AND** the global config default is `true`
- **AND** a new Claude session is created for that workspace
- **THEN** `session.orchestration_enabled` SHALL be `false`

#### Scenario: Global config used when both inherit
- **WHEN** both workspace and project have `orchestration_enabled = None`
- **AND** the global config default is `true`
- **AND** a new Claude session is created for that workspace
- **THEN** `session.orchestration_enabled` SHALL be `true`

### Requirement: Sidebar hotkey cycles workspace or project orchestration default
Pressing `M-S-o` while a workspace or project node is focused in the sidebar SHALL cycle that node's orchestration default through: `None` (inherit) → `true` (force on) → `false` (force off) → `None` (inherit).

#### Scenario: Cycle from inherit to on
- **WHEN** a workspace node is focused in the sidebar
- **AND** its orchestration default is `None`
- **AND** the user presses `M-S-o`
- **THEN** the workspace orchestration default SHALL be set to `true`
- **AND** a confirmation message SHALL be shown

#### Scenario: Cycle from on to off
- **WHEN** a workspace node is focused in the sidebar
- **AND** its orchestration default is `true`
- **AND** the user presses `M-S-o`
- **THEN** the workspace orchestration default SHALL be set to `false`

#### Scenario: Cycle from off back to inherit
- **WHEN** a workspace node is focused
- **AND** its orchestration default is `false`
- **AND** the user presses `M-S-o`
- **THEN** the workspace orchestration default SHALL be set to `None`

#### Scenario: Same cycle applies to project nodes
- **WHEN** a project (repository) node is focused in the sidebar
- **AND** the user presses `M-S-o`
- **THEN** the project's orchestration default SHALL cycle identically to the workspace case

### Requirement: Session hotkey opens orchestration selector
Pressing `M-S-o` while a Claude session tab is active SHALL open the orchestration selector modal for that session (same as `Action::ShowOrchestrationSelector`).

#### Scenario: Hotkey opens selector in session view
- **WHEN** the user is in a Claude session tab (not in the sidebar)
- **AND** presses `M-S-o`
- **THEN** the orchestration selector modal SHALL open for the current session
