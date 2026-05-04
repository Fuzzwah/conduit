## ADDED Requirements

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
