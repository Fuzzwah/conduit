## ADDED Requirements

### Requirement: Creation persists picked issue and spec onto the workspace row

When the new-workspace prelude has an issue or spec selected at the time the workspace is created, the system SHALL pass those selections through to workspace creation so they are written to the new row's `active_change_id` and `active_issue_number` columns. Both selections SHALL be optional and independent: picking an issue alone, a spec alone, both, or neither are all valid outcomes. Existing prelude phasing semantics (strict ordering of remote sync, issue selection, spec selection, naming) are unchanged.

#### Scenario: Both issue and spec are picked

- **WHEN** the user picks an issue in the issue picker and a change in the spec picker, then completes naming
- **THEN** the resulting workspace row SHALL have `active_issue_number` set to the picked issue's number and `active_change_id` set to the picked change's id

#### Scenario: Only an issue is picked

- **WHEN** the user picks an issue and dismisses the spec picker
- **THEN** the resulting workspace row SHALL have `active_issue_number` set and `active_change_id IS NULL`

#### Scenario: Only a spec is picked

- **WHEN** the user dismisses the issue picker and picks a spec
- **THEN** the resulting workspace row SHALL have `active_change_id` set and `active_issue_number IS NULL`

#### Scenario: Neither is picked

- **WHEN** the user dismisses both pickers
- **THEN** the resulting workspace row SHALL have both link columns NULL, and the workspace creation flow SHALL proceed exactly as it does today

#### Scenario: Phase ordering is unchanged

- **WHEN** the user enters the new-workspace prelude
- **THEN** the strict order `SyncingRemote → FetchingIssues → PickingIssue → FetchingSpecs → PickingSpec → Naming` is preserved, with no parallelism or speculative fetches introduced by the linkage feature
