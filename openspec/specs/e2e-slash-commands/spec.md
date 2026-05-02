## ADDED Requirements

### Requirement: Slash menu opens on / key
The system SHALL display a slash-command menu when the user types `/` in the input box, listing available commands.

#### Scenario: Slash menu appears
- **WHEN** the user is on a workspace session tab and types `/`
- **THEN** a menu or dropdown listing available slash commands SHALL appear on screen

#### Scenario: Slash menu dismisses on Escape
- **WHEN** the slash menu is visible and the user presses Escape
- **THEN** the slash menu SHALL close and the input box SHALL be cleared or restored

### Requirement: Command palette opens with Ctrl+P
The system SHALL display a searchable command palette when the user presses Ctrl+P, and it SHALL close when Escape is pressed.

#### Scenario: Command palette appears
- **WHEN** the user presses Ctrl+P
- **THEN** a command palette or search dialog SHALL appear containing a list of available commands

#### Scenario: Command palette dismisses on Escape
- **WHEN** the command palette is visible and the user presses Escape
- **THEN** the command palette SHALL close and the main UI SHALL be restored

### Requirement: Slash command filtering by text
The system SHALL filter the visible slash-command list as the user types after `/`, showing only commands matching the typed prefix.

#### Scenario: Filter narrows results
- **WHEN** the slash menu is open and the user types additional characters (e.g. `/op`)
- **THEN** only commands whose names match the typed prefix SHALL remain visible in the menu
