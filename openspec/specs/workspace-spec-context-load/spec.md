## ADDED Requirements

### Requirement: Context-load message sent on spec-linked workspace creation
When a workspace is created with an OpenSpec or Specify spec selected, the system SHALL automatically send an initial user message to the agent upon first opening the workspace. The message SHALL ask the agent to read the relevant spec files and summarize remaining work. The message SHALL be sent only once — at creation time — and SHALL NOT be re-sent on subsequent reopens.

#### Scenario: OpenSpec workspace opens with context message
- **WHEN** a workspace is created with an OpenSpec change selected
- **THEN** the agent receives a message asking it to read `openspec/changes/{change_id}/` (proposal.md, design.md, tasks.md) and summarize remaining work

#### Scenario: Specify workspace opens with context message
- **WHEN** a workspace is created with a Specify spec selected
- **THEN** the agent receives a message asking it to read `.specify/specs/{spec_id}/tasks.md` and summarize remaining work

#### Scenario: No-spec workspace opens without context message
- **WHEN** a workspace is created without any spec selected
- **THEN** no initial message is automatically sent and the agent starts with a blank session

#### Scenario: Reopening a spec-linked workspace does not re-send
- **WHEN** a workspace that was previously created with a spec is closed and reopened
- **THEN** the context-load message is NOT sent again; the existing session history is restored normally
