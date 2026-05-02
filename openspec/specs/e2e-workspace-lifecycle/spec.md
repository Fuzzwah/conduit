## ADDED Requirements

### Requirement: Archive workspace via key binding
The system SHALL allow a user to archive an active workspace and the workspace tab SHALL disappear from the tab bar after archival is confirmed.

#### Scenario: Archive confirmation dialog appears
- **WHEN** a workspace tab is active and the user triggers the archive action
- **THEN** a confirmation dialog containing the workspace name and a prompt to confirm SHALL appear on screen

#### Scenario: Archive cancelled leaves workspace intact
- **WHEN** the archive confirmation dialog is shown and the user presses Escape or selects the cancel option
- **THEN** the dialog SHALL close and the workspace tab SHALL remain in the tab bar

#### Scenario: Archive confirmed removes workspace tab
- **WHEN** the archive confirmation dialog is shown and the user confirms
- **THEN** the dialog SHALL close and the workspace tab SHALL no longer appear in the tab bar

### Requirement: Workspace preflight shown before archive
The system SHALL display a preflight check summary before the archive confirmation so the user can see any warnings (e.g. uncommitted changes, open PRs).

#### Scenario: Preflight content visible
- **WHEN** the user triggers archive on a workspace with a known-clean git state
- **THEN** the preflight screen SHALL appear before the final confirmation step
