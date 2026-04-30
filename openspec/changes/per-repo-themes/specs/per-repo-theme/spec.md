## ADDED Requirements

### Requirement: Repository theme assignment
A repository SHALL support an optional theme name that overrides the global theme for all workspace tabs belonging to that repository. The theme name SHALL reference any theme available in the theme registry (built-in, user TOML, or VS Code).

#### Scenario: Assign theme to repository via theme picker
- **WHEN** a workspace tab is active and the user opens the theme picker in "This project" scope
- **THEN** confirming a theme SHALL save it to the repository record in the database

#### Scenario: Remove repository theme
- **WHEN** the user selects "Clear project theme" in the theme picker while a project theme is active
- **THEN** the repository's theme name SHALL be set to NULL and the global theme SHALL be applied

#### Scenario: Project scope unavailable without workspace context
- **WHEN** the active tab has no associated repository (e.g., a new-workspace tab)
- **THEN** the "This project" scope toggle SHALL be disabled in the theme picker

### Requirement: Automatic theme application on tab switch
The TUI SHALL automatically apply the active repository's theme when switching to a workspace tab whose repository has a project theme configured.

#### Scenario: Switch to tab with project theme
- **WHEN** the user switches to a workspace tab associated with a repository that has a theme configured
- **THEN** the TUI SHALL apply that repository's theme immediately upon tab activation

#### Scenario: Switch to tab without project theme
- **WHEN** the user switches to a workspace tab whose repository has no theme configured, or to a tab with no repository context
- **THEN** the TUI SHALL apply the global theme from `~/.conduit/config.toml`

#### Scenario: Project theme applied on startup
- **WHEN** the application starts and restores the previously active tab
- **THEN** if that tab's repository has a project theme, it SHALL be applied during startup

### Requirement: Theme scope toggle in theme picker
The theme picker SHALL provide a toggle allowing the user to choose between saving a theme globally or scoping it to the current project.

#### Scenario: Toggle between Global and This project scope
- **WHEN** the user presses the designated key (e.g., Tab) inside the theme picker
- **THEN** the scope SHALL toggle between "Global" and "This project"

#### Scenario: Scope persists during picker session
- **WHEN** the user changes the scope within the theme picker
- **THEN** the selected scope SHALL remain active until the picker is closed or the scope is changed again

### Requirement: Project theme indicator
The TUI SHALL clearly indicate when a project-level theme override is active, distinguishing it from the global theme.

#### Scenario: Indicator shown in theme picker with active project theme
- **WHEN** the active tab has a project theme applied and the user opens the theme picker
- **THEN** the picker SHALL display a "(project)" label or badge indicating the theme is a project override

#### Scenario: No indicator when using global theme
- **WHEN** the active tab uses the global theme (no project override)
- **THEN** no project theme indicator SHALL be shown
