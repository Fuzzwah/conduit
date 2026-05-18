## ADDED Requirements

### Requirement: Pi agent can delegate sub-tasks via Agent tool
The system SHALL provide a custom Pi Agent tool named "Agent" that accepts `subagent_type` (string, required) and `task` (string, optional) arguments, enabling the main Pi agent to delegate sub-tasks to sub-sessions running configured models.

#### Scenario: Main agent calls conduit-explore sub-agent
- **WHEN** the main Pi agent invokes the `Agent` tool with `subagent_type: "conduit-explore"`
- **THEN** the extension spawns a sub-session, sends the exploration task, collects the output, and returns it as the tool result

#### Scenario: Main agent calls conduit-review sub-agent
- **WHEN** the main Pi agent invokes the `Agent` tool with `subagent_type: "conduit-review"`
- **THEN** the extension spawns a sub-session, sends the review task, collects the output, and returns it

#### Scenario: Agent tool fails gracefully on sub-agent error
- **WHEN** the sub-agent session fails (model unavailable, timeout, SDK error)
- **THEN** the tool returns an error message describing the failure without crashing the main Pi process

### Requirement: Orchestration is toggleable in workspace config for Pi
The workspace progress dialog SHALL allow the user to enable/disable orchestration for Pi Agent sessions. The orchestration and adversarial review controls SHALL NOT be greyed out when Pi is the selected provider.

#### Scenario: Pi session shows orchestration controls
- **WHEN** the user opens the workspace config dialog with Pi as the selected provider
- **THEN** the Orchestration, Adversarial Review, and Review Model rows are interactive (not dimmed/greyed out)

#### Scenario: Toggle orchestration on for Pi
- **WHEN** the user toggles orchestration to On for a Pi session
- **THEN** Conduit passes `orchestration_enabled: true` to the Pi runner

### Requirement: Pi discovers orchestration skills
The system SHALL write Pi-native skill definition files (SKILL.md format) to `~/.pi/agent/skills/conduit-explore/SKILL.md`, `~/.pi/agent/skills/conduit-review/SKILL.md`, and `~/.pi/agent/skills/conduit-adversarial-review/SKILL.md` when orchestration is enabled for a Pi session. These SHALL follow the Agent Skills standard.

#### Scenario: Skills written on Pi session start with orchestration
- **WHEN** the Pi runner starts with `orchestration_enabled: true`
- **THEN** skill files for conduit-explore, conduit-review, and (if enabled) conduit-adversarial-review are written to `~/.pi/agent/skills/conduit-*/SKILL.md`

#### Scenario: Skills passed via --skill CLI arg
- **WHEN** the Pi runner starts with orchestration enabled
- **THEN** `--skill` CLI arguments are added for each orchestration skill directory

### Requirement: Orchestration instructions injected into Pi system prompt
When orchestration is enabled for Pi, the system SHALL inject orchestration instructions via `--append-system-prompt`, telling the agent to use the `Agent` tool for sub-agent delegation instead of loading raw file content.

#### Scenario: Instructions appended
- **WHEN** the Pi runner starts with `orchestration_enabled: true`
- **THEN** orchestration instructions are appended to the system prompt

### Requirement: Pi extension loaded via --extension
The system SHALL write a TypeScript extension to `~/.conduit/pi-agent-extensions/agent-tool.ts` and pass it to Pi via `--extension` when orchestration is enabled. The extension SHALL register an `Agent` tool using `pi.registerTool()`.

#### Scenario: Extension arg passed to pi
- **WHEN** the Pi runner starts with `orchestration_enabled: true`
- **THEN** `--extension ~/.conduit/pi-agent-extensions/agent-tool.ts` is added to the Pi CLI args

#### Scenario: Extension registers Agent tool
- **WHEN** Pi loads the extension at startup
- **THEN** the `Agent` tool is registered and available for the LLM to call

### Requirement: Pi orchestration badge shown in status bar
The Conduit status bar SHALL show the orchestration badge (On/Off indicator) when a Pi session has orchestration enabled.

#### Scenario: Status bar shows badge
- **WHEN** a Pi session has `orchestration_enabled` set
- **THEN** the status bar displays the orchestration badge, same as it does for Claude

### Requirement: Pi Agent tool calls detected for delegation badge
The agent events handler SHALL detect when the Pi agent calls the `Agent` tool with `subagent_type` matching conduit-explore or conduit-review, and display the delegation badge in the status bar.

#### Scenario: Delegation badge appears
- **WHEN** the Pi agent calls the `Agent` tool with `subagent_type: "conduit-explore"`
- **THEN** the status bar shows "Exploring..." badge, matching Claude's behavior

### Requirement: Adversarial review model configurable for Pi
The workspace config dialog SHALL allow the user to select which model the adversarial review sub-agent uses for Pi sessions. The selected model SHALL be stored in the session and passed to the Pi runner, which embeds it in the adversarial-review skill file.

#### Scenario: Model picker available
- **WHEN** adversarial review is enabled for a Pi session
- **THEN** the Review Model row is interactive and lets the user pick a model

#### Scenario: Model embedded in skill
- **WHEN** the Pi runner starts with adversarial review enabled
- **THEN** the conduit-adversarial-review skill file contains the selected model name in its metadata, and the extension uses it when spawning the review sub-agent
