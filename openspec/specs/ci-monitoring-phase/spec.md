### Requirement: CI monitoring phase entered after PR creation
After the `OpenPr` action completes successfully in the Work Complete dialog, the dialog SHALL automatically enter a CI monitoring phase without requiring any user input.

#### Scenario: PR created successfully
- **WHEN** the Work Complete dialog executes the `OpenPr` action and it succeeds
- **THEN** the dialog immediately transitions to a `MonitoringCi` phase showing a spinner, the PR URL, and accumulated log output

### Requirement: CI monitoring phase entered after push with existing PR
After the `Push` action completes successfully and the current workspace already has an open pull request, the dialog SHALL automatically enter the CI monitoring phase.

#### Scenario: Push succeeds with open PR
- **WHEN** the Work Complete dialog executes the `Push` action and the preflight data contains an open PR
- **THEN** the dialog immediately transitions to a `MonitoringCi` phase showing a spinner, the existing PR URL, and accumulated log output

#### Scenario: Push succeeds with no open PR
- **WHEN** the Work Complete dialog executes the `Push` action and there is no open PR in the preflight data
- **THEN** the dialog refreshes preflight as normal (no CI monitoring phase)

### Requirement: CI monitoring runs gh pr checks to completion
During the `MonitoringCi` phase, the system SHALL run `gh pr checks --watch <pr_url>` and wait for all checks to reach a terminal state.

#### Scenario: Checks all pass
- **WHEN** `gh pr checks --watch` exits with code 0
- **THEN** the monitoring phase ends, output lines are appended to the dialog log, and the dialog refreshes preflight

#### Scenario: One or more checks fail
- **WHEN** `gh pr checks --watch` exits with a non-zero code
- **THEN** the monitoring phase ends, output lines are appended to the dialog log, and the dialog refreshes preflight (allowing the user to see the updated state)

#### Scenario: gh command unavailable or errors immediately
- **WHEN** `gh pr checks --watch` cannot run or exits immediately with an error
- **THEN** the error is appended to the log, the monitoring phase ends, and the dialog refreshes preflight

### Requirement: Dialog blocks interaction during CI monitoring
During the `MonitoringCi` phase, the dialog SHALL swallow all keyboard input, preventing the user from selecting actions or closing the dialog.

#### Scenario: User presses a key during monitoring
- **WHEN** the `MonitoringCi` phase is active and the user presses any key
- **THEN** the keypress is consumed and has no effect on the dialog state

### Requirement: MergePr action surfaces after CI monitoring completes
After CI monitoring ends and the dialog refreshes preflight, if the checks passed and the PR is merge-ready, the suggested action list SHALL present `MergePr` prominently.

#### Scenario: Checks passed, PR is merge-ready
- **WHEN** CI monitoring completes with passing checks
- **THEN** the refreshed preflight classifies the scenario such that `MergePr` is the top suggested action

### Requirement: CI monitoring phase renders spinner and PR URL
The `MonitoringCi` phase SHALL display a spinner animation, a label indicating that CI checks are being monitored, and the PR URL being watched.

#### Scenario: Dialog renders during monitoring
- **WHEN** the `MonitoringCi` phase is active
- **THEN** the dialog shows a spinner, the text "Monitoring CI checks…", the PR URL, and any accumulated log lines from prior actions
