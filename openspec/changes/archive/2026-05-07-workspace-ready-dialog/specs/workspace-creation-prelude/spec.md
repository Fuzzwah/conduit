## MODIFIED Requirements

### Requirement: Workspace creation runs a strict three-phase prelude
When the user initiates new-workspace creation for a repository (e.g. via Alt+N), the system SHALL run three phases in strict order before showing the name/branch dialog: (1) remote sync, (2) issue selection, (3) spec selection. Each phase SHALL fully resolve before the next begins. Issue and spec data fetches SHALL be initiated only by transitions out of the immediately-prior phase, never speculatively in parallel. After the Naming phase completes and the workspace is created successfully, the system SHALL always show the workspace-ready config panel before opening the workspace tab.

#### Scenario: Phases execute in order
- **WHEN** the user starts new-workspace creation
- **THEN** the system enters the SyncingRemote phase first, then FetchingIssues, then (if any issues) PickingIssue, then FetchingSpecs, then (if any specs) PickingSpec, then Naming
- **AND** no later-phase fetch is initiated until the prior phase resolves

#### Scenario: Spec fetch never precedes remote sync
- **WHEN** the user starts new-workspace creation
- **THEN** the spec-fetch effect is not emitted until the system has received the RemoteSynced event AND the issue phase has resolved (either no issues found or the user pressed Enter/Esc on the issue picker)

#### Scenario: User can cancel the prelude
- **WHEN** the user presses Esc during any picker phase
- **THEN** that picker is dismissed and the flow advances to the next phase (issue picker dismissal advances to spec phase; spec picker dismissal advances to naming)

#### Scenario: Config panel shown after successful creation
- **WHEN** the workspace creation task completes without error
- **THEN** the progress dialog transitions to the workspace-ready config panel before the workspace tab is opened

#### Scenario: No config panel on workspace creation error
- **WHEN** the workspace creation task fails
- **THEN** the dialog remains in its error state; the config panel is not shown

## ADDED Requirements

### Requirement: Keyboard input during config panel is scoped to InputMode::CreatingWorkspace
During `InputMode::CreatingWorkspace`, when the config panel is active, the system SHALL route keyboard input to the config panel row navigation and row activation logic. Input SHALL NOT be routed to the sidebar, tab bar, prompt input, or any other UI component while the config panel is displayed.

#### Scenario: Sidebar navigation blocked during config panel
- **WHEN** the config panel is displayed and the user presses an arrow key
- **THEN** the config panel row focus changes; the sidebar selection is unchanged

#### Scenario: Esc closes config panel (same as Enter on Continue)
- **WHEN** the config panel is displayed and the user presses Esc
- **THEN** the config panel dismisses with the current selections applied (same behaviour as pressing Enter on the Continue button)
