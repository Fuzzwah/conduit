## ADDED Requirements

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

### Requirement: Remote sync fetches and opportunistically fast-forwards
The system SHALL run `git fetch origin --quiet` on the repository's base checkout during the SyncingRemote phase. After a successful fetch, the system SHALL additionally attempt a fast-forward of the local default branch when ALL of the following hold: (a) HEAD is on the default branch, (b) the working tree is clean, (c) the local default branch is strictly behind `origin/<default>` and is an ancestor of it. The fast-forward SHALL be performed via `git merge --ff-only origin/<default>`. Any failure of either step SHALL be logged and SHALL NOT abort workspace creation.

#### Scenario: Clean default-branch checkout is fast-forwarded
- **WHEN** the user's base checkout is on the default branch with a clean working tree, and the local default branch is behind origin
- **THEN** after sync the local default branch matches `origin/<default>`

#### Scenario: Feature-branch checkout is left alone
- **WHEN** the user's base checkout is on a non-default branch
- **THEN** the fetch runs but no merge is attempted; the working tree is unchanged

#### Scenario: Dirty working tree is left alone
- **WHEN** the user's base checkout is on the default branch but has staged or unstaged changes
- **THEN** the fetch runs but no merge is attempted; the working tree is unchanged

#### Scenario: Sync failure does not abort the flow
- **WHEN** `git fetch` fails (e.g. no network, no remote configured)
- **THEN** the failure is logged as a warning AND the flow proceeds to the FetchingIssues phase

### Requirement: Spec scanning reads from the freshly-fetched remote ref
During the FetchingSpecs phase, the system SHALL scan for incomplete OpenSpec changes and incomplete spec-kit (`.specify`) specs by reading directly from `origin/<default_branch>` rather than from the working tree. Reading SHALL use `git ls-tree` to enumerate change directories (excluding `archive`) and `git show` to read each `tasks.md`. If `origin/<default_branch>` cannot be resolved (e.g. no remote, no `origin/HEAD`), the system SHALL fall back to scanning the working tree.

#### Scenario: Archived openspec change does not appear after archival on remote
- **GIVEN** an OpenSpec change exists in the user's local working tree at `openspec/changes/foo/`
- **AND** that change has been archived on the remote and is no longer present in `origin/<default_branch>`
- **WHEN** the user starts new-workspace creation
- **THEN** the spec picker does not list the change

#### Scenario: Locally-uncommitted change does not appear
- **GIVEN** the user has created `openspec/changes/bar/tasks.md` locally but not committed and pushed it
- **WHEN** the user starts new-workspace creation
- **THEN** the spec picker does not list `bar` (because it is not in `origin/<default>`)

#### Scenario: Working-tree fallback when ref cannot be resolved
- **GIVEN** a repository with no `origin` remote
- **WHEN** the user starts new-workspace creation
- **THEN** the spec picker scans the working tree and lists incomplete specs found there

### Requirement: Issue picker reflects sync and fetch phases distinctly
The issue picker dialog SHALL be visible from the start of the SyncingRemote phase and SHALL display distinct messaging for the syncing phase ("Syncing with remote…") and the issue-fetching phase ("Fetching open issues…"), each accompanied by a spinner. Picker input SHALL be disabled during these two phases.

#### Scenario: Sync message shows during fetch
- **WHEN** the SyncingRemote phase is in progress
- **THEN** the issue picker is visible and shows "Syncing with remote…" with a spinner

#### Scenario: Fetch message shows after sync resolves
- **WHEN** the system transitions into the FetchingIssues phase
- **THEN** the issue picker shows "Fetching open issues…" with a spinner

#### Scenario: Input is ignored during pre-list phases
- **WHEN** the user presses Enter or arrow keys while the picker is in syncing or fetching phase
- **THEN** the input has no effect

### Requirement: Empty phases auto-advance
When a fetch phase completes with an empty result, the system SHALL skip the corresponding picker and advance directly to the next phase.

#### Scenario: No issues skips the issue picker
- **WHEN** the FetchingIssues phase completes with zero issues
- **THEN** the issue picker is dismissed and the system advances to FetchingSpecs

#### Scenario: No specs skips the spec picker
- **WHEN** the FetchingSpecs phase completes with zero OpenSpec and zero spec-kit specs
- **THEN** the spec picker is not shown and the system advances directly to the Naming phase

### Requirement: Keyboard input during config panel is scoped to InputMode::CreatingWorkspace
During `InputMode::CreatingWorkspace`, when the config panel is active, the system SHALL route keyboard input to the config panel row navigation and row activation logic. Input SHALL NOT be routed to the sidebar, tab bar, prompt input, or any other UI component while the config panel is displayed.

#### Scenario: Sidebar navigation blocked during config panel
- **WHEN** the config panel is displayed and the user presses an arrow key
- **THEN** the config panel row focus changes; the sidebar selection is unchanged

#### Scenario: Esc closes config panel (same as Enter on Continue)
- **WHEN** the config panel is displayed and the user presses Esc
- **THEN** the config panel dismisses with the current selections applied (same behaviour as pressing Enter on the Continue button)

#### Scenario: Config panel shown after successful creation
- **WHEN** workspace creation completes without error
- **THEN** the progress dialog transitions to the workspace-ready config panel before the workspace tab is opened

#### Scenario: No config panel on workspace creation error
- **WHEN** the workspace creation task fails
- **THEN** the dialog remains in its error state; the config panel is not shown
