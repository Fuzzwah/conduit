## ADDED Requirements

### Requirement: Numbered tab navigation
The system SHALL allow users to jump directly to a tab by its number using Ctrl+1 through Ctrl+9, and the jumped-to tab SHALL become the active tab.

#### Scenario: Ctrl+2 activates second tab
- **WHEN** two or more workspace tabs are open and the user presses Ctrl+2
- **THEN** the second tab SHALL become active and its content SHALL be visible

#### Scenario: Ctrl+1 returns to first tab
- **WHEN** the second tab is active and the user presses Ctrl+1
- **THEN** the first tab SHALL become active

### Requirement: Close tab with Ctrl+W
The system SHALL allow users to close the active tab with Ctrl+W, and the remaining tabs SHALL be renumbered or the app SHALL return to a no-tab state if the last tab is closed.

#### Scenario: Ctrl+W closes active tab
- **WHEN** two workspace tabs are open and the user presses Ctrl+W on the second tab
- **THEN** the second tab SHALL close and the first tab SHALL become active

#### Scenario: Closing last tab leaves empty state
- **WHEN** only one workspace tab is open and the user presses Ctrl+W
- **THEN** the tab bar SHALL show no workspace tabs and the main screen SHALL display an appropriate empty state or the new-session hint

### Requirement: Tab bar reflects active tab visually
The system SHALL visually distinguish the active tab from inactive tabs in the tab bar (e.g. with a prefix marker or highlight).

#### Scenario: Active tab indicated
- **WHEN** two tabs are open and the second is active
- **THEN** the tab bar SHALL show a visual marker (such as ▶ or bold text) on the second tab's label
