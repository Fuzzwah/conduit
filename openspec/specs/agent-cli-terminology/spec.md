## ADDED Requirements

### Requirement: User-facing labels use "Agent CLI" terminology
All user-visible references to the set of coding agents (Claude Code, Codex CLI, Gemini CLI, OpenCode, Pi, Dirac, GitHub Copilot) SHALL use "Agent CLI" terminology instead of "Provider". This applies to dialog titles, menu entries, row labels, picker titles, descriptions, and confirmation messages across the TUI.

#### Scenario: Workspace config panel uses "Agent CLI" label
- **WHEN** the workspace creation config panel is displayed
- **THEN** the first config row SHALL display "Agent CLI" (not "Provider") as its label

#### Scenario: Settings menu uses "Agent CLI" in title and description
- **WHEN** the settings menu is rendered
- **THEN** the entry for selecting agents SHALL display title "Agent CLIs" and description using "Agent CLI" terminology

#### Scenario: Multi-select dialog uses "Agent CLI" in title and description
- **WHEN** the agent selection dialog is opened (from settings or workspace config)
- **THEN** the dialog title SHALL use "Select Agent CLIs" (not "Select Providers") and the description SHALL use "Agent CLI" terminology

#### Scenario: Confirmation message uses "Agent CLI" terminology
- **WHEN** the agent selection is confirmed
- **THEN** the toast notification SHALL use "Agent CLI" terminology

#### Scenario: Web UI settings dialog uses "Agent CLIs" heading
- **WHEN** the web UI settings dialog opens the Providers sub-editor
- **THEN** the sub-editor heading SHALL display "Agent CLIs" instead of "Enabled Providers"

#### Scenario: Web UI backend returns "Agent CLI" titles
- **WHEN** the web UI fetches settings items
- **THEN** the providers setting item SHALL have title "Agent CLIs" and description using "Agent CLI" terminology

#### Scenario: Action description uses "agent CLI" terminology
- **WHEN** the ShowProvidersSelector action is displayed in the command palette or keybindings list
- **THEN** its description SHALL use "agent CLI" terminology
