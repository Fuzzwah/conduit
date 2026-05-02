## ADDED Requirements

### Requirement: Open file in file viewer tab via command
The system SHALL open a named file in a new file viewer tab when the user types the `:open <filename>` command in the input box and presses Enter.

#### Scenario: File viewer tab opens
- **WHEN** a workspace is active and the user types `:open README.md` and presses Enter
- **THEN** a new file viewer tab SHALL appear in the tab bar and the contents of README.md SHALL be visible

#### Scenario: File viewer tab shows file content
- **WHEN** a file viewer tab is open for README.md
- **THEN** the screen SHALL contain text from the README.md file

### Requirement: File viewer tab can be closed independently
The system SHALL allow the user to close a file viewer tab (e.g. with Ctrl+W or Escape) without closing the associated workspace tab.

#### Scenario: File tab closes on Ctrl+W
- **WHEN** a file viewer tab is the active tab and the user presses Ctrl+W
- **THEN** the file viewer tab SHALL close and the previous workspace tab SHALL become active

#### Scenario: Workspace tab remains after file tab closed
- **WHEN** a file viewer tab is closed
- **THEN** the workspace tab associated with the same session SHALL still be present in the tab bar

### Requirement: Switching between workspace tab and file tab
The system SHALL allow the user to switch between a workspace session tab and an open file viewer tab by pressing Tab.

#### Scenario: Tab key switches to file viewer
- **WHEN** a workspace tab is active and a file viewer tab is open, and the user presses Tab
- **THEN** the file viewer tab SHALL become active

#### Scenario: Tab key switches back to workspace
- **WHEN** the file viewer tab is active and the user presses Tab
- **THEN** the workspace session tab SHALL become active
