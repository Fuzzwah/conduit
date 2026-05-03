## ADDED Requirements

### Requirement: Per-server MCP configuration at project scope
The system SHALL store a list of disabled MCP server names per project (repository). All servers not in the list SHALL be considered enabled. An empty list means all servers are enabled.

#### Scenario: Default state — all servers enabled
- **WHEN** a project has never had MCP configuration changed
- **THEN** `mcp_disabled_servers` is `[]` and all detected servers are enabled

#### Scenario: Disable specific server at project level
- **WHEN** user opens the MCP dialog, selects Project scope, toggles a server off, and saves
- **THEN** that server's name is added to the project's `mcp_disabled_servers` list

#### Scenario: Re-enable a disabled server at project level
- **WHEN** user opens the MCP dialog, selects Project scope, toggles a previously disabled server on, and saves
- **THEN** that server's name is removed from the project's `mcp_disabled_servers` list

### Requirement: Per-server MCP configuration at workspace scope
The system SHALL store a separate list of disabled MCP server names per workspace. When a workspace has no saved configuration (`NULL`), the project's disabled list SHALL be used. When a workspace has a saved list (even `[]`), it SHALL fully replace the project list for that workspace.

#### Scenario: Workspace inherits project config before first save
- **WHEN** a workspace has never had MCP configuration saved
- **THEN** the effective disabled list is the project's `mcp_disabled_servers`

#### Scenario: Workspace overrides project config after save
- **WHEN** user saves a workspace-scoped MCP configuration
- **THEN** the workspace's own `mcp_disabled_servers` list is used regardless of the project list

#### Scenario: Workspace re-enables a server disabled at project level
- **WHEN** a server is in the project's disabled list AND the user saves a workspace config that does not include that server
- **THEN** that server is enabled for sessions in that workspace

### Requirement: MCP dialog with Project/Workspace scope tabs
The system SHALL present a unified MCP configuration dialog with two scope tabs: Project and Workspace. The dialog SHALL list all detected MCP servers with their enabled/disabled state for the selected scope.

#### Scenario: Dialog opens with Project tab default from project sidebar node
- **WHEN** user presses M-S-m with a project (repository) node highlighted in the sidebar
- **THEN** the dialog opens with the Project tab selected

#### Scenario: Dialog opens with Workspace tab default from workspace sidebar node
- **WHEN** user presses M-S-m with a workspace node highlighted in the sidebar
- **THEN** the dialog opens with the Workspace tab selected

#### Scenario: Dialog opens with Workspace tab default from active session
- **WHEN** user presses M-S-m while the active tab is a workspace session (Chat or Scrolling context)
- **THEN** the dialog opens with the Workspace tab selected and the active workspace's configuration loaded

#### Scenario: Switching tabs within dialog
- **WHEN** user switches between Project and Workspace tabs in the open dialog
- **THEN** the server list updates to show the configuration for the newly selected scope

#### Scenario: Workspace tab pre-populated from project on first open
- **WHEN** user opens the Workspace tab for a workspace with no saved configuration
- **THEN** the server list shows the project's current configuration as the starting state (visual aid only — nothing is saved until the user explicitly saves)

#### Scenario: No servers detected
- **WHEN** the dialog opens and no `.mcp.json` or `.codex/config.toml` is found in the project
- **THEN** the dialog displays a message "No MCP servers detected" and no list items are shown

### Requirement: Hotkey available from session context
The system SHALL allow the M-S-m hotkey to fire `Action::ManageMcp` from the Chat and Scrolling key contexts in addition to the Sidebar context.

#### Scenario: Hotkey fires from Chat context
- **WHEN** user presses M-S-m while in the main session chat input area
- **THEN** the MCP dialog opens with Workspace scope and the current workspace's configuration

#### Scenario: Hotkey fires from Scrolling context
- **WHEN** user presses M-S-m while scrolling through session history
- **THEN** the MCP dialog opens with Workspace scope and the current workspace's configuration

### Requirement: MCP enforcement uses per-server disabled list
The system SHALL disable only the specific MCP servers in the effective disabled list when launching an agent session. Servers not in the list SHALL remain enabled.

#### Scenario: Codex agent — partial server disable
- **WHEN** an agent session starts in a workspace where server "github" is disabled but "context7" is enabled
- **THEN** `session_config_overrides` contains `mcp_servers.github.enabled = false` and does NOT contain an override for `context7`

#### Scenario: Claude agent — specific tool denied
- **WHEN** Claude requests a tool call for `mcp__github__search_issues` and "github" is in the effective disabled list
- **THEN** the tool request is denied with an appropriate message

#### Scenario: Claude agent — allowed tool not blocked
- **WHEN** Claude requests a tool call for `mcp__context7__resolve-library-id` and "context7" is NOT in the disabled list
- **THEN** the tool request proceeds normally
