## ADDED Requirements

### Requirement: Workspaces persist active OpenSpec change linkage

The system SHALL store the linked OpenSpec change identifier on each workspace in a nullable `active_change_id: TEXT` column on the `workspaces` table. When a workspace is created via the new-workspace prelude with an OpenSpec change picked, the system SHALL populate this column with the picked change's id. When the workspace is created without picking a change, the column SHALL remain NULL.

#### Scenario: Schema migration adds the column idempotently

- **WHEN** the application starts against a database that lacks `active_change_id`
- **THEN** the migration SHALL add the nullable column to the `workspaces` table without affecting existing rows
- **AND** running the migration a second time SHALL be a no-op

#### Scenario: Picking an OpenSpec change at creation persists the link

- **WHEN** the user picks an OpenSpec change in the new-workspace prelude and completes creation
- **THEN** the resulting workspace row SHALL have `active_change_id` equal to the picked change's id

#### Scenario: Skipping the spec picker leaves the column NULL

- **WHEN** the user dismisses the spec picker (no change selected)
- **THEN** the resulting workspace row SHALL have `active_change_id IS NULL`

### Requirement: Workspaces persist active GitHub issue linkage

The system SHALL store the linked GitHub issue number on each workspace in a nullable `active_issue_number: INTEGER` column on the `workspaces` table. When a workspace is created via the new-workspace prelude with an issue picked, the system SHALL populate this column with the picked issue's number. When the workspace is created without picking an issue, the column SHALL remain NULL.

#### Scenario: Schema migration adds the column idempotently

- **WHEN** the application starts against a database that lacks `active_issue_number`
- **THEN** the migration SHALL add the nullable column to the `workspaces` table without affecting existing rows

#### Scenario: Picking an issue at creation persists the link

- **WHEN** the user picks an issue in the new-workspace prelude and completes creation
- **THEN** the resulting workspace row SHALL have `active_issue_number` equal to the picked issue's number

#### Scenario: Skipping the issue picker leaves the column NULL

- **WHEN** the user dismisses the issue picker (no issue selected)
- **THEN** the resulting workspace row SHALL have `active_issue_number IS NULL`

### Requirement: Worktree-scan fallback infers links for legacy workspaces

When Work Complete preflight runs against a workspace whose `active_change_id` or `active_issue_number` is NULL, the system SHALL attempt to infer the value from the workspace's git state. Spec inference SHALL run `git log --diff-filter=A --name-only origin/<base>..HEAD -- openspec/changes/` and take the unique change directory created on the branch. Issue inference SHALL match a `#<digits>` token in the branch name. If inference returns a value, the system SHALL persist it back to the workspace row via an `update_active_links` operation.

#### Scenario: Spec is inferred from a single change directory created on the branch

- **WHEN** preflight runs on a legacy workspace and `git log --diff-filter=A --name-only origin/<base>..HEAD -- openspec/changes/` shows exactly one change directory
- **THEN** the system SHALL set `active_change_id` to that directory's basename and persist it to the workspace row

#### Scenario: Spec inference returns no value when no directory was added

- **WHEN** preflight runs on a legacy workspace and no `openspec/changes/<id>` directory was added on the branch
- **THEN** `active_change_id` remains NULL and no warning is raised

#### Scenario: Spec inference defers when multiple change directories were added

- **WHEN** preflight runs on a legacy workspace and more than one `openspec/changes/<id>` directory was created on the branch
- **THEN** the system SHALL pick the most-recently-modified directory and persist it, treating multi-spec inference as a known edge case to revisit

#### Scenario: Issue is inferred from `#N` in the branch name

- **WHEN** the workspace's branch name contains a `#<digits>` token (e.g., `feat/foo-#123`)
- **THEN** the system SHALL set `active_issue_number` to that integer and persist it

#### Scenario: Issue inference returns no value when no token is present

- **WHEN** the workspace's branch name contains no `#<digits>` token
- **THEN** `active_issue_number` remains NULL

### Requirement: Update operation rewrites the persisted link columns

The system SHALL provide a `WorkspaceStore::update_active_links(id, active_change_id, active_issue_number)` operation that rewrites the two columns on the workspace row identified by `id`. The operation SHALL be safe to call repeatedly with identical values.

#### Scenario: Update writes both columns

- **WHEN** the inference fallback resolves both spec and issue
- **THEN** a single `update_active_links` call writes both values atomically

#### Scenario: Update preserves NULL when value is not provided

- **WHEN** `update_active_links` is called with one value `Some(...)` and the other `None`
- **THEN** the provided column is updated and the other column is preserved as-is

### Requirement: SELECT queries surface the new columns

All `WorkspaceStore` reads (`get_by_id`, `get_by_repository`, `get_all`, `get_default_for_repository`, `get_by_path`) SHALL include `active_change_id` and `active_issue_number` in their SELECT lists, and `row_to_workspace` SHALL deserialise them into the `Workspace` struct fields.

#### Scenario: Loaded workspaces carry the link fields

- **WHEN** any `WorkspaceStore` getter loads a workspace whose row has populated link columns
- **THEN** the resulting `Workspace` struct's `active_change_id` and `active_issue_number` fields are populated accordingly
