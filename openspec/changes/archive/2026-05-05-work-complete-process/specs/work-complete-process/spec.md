## ADDED Requirements

### Requirement: Work Complete replaces archive on the existing keybinding

The system SHALL bind `Alt+Shift+X` and the sidebar `x` button to `Action::CompleteWorkspaceWork`, which invokes the Work Complete flow rather than the legacy archive flow. The system SHALL also accept the legacy keybinding-config string `"ArchiveCurrentWorkspace"` as an alias for the new action so that user-customised key configs continue to function.

#### Scenario: Default keybinding triggers Work Complete

- **WHEN** the user presses `Alt+Shift+X` on an active workspace
- **THEN** the system enters the Work Complete flow (not the legacy archive dialog)

#### Scenario: Sidebar archive button triggers Work Complete

- **WHEN** the user clicks the workspace `x` button in the sidebar
- **THEN** the system enters the Work Complete flow

#### Scenario: Legacy keybinding string still resolves

- **WHEN** the system loads a user keybinding config that maps a key to the string `"ArchiveCurrentWorkspace"`
- **THEN** the keybinding parser SHALL resolve that string to `Action::CompleteWorkspaceWork`

### Requirement: Work Complete preflight introspects full workspace state

The system SHALL provide an HTTP endpoint `GET /workspaces/{id}/work-complete/preflight` that returns the workspace's git state, PR state, OpenSpec change state (when linked or inferable), GitHub issue state (when linked or inferable), a classified scenario, and an ordered list of suggested actions. Both the TUI and web UI SHALL drive their dialog from this single endpoint.

#### Scenario: Preflight returns git state

- **WHEN** the preflight runs for an active workspace
- **THEN** the response SHALL include `branch_name`, `base_branch`, `is_dirty`, an enumeration of dirty files, `commits_ahead`, `commits_behind`, `is_merged`, `has_upstream`, and `remote_branch_exists`

#### Scenario: Preflight returns PR state when a PR exists

- **WHEN** the workspace's branch has a PR discoverable via `gh pr view`
- **THEN** the response's `pr` field SHALL include the PR number, URL, state, draft flag, check rollup, mergeable status, review decision, and computed merge readiness

#### Scenario: Preflight returns spec state from persisted link

- **WHEN** the workspace has a non-NULL `active_change_id` and the corresponding `openspec/changes/<id>/tasks.md` is readable
- **THEN** the response's `spec` field SHALL include the change id, total task count, and completed task count, with `source: "linked"`

#### Scenario: Preflight infers spec from worktree when link is absent

- **WHEN** the workspace has a NULL `active_change_id` and `git log --diff-filter=A --name-only origin/<base>..HEAD -- openspec/changes/` reveals exactly one change directory created on this branch
- **THEN** the response's `spec` field SHALL be populated using that inferred change id with `source: "detected"`
- **AND** the system SHALL persist the inferred id to the workspace row via `update_active_links` so subsequent preflights can use the persisted value

#### Scenario: Preflight returns issue state from persisted link

- **WHEN** the workspace has a non-NULL `active_issue_number`
- **THEN** the response's `issue` field SHALL include the issue number, state (`open` or `closed`), title, URL, and `source: "linked"`

#### Scenario: Preflight infers issue from branch name when link is absent

- **WHEN** the workspace has a NULL `active_issue_number` and the branch name contains a `#<number>` token
- **THEN** the response's `issue` field SHALL resolve the issue via `gh issue view <number>` and report it with `source: "detected"`
- **AND** the system SHALL persist the inferred number to the workspace row

#### Scenario: Preflight classifies scenario when no edits exist

- **WHEN** preflight finds the worktree clean, no commits ahead, and no PR / spec / issue context that demands action
- **THEN** the response's `scenario` field SHALL be `CleanReady` and `suggested_actions` SHALL be `["archive"]`

#### Scenario: Preflight classifies scenario for unlinked edits

- **WHEN** preflight finds dirty files or commits ahead, and neither a spec nor an issue is linked or inferred
- **THEN** the response's `scenario` field SHALL be `EditsNoLink`

#### Scenario: Preflight classifies scenario when spec tasks are complete

- **WHEN** preflight resolves a spec link and every task line is checked
- **THEN** the response's `scenario` field SHALL be `SpecComplete`

#### Scenario: Preflight classifies scenario when spec tasks remain

- **WHEN** preflight resolves a spec link and at least one `- [ ]` line remains
- **THEN** the response's `scenario` field SHALL be `SpecIncomplete`

#### Scenario: Preflight classifies scenario when linked issue is open

- **WHEN** preflight resolves an issue link with state `open`
- **THEN** the response's `scenario` field SHALL be `IssueOpen`

#### Scenario: Preflight classifies scenario when linked issue is closed

- **WHEN** preflight resolves an issue link with state `closed`
- **THEN** the response's `scenario` field SHALL be `IssueClosed`

### Requirement: Suggested actions reflect the classified scenario

The system SHALL emit `suggested_actions` ordered by the natural completion sequence (commit → push → open PR → merge → close issue → archive spec → archive workspace), filtered to only those that make sense given the current state.

#### Scenario: CleanReady offers only archive

- **WHEN** the scenario is `CleanReady`
- **THEN** `suggested_actions` SHALL be exactly `["archive"]`

#### Scenario: EditsNoLink offers full git pipeline

- **WHEN** the scenario is `EditsNoLink` and the worktree is dirty, commits are ahead, no PR exists, and a base branch is configured
- **THEN** `suggested_actions` SHALL be `["commit", "push", "open_pr", "archive"]` (merge omitted because no PR yet)

#### Scenario: SpecComplete includes archive_spec

- **WHEN** the scenario is `SpecComplete`
- **THEN** `suggested_actions` SHALL include `archive_spec` in addition to whatever git actions remain

#### Scenario: IssueOpen includes close_issue

- **WHEN** the scenario is `IssueOpen` and no PR is open (so merge will not auto-close it)
- **THEN** `suggested_actions` SHALL include `close_issue`

### Requirement: Merge readiness gates the merge action

The system SHALL set the merge action's enabled state based on `MergeReadiness`. When `MergeReadiness` is `Ready` the action SHALL be enabled. When `MergeReadiness` is `Blocked`, `HasConflicts`, or `Unknown` the action SHALL be disabled, with the reason surfaced in the dialog. An "admin override" merge SHALL be available only behind a secondary explicit confirmation phase, never as a one-click action.

#### Scenario: Ready PR enables merge

- **WHEN** the PR's `merge_readiness` is `Ready`
- **THEN** the dialog's merge action is enabled

#### Scenario: Failing checks disable merge

- **WHEN** the PR's `merge_readiness` is `Blocked` due to failing required checks
- **THEN** the dialog's merge action is disabled and shows the reason text "Required checks not passing"

#### Scenario: Conflicting PR disables merge

- **WHEN** the PR's `merge_readiness` is `HasConflicts`
- **THEN** the dialog's merge action is disabled and shows the reason text "PR has merge conflicts"

#### Scenario: Admin override requires secondary confirmation

- **WHEN** the user requests "merge with --admin"
- **THEN** the dialog SHALL transition to a sub-confirm phase requiring an explicit second confirmation before invoking `gh pr merge --admin`

### Requirement: Dialog displays a spec summary line

The dialog SHALL render a single-line summary for the workspace's resolved OpenSpec change whenever a change is linked or successfully inferred, regardless of whether tasks are complete or incomplete. The summary SHALL show the change id, the change's source (`linked` from the persisted column or `detected` from the worktree-scan fallback), and the completed-vs-total count formatted as `X of Y tasks complete`. The dialog SHALL NOT render the full task list inline. When no change is linked or inferable, the dialog SHALL omit this section entirely.

#### Scenario: Spec summary renders for SpecIncomplete

- **WHEN** the scenario is `SpecIncomplete` with 7 of 12 tasks complete
- **THEN** the dialog's spec section SHALL show a single line such as `change-id (linked) — 7 of 12 tasks complete`

#### Scenario: Spec summary renders for SpecComplete

- **WHEN** the scenario is `SpecComplete` with all 12 tasks complete
- **THEN** the dialog's spec section SHALL show a single line such as `change-id (linked) — 12 of 12 tasks complete`

#### Scenario: Spec section omitted when no spec is linked or inferable

- **WHEN** the workspace has no linked or inferable spec
- **THEN** the dialog SHALL NOT render a spec section

### Requirement: Show-remaining-tasks action delegates to the agent

When the resolved spec has at least one incomplete task, the dialog SHALL expose a "Show remaining tasks" action alongside the standard suggested actions. Choosing this action SHALL close the Work Complete dialog and send the prompt `show incomplete tasks in <change_id>` to the active agent session for the workspace, so the user can review and act on the remaining items in the agent's context. The dialog SHALL NOT expose this action when the resolved spec has zero incomplete tasks.

#### Scenario: Action is available when tasks remain

- **WHEN** the resolved spec has at least one `- [ ]` line
- **THEN** the dialog's action list SHALL include "Show remaining tasks"

#### Scenario: Action closes the dialog and prompts the agent

- **WHEN** the user selects "Show remaining tasks"
- **THEN** the Work Complete dialog SHALL close without performing any other action
- **AND** the system SHALL deliver the prompt `show incomplete tasks in <change_id>` to the active agent session for the workspace

#### Scenario: Action is hidden when all tasks are complete

- **WHEN** the resolved spec has zero `- [ ]` lines
- **THEN** the dialog's action list SHALL NOT include "Show remaining tasks"

#### Scenario: Action is hidden when no spec is linked

- **WHEN** the workspace has no linked or inferable spec
- **THEN** the dialog's action list SHALL NOT include "Show remaining tasks"

### Requirement: Dialog always displays resolved issue status

The dialog SHALL render a status section for the workspace's resolved GitHub issue whenever an issue is linked or successfully inferred, regardless of whether the issue is open or closed. The section SHALL show the issue number, title, state (`open` or `closed`), URL, and source (`linked` or `detected`). When the issue is closed the section SHALL still be shown so the user can confirm it was the expected issue. When no issue is linked or inferable, the dialog SHALL omit this section entirely.

#### Scenario: Issue status renders when issue is open

- **WHEN** the scenario is `IssueOpen`
- **THEN** the dialog's issue section SHALL show the number, title, `open` state, URL, and source

#### Scenario: Issue status renders when issue is closed

- **WHEN** the scenario is `IssueClosed`
- **THEN** the dialog's issue section SHALL show the number, title, `closed` state, URL, and source so the user can confirm the linkage was correct

#### Scenario: Issue section omitted when no issue is linked or inferable

- **WHEN** the workspace has no linked or inferable issue
- **THEN** the dialog SHALL NOT render an issue section

### Requirement: Dialog always displays resolved git and PR status

The dialog SHALL render a status section showing branch name, base branch, dirty-file count and list, commits ahead and behind, merge state, and (when a PR exists) the PR number, state, draft flag, check rollup, mergeable status, review decision, and computed merge readiness. This section SHALL appear for every scenario, including `CleanReady`, so the user always sees the full git context before confirming any action.

#### Scenario: Git status renders for CleanReady

- **WHEN** the scenario is `CleanReady`
- **THEN** the dialog's git section SHALL still show the branch name, base branch, "0 dirty files", "0 commits ahead", and merge state, even though no git actions are offered

#### Scenario: PR status renders when a PR exists

- **WHEN** the workspace's branch has a discoverable PR
- **THEN** the dialog SHALL include a PR sub-section with number, URL, state, checks, mergeable status, review decision, and merge readiness

#### Scenario: PR sub-section omitted when no PR exists

- **WHEN** no PR is discoverable for the branch
- **THEN** the dialog SHALL NOT render a PR sub-section

### Requirement: Force-complete sub-flow for incomplete spec or open issue

When the scenario is `SpecIncomplete` or `IssueOpen`, the dialog SHALL show the relevant context (remaining task lines or open issue title and URL) and require an explicit "complete anyway" confirmation before allowing the user to proceed to the action list. Cancelling the confirmation SHALL exit the Work Complete flow without changes.

#### Scenario: Incomplete spec requires force-confirm

- **WHEN** the scenario is `SpecIncomplete`
- **THEN** the dialog SHALL display the spec summary line (per the spec-summary requirement) and SHALL require an explicit "Complete anyway" confirmation before exposing the standard action list (the "Show remaining tasks" action remains available before the force-confirm so the user can review tasks in the agent without committing to completion)

#### Scenario: Open issue requires force-confirm

- **WHEN** the scenario is `IssueOpen`
- **THEN** the dialog SHALL display the issue status section (per the always-display requirement) with the `open` state emphasised and SHALL require an explicit "Complete anyway" confirmation before exposing the action list

#### Scenario: Cancelling force-confirm exits the flow

- **WHEN** the user cancels the force-confirm phase
- **THEN** the dialog closes and no changes are made to the workspace

### Requirement: Inline commit action with pre-filled message

The system SHALL provide an HTTP endpoint `POST /workspaces/{id}/work-complete/commit` accepting `{ message: string }` that runs `git add -A && git commit -m <message>` in the workspace's worktree and returns the new commit SHA plus a log of executed commands. The dialog SHALL pre-fill the commit message from the branch name, the first one or two dirty file paths, and (when linked) `Implement <change_id>` or `Fix #<issue_number>`. The user MAY edit the pre-filled message before confirming.

#### Scenario: Commit endpoint creates a commit

- **WHEN** the dialog posts to `.../commit` with a non-empty message and the worktree has uncommitted changes
- **THEN** the system runs `git add -A && git commit -m <message>` in the workspace's path and the response includes the new commit SHA

#### Scenario: Pre-filled message includes spec link

- **WHEN** the dialog opens the commit input and the workspace has a non-NULL `active_change_id`
- **THEN** the input SHALL be pre-filled with text containing `Implement <change_id>`

#### Scenario: Pre-filled message includes issue link

- **WHEN** the dialog opens the commit input and the workspace has a non-NULL `active_issue_number`
- **THEN** the input SHALL be pre-filled with text containing `Fix #<issue_number>`

#### Scenario: Empty message is rejected

- **WHEN** the dialog posts to `.../commit` with an empty message
- **THEN** the endpoint returns a 400 error and no commit is made

### Requirement: Inline push action

The system SHALL provide an HTTP endpoint `POST /workspaces/{id}/work-complete/push` that runs `git push -u origin <branch>` in the workspace's worktree and returns a log of executed commands.

#### Scenario: Push uploads the branch

- **WHEN** the dialog posts to `.../push` and the branch has commits ahead of the remote
- **THEN** the system runs `git push -u origin <branch>` and the response indicates success

#### Scenario: Push without commits is a no-op

- **WHEN** the dialog posts to `.../push` and the branch is in sync with `origin/<branch>`
- **THEN** the endpoint returns success with a log line indicating "Already up to date"

### Requirement: Inline PR creation action

The system SHALL provide an HTTP endpoint `POST /workspaces/{id}/work-complete/pr` accepting an optional `{ title?: string, body?: string }` that invokes `gh pr create --base <base>` (with `--fill` when title and body are not provided, otherwise with `--title` and `--body`) and returns `{ url, number }`. This endpoint SHALL be distinct from the legacy `create_workspace_pr` agent-prompt path, which remains available for the agent flow.

#### Scenario: PR is created with --fill defaults

- **WHEN** the dialog posts to `.../pr` without title or body
- **THEN** the system invokes `gh pr create --base <base> --fill` and returns the new PR URL and number

#### Scenario: PR is created with explicit title and body

- **WHEN** the dialog posts to `.../pr` with both `title` and `body` populated
- **THEN** the system invokes `gh pr create --base <base> --title <t> --body <b>` and returns the new PR URL and number

### Requirement: Inline PR merge action

The system SHALL provide an HTTP endpoint `POST /workspaces/{id}/work-complete/pr/merge` accepting `{ method: "squash" | "merge" | "rebase", admin: bool }` that invokes `gh pr merge --<method>` (adding `--admin` when `admin: true`) and returns a log of executed commands. The endpoint SHALL refuse to run when `MergeReadiness` is not `Ready` unless `admin: true` is present.

#### Scenario: Squash merge of a Ready PR

- **WHEN** the dialog posts to `.../pr/merge` with `{ method: "squash", admin: false }` and the PR's readiness is `Ready`
- **THEN** the system invokes `gh pr merge --squash` and the response indicates success

#### Scenario: Merge refused when readiness is not Ready

- **WHEN** the dialog posts to `.../pr/merge` with `{ admin: false }` and the PR's readiness is `Blocked`, `HasConflicts`, or `Unknown`
- **THEN** the endpoint returns a 409 error with the readiness state in the body and no merge is attempted

#### Scenario: Admin override bypasses readiness gate

- **WHEN** the dialog posts to `.../pr/merge` with `{ admin: true }` regardless of readiness
- **THEN** the system invokes `gh pr merge --<method> --admin` and the response indicates success or failure

### Requirement: Inline issue close action

The system SHALL provide an HTTP endpoint `POST /workspaces/{id}/work-complete/issue/close` that invokes `gh issue close <active_issue_number>` and returns a log of executed commands. The endpoint SHALL return a 400 error when the workspace has no resolved issue link.

#### Scenario: Close an open linked issue

- **WHEN** the dialog posts to `.../issue/close` and the workspace's resolved issue is open
- **THEN** the system invokes `gh issue close <number>` and the response indicates success

#### Scenario: Close without an issue link

- **WHEN** the dialog posts to `.../issue/close` and the workspace has no resolved issue link
- **THEN** the endpoint returns a 400 error

### Requirement: Inline spec archive action

The system SHALL provide an HTTP endpoint `POST /workspaces/{id}/work-complete/spec/archive` accepting `{ change_id: string }` that renames `openspec/changes/<change_id>` to `openspec/changes/archive/YYYY-MM-DD-<change_id>` and returns `{ new_path, warnings }`. The endpoint SHALL NOT commit the rename; the dialog cycles back through preflight, which then offers the rename as a regular dirty change to commit. Spec deltas (delta `.md` files within the change directory) are NOT applied to canonical specs in v1; if any are present, the response SHALL include a warning recommending the user run `/opsx:archive` in their agent session.

#### Scenario: Archive moves the change directory

- **WHEN** the dialog posts to `.../spec/archive` with a `change_id` that exists at `openspec/changes/<change_id>`
- **THEN** the system renames the directory to `openspec/changes/archive/<today>-<change_id>` and returns the new path

#### Scenario: Archive refuses when the change is missing

- **WHEN** the dialog posts to `.../spec/archive` with a `change_id` that does not exist
- **THEN** the endpoint returns a 404 error and no filesystem change occurs

#### Scenario: Archive refuses to overwrite an existing target

- **WHEN** a directory already exists at `openspec/changes/archive/<today>-<change_id>`
- **THEN** the endpoint returns a 409 error and no filesystem change occurs

#### Scenario: Archive warns when delta specs are present

- **WHEN** the change directory contains delta spec files (e.g. `specs/<capability>/spec.md`)
- **THEN** the response's `warnings` SHALL include a message recommending the user run `/opsx:archive` in the agent session for spec-sync

### Requirement: Workspace archive endpoint terminates the flow

The system SHALL provide an HTTP endpoint `POST /workspaces/{id}/work-complete/archive` that performs the same workspace archive operation as the legacy `POST /workspaces/{id}` archive variant: stop active sessions, capture the branch SHA, remove the worktree, optionally delete local and remote branches per request and settings, set `archived_at` and `archived_commit_sha` on the workspace row, close session tabs, and remove the workspace from the status manager.

#### Scenario: Archive is the terminal action

- **WHEN** the dialog posts to `.../archive` after the user confirms
- **THEN** the workspace's `archived_at` is set, the worktree is removed, the workspace is filtered from active workspace lists, and the dialog closes

#### Scenario: Archive captures the branch SHA before cleanup

- **WHEN** archive runs
- **THEN** `archived_commit_sha` is populated with the branch's HEAD SHA captured before worktree removal

### Requirement: Backend endpoints are composable and idempotent

Each Work Complete action endpoint SHALL be independently invocable, return a uniform response shape `{ status, log_lines: string[] }` (extending it with action-specific fields where useful), and SHALL NOT depend on other endpoints having been called first. Sequencing across actions is the responsibility of the dialog, not the backend.

#### Scenario: Endpoints can be called in any meaningful order

- **WHEN** any individual action endpoint is invoked
- **THEN** it executes its own side effect against the current workspace state and returns a result, regardless of which other endpoints have or have not been called

#### Scenario: Repeated calls do not corrupt state

- **WHEN** the same action endpoint is called twice in a row (e.g. push when already up-to-date, archive-spec when already archived)
- **THEN** the second call returns a clean error or no-op without leaving the workspace in a partial state

### Requirement: TUI dialog uses a state machine modelled on workspace creation

The TUI SHALL implement Work Complete as a top-level overlay session with a pure transition function, mirroring the architecture of `WorkspaceCreationSession`. Phases SHALL include `Idle`, `LoadingPreflight`, `ReviewingState{scenario}`, `AwaitingCommitMessage`, `Executing{action}`, `ConfirmingForceComplete`, and `Done`. The transition function SHALL be unit-testable.

#### Scenario: Dialog transitions through phases

- **WHEN** the user triggers Work Complete on a workspace with edits and a linked open issue
- **THEN** the dialog phases progress `Idle → LoadingPreflight → ReviewingState{IssueOpen} → ConfirmingForceComplete → ReviewingState{IssueOpen} → Executing{commit} → … → Done`

#### Scenario: Cancelling at any phase exits cleanly

- **WHEN** the user presses Esc during any phase except `Executing`
- **THEN** the dialog closes and no further actions are run

### Requirement: Web dialog mirrors TUI behaviour over the same endpoints

The web UI SHALL provide a `WorkCompleteDialog` component that uses the same `GET /work-complete/preflight` and per-action endpoints, presents the same scenarios, requires the same force-confirm sub-flow for `SpecIncomplete` and `IssueOpen`, and gates the merge action on `MergeReadiness` identically to the TUI.

#### Scenario: Web dialog drives off the same endpoint

- **WHEN** the user clicks the workspace `x` button in the web sidebar
- **THEN** the web component fetches `GET /workspaces/{id}/work-complete/preflight` and renders the same scenario classification as the TUI would on identical state

#### Scenario: Web dialog enforces force-confirm

- **WHEN** the scenario is `SpecIncomplete` in the web dialog
- **THEN** the web component lists the remaining task lines and requires an explicit "Complete anyway" click before exposing action buttons

### Requirement: Action log streams into the dialog

Each action endpoint's `log_lines` field SHALL be appended to a scrolling log panel within the dialog (TUI and web alike) so the user can see what was actually executed. The log SHALL persist across actions within one Work Complete session.

#### Scenario: Log accumulates across actions

- **WHEN** the user runs commit, then push, then open PR within one Work Complete session
- **THEN** the dialog's log panel shows all three sets of log lines in order
