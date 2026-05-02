## ADDED Requirements

### Requirement: Sidebar toggle shows and hides sidebar
The system SHALL toggle the sidebar visibility when the user presses Ctrl+T, and the sidebar SHALL be hidden when toggled off.

#### Scenario: Sidebar hidden after toggle off
- **WHEN** the sidebar is visible and the user presses Ctrl+T
- **THEN** the sidebar SHALL no longer be visible on screen

#### Scenario: Sidebar shown after toggle on
- **WHEN** the sidebar is hidden and the user presses Ctrl+T
- **THEN** the sidebar SHALL become visible, showing the project tree

### Requirement: Arrow key navigation within sidebar
The system SHALL allow keyboard navigation through the sidebar project/workspace tree using Up and Down arrow keys, and the selected item SHALL change with each key press.

#### Scenario: Down arrow moves selection
- **WHEN** the sidebar is visible and focused, and a project with workspaces is listed, and the user presses Down
- **THEN** the next item in the sidebar list SHALL become selected

#### Scenario: Up arrow moves selection back
- **WHEN** the sidebar has a non-first item selected and the user presses Up
- **THEN** the previous item SHALL become selected

### Requirement: Selecting workspace from sidebar activates its tab
The system SHALL activate the corresponding workspace tab when the user selects a workspace from the sidebar and presses Enter.

#### Scenario: Enter on sidebar workspace switches active tab
- **WHEN** a workspace is selected in the sidebar and the user presses Enter
- **THEN** the workspace's tab SHALL become the active tab in the tab bar

### Requirement: Sidebar shows project and workspace names
The system SHALL display project names and their associated workspace names in the sidebar tree.

#### Scenario: Project name visible in sidebar
- **WHEN** a project has been added and the sidebar is visible
- **THEN** the project's name SHALL appear in the sidebar

#### Scenario: Workspace name visible under project
- **WHEN** a project has active workspaces and the sidebar is visible
- **THEN** each workspace's name SHALL appear in the sidebar under its project
