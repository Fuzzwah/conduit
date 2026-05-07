## ADDED Requirements

### Requirement: Inline prompt yields all keys when a dialog overlay is active
While an inline prompt is active, ALL key events SHALL be passed through (returned as `PromptAction::NotHandled` at the dispatch layer) whenever any dialog overlay is open or a Work Complete session is in progress. The inline prompt SHALL only consume keys when the active workspace has no overlapping dialog and the sidebar does not have focus.

#### Scenario: Enter pressed during ExitPlanMode prompt while Work Complete is open
- **WHEN** an ExitPlanMode inline prompt is displayed in the active workspace
- **AND** a Work Complete dialog is open (`work_complete_session.is_some()`)
- **THEN** the Enter key is NOT consumed by the inline prompt and is received by the Work Complete dialog handler

#### Scenario: Enter pressed during ExitPlanMode prompt while a confirmation dialog is open
- **WHEN** an ExitPlanMode inline prompt is displayed in the active workspace
- **AND** a confirmation dialog overlay is visible (`has_active_overlay()` returns true)
- **THEN** the Enter key is NOT consumed by the inline prompt

#### Scenario: Enter pressed during ExitPlanMode prompt with no overlays
- **WHEN** an ExitPlanMode inline prompt is displayed in the active workspace
- **AND** no dialog overlays are active
- **AND** the sidebar does not have focus
- **THEN** Enter IS consumed by the inline prompt and triggers the selected option (approve / feedback)

#### Scenario: Arrow keys still bypass inline prompt when sidebar has focus
- **WHEN** an inline prompt is displayed
- **AND** the sidebar has focus (`InputMode::SidebarNavigation`)
- **THEN** arrow keys are NOT consumed by the inline prompt (existing behaviour preserved)
