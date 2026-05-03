## ADDED Requirements

### Requirement: Archive preflight warns on incomplete OpenSpec tasks
When the archive preflight runs for a workspace whose name matches an OpenSpec change directory, the system SHALL check whether `openspec/changes/{workspace_name}/tasks.md` exists in the repository root and, if so, count lines matching the `- [ ]` incomplete-task marker. If any incomplete tasks are found, the system SHALL add a warning to the preflight result.

#### Scenario: Workspace has a linked OpenSpec change with incomplete tasks
- **WHEN** the archive preflight runs for workspace named `my-feature`
- **AND** `openspec/changes/my-feature/tasks.md` exists and contains one or more `- [ ]` lines
- **THEN** the preflight result SHALL include a warning such as `"OpenSpec change has N incomplete task(s)"`

#### Scenario: Workspace has a linked OpenSpec change with all tasks complete
- **WHEN** the archive preflight runs for workspace named `my-feature`
- **AND** `openspec/changes/my-feature/tasks.md` exists but contains no `- [ ]` lines
- **THEN** no spec-related warning is added to the preflight result

#### Scenario: Workspace has no linked OpenSpec change
- **WHEN** the archive preflight runs for workspace named `my-feature`
- **AND** `openspec/changes/my-feature/tasks.md` does not exist
- **THEN** no spec-related warning is added and the preflight proceeds normally

### Requirement: Archive preflight warns on incomplete Specify spec tasks
When the archive preflight runs for a workspace whose name matches a Specify spec directory, the system SHALL check whether `.specify/specs/{workspace_name}/tasks.md` exists in the repository root and, if so, count lines matching the `- [ ]` incomplete-task marker. If any incomplete tasks are found, the system SHALL add a warning to the preflight result.

#### Scenario: Workspace has a linked Specify spec with incomplete tasks
- **WHEN** the archive preflight runs for workspace named `my-spec`
- **AND** `.specify/specs/my-spec/tasks.md` exists and contains one or more `- [ ]` lines
- **THEN** the preflight result SHALL include a warning such as `"Specify spec has N incomplete task(s)"`

#### Scenario: Workspace has a linked Specify spec with all tasks complete
- **WHEN** the archive preflight runs for workspace named `my-spec`
- **AND** `.specify/specs/my-spec/tasks.md` exists but contains no `- [ ]` lines
- **THEN** no spec-related warning is added to the preflight result

#### Scenario: Both OpenSpec and Specify specs exist for a workspace
- **WHEN** the archive preflight runs and both `openspec/changes/{name}/tasks.md` and `.specify/specs/{name}/tasks.md` exist with incomplete tasks
- **THEN** both warnings SHALL be independently added to the preflight result

#### Scenario: Spec tasks.md cannot be read
- **WHEN** the archive preflight runs and `tasks.md` exists but cannot be read (permissions error, etc.)
- **THEN** the file read error SHALL be silently ignored and no warning added; the rest of the preflight continues normally

### Requirement: Archive terminates all agent processes for the workspace
When a workspace is archived, the system SHALL terminate all agent processes associated with that workspace's session tabs, regardless of whether those processes were launched by the TUI or by the web server. Termination SHALL use SIGTERM with a grace period followed by SIGKILL, consistent with existing session stop behaviour.

#### Scenario: TUI-launched processes are killed on archive
- **WHEN** a workspace is archived while one or more agent processes are running under TUI sessions
- **THEN** all such processes SHALL receive SIGTERM
- **AND** any process that does not exit within the grace period SHALL receive SIGKILL
- **AND** no agent processes for that workspace SHALL remain after archive completes

#### Scenario: Process already exited before archive
- **WHEN** a workspace is archived and a session's stored PID refers to a process that has already exited
- **THEN** the archive SHALL complete successfully without error
- **AND** the stale PID SHALL not cause a kill of an unrelated process (verified via PID start time)

#### Scenario: No running processes
- **WHEN** a workspace is archived and no agent processes are running for its sessions
- **THEN** the archive SHALL complete successfully without attempting any process termination
