## ADDED Requirements

### Requirement: Status bar reflects active sub-agent during delegation
When a Claude session with orchestration mode enabled invokes the Agent tool for a known conduit sub-agent, the footer status bar SHALL replace the mode label and model name with values reflecting the active sub-agent, for the duration of that tool call.

#### Scenario: Delegation to conduit-explore updates status bar
- **WHEN** a Claude session has orchestration mode enabled
- **AND** a `tool_use` event arrives with `tool_name == "Agent"` and `subagent_type == "conduit-explore"`
- **THEN** the mode chip in the status bar SHALL display "Explore"
- **AND** the model chip SHALL display the short name for `claude-haiku-4-5`

#### Scenario: Delegation to conduit-review updates status bar
- **WHEN** a Claude session has orchestration mode enabled
- **AND** a `tool_use` event arrives with `tool_name == "Agent"` and `subagent_type == "conduit-review"`
- **THEN** the mode chip in the status bar SHALL display "Review"
- **AND** the model chip SHALL display the short name for `claude-haiku-4-5`

#### Scenario: Status bar reverts when delegation completes
- **WHEN** a `tool_result` event arrives whose `tool_id` matches the in-flight Agent delegation
- **THEN** the mode chip SHALL revert to the session's normal agent mode label (e.g. "Build" or "Plan")
- **AND** the model chip SHALL revert to the orchestrator session's model name

#### Scenario: Non-orchestration sessions are unaffected
- **WHEN** a Claude session has orchestration mode disabled
- **AND** an Agent tool_use event arrives
- **THEN** the mode chip and model chip SHALL NOT change

#### Scenario: Unknown sub-agent names are ignored
- **WHEN** a `tool_use` event arrives with `tool_name == "Agent"` and an unrecognized `subagent_type`
- **THEN** the status bar SHALL NOT change

#### Scenario: Status bar reverts on session stop or interrupt
- **WHEN** the session transitions to idle/ready state (stop, interrupt, or error)
- **AND** a delegation was in flight
- **THEN** the mode chip and model chip SHALL revert to their normal values
