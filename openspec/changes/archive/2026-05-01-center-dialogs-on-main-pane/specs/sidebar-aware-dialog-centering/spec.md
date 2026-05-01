## ADDED Requirements

### Requirement: Dialogs center within the main pane
When the sidebar is visible, all dialog windows SHALL be horizontally and vertically centered within the main pane (the terminal area excluding the sidebar column), not within the full terminal area.

#### Scenario: Dialog opens with sidebar visible
- **WHEN** the sidebar is visible and any dialog is opened (help, model selector, confirmation, error, etc.)
- **THEN** the dialog SHALL appear centered in the area to the right of the sidebar

#### Scenario: Dialog opens with sidebar hidden
- **WHEN** the sidebar is hidden and any dialog is opened
- **THEN** the dialog SHALL appear centered in the full terminal area (behavior unchanged)

#### Scenario: Sidebar toggled while dialog is open
- **WHEN** a dialog is open and the sidebar visibility changes
- **THEN** the dialog SHALL re-center to reflect the new main pane boundaries on the next render frame
