## ADDED Requirements

### Requirement: TUI persists agent PID to database on session start
When the TUI starts an agent process for a session tab, the system SHALL write the process PID and PID start time to the `agent_pid` and `agent_pid_start_time` columns of the corresponding `session_tabs` row. Errors during this write SHALL be logged and ignored so they never interrupt agent startup.

#### Scenario: Agent starts successfully
- **WHEN** the TUI spawns an agent process for session tab `S`
- **THEN** `session_tabs.agent_pid` for `S` SHALL be set to the process PID
- **AND** `session_tabs.agent_pid_start_time` for `S` SHALL be set to the process start time

#### Scenario: DB write fails on agent start
- **WHEN** the TUI spawns an agent process and the DB write returns an error
- **THEN** the agent SHALL continue running normally
- **AND** the error SHALL be logged at WARN level

### Requirement: TUI clears agent PID from database on session end
When a TUI agent process exits (normally, via interrupt, or via explicit stop), the system SHALL clear `agent_pid` and `agent_pid_start_time` to NULL for the corresponding session tab. Errors during this write SHALL be logged and ignored.

#### Scenario: Agent exits normally
- **WHEN** the TUI agent process for session `S` exits
- **THEN** `session_tabs.agent_pid` for `S` SHALL be NULL
- **AND** `session_tabs.agent_pid_start_time` for `S` SHALL be NULL

#### Scenario: Agent is interrupted
- **WHEN** the user interrupts the agent for session `S`
- **THEN** `session_tabs.agent_pid` for `S` SHALL be NULL after the interrupt completes

#### Scenario: Agent is stopped via stop_agent_for_tab
- **WHEN** `stop_agent_for_tab` is called for a tab
- **THEN** `session_tabs.agent_pid` for that tab SHALL be NULL after the stop completes

### Requirement: DB schema includes agent PID columns
The `session_tabs` table SHALL have nullable `agent_pid INTEGER` and `agent_pid_start_time INTEGER` columns. Existing rows without an active agent SHALL have NULL in both columns.

#### Scenario: Fresh database
- **WHEN** conduit initialises a new database
- **THEN** `session_tabs` SHALL include `agent_pid INTEGER` and `agent_pid_start_time INTEGER` columns

#### Scenario: Existing database migration
- **WHEN** conduit opens an existing database that lacks these columns
- **THEN** both columns SHALL be added via `ALTER TABLE` migration
- **AND** all existing rows SHALL have NULL values for both columns
