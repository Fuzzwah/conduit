## ADDED Requirements

### Requirement: Sub-agent definition files are written to user's Claude agents directory
When a Claude session starts with orchestration enabled, conduit SHALL write `conduit-explore.md` and `conduit-review.md` to `~/.claude/agents/`. Writes SHALL be idempotent — files are only written (or updated) when the content differs from what is on disk. The `~/.claude/agents/` directory SHALL be created if it does not exist.

#### Scenario: First session start with orchestration enabled
- **WHEN** a Claude session starts with orchestration enabled and `~/.claude/agents/conduit-explore.md` does not exist
- **THEN** conduit writes the file with the `conduit-explore` agent definition (model: `claude-haiku-4-5`, read-only exploration system prompt)

#### Scenario: Subsequent session start with unchanged definitions
- **WHEN** a Claude session starts with orchestration enabled and `~/.claude/agents/conduit-explore.md` already contains the current definition
- **THEN** conduit does not rewrite the file

#### Scenario: Subsequent session start after definition upgrade
- **WHEN** a Claude session starts with orchestration enabled and `~/.claude/agents/conduit-explore.md` contains an older definition
- **THEN** conduit overwrites the file with the current definition

#### Scenario: Session start with orchestration disabled
- **WHEN** a Claude session starts with orchestration disabled
- **THEN** conduit does NOT write or modify any agent definition files

### Requirement: Orchestration instructions are injected into the session prompt
When orchestration is enabled for a Claude session, conduit SHALL append a clearly demarcated instruction block to the session prompt. The block SHALL instruct Claude to delegate codebase exploration tasks to `conduit-explore` and diff review tasks to `conduit-review` via the `Agent` tool.

#### Scenario: Session prompt with orchestration enabled
- **WHEN** a Claude session starts with orchestration enabled and the user prompt is "Refactor the session service"
- **THEN** the prompt sent to the Claude CLI includes the user's original text followed by a separator and the orchestration instruction block

#### Scenario: Session prompt with orchestration disabled
- **WHEN** a Claude session starts with orchestration disabled
- **THEN** the prompt sent to the Claude CLI is unchanged from what the user provided

### Requirement: Per-session orchestration toggle in the TUI
The TUI SHALL provide a per-session toggle to enable or disable orchestration mode. The toggle SHALL follow the same UX pattern as the reasoning effort and model selectors (modal overlay, discoverable via command palette).

#### Scenario: Toggle orchestration on via selector
- **WHEN** the user opens the orchestration selector and chooses "Enabled"
- **THEN** the current session's orchestration state is set to enabled and the next agent run uses orchestration

#### Scenario: Toggle orchestration off via selector
- **WHEN** the user opens the orchestration selector and chooses "Disabled"
- **THEN** the current session's orchestration state is set to disabled

#### Scenario: Orchestration selector only available for Claude sessions
- **WHEN** the active session uses a non-Claude agent (Codex, Gemini, etc.)
- **THEN** the orchestration selector is not shown or is disabled

### Requirement: Global default for orchestration mode
The conduit config SHALL support an `orchestration.enabled_by_default` boolean field. New Claude sessions SHALL inherit this default for their initial orchestration state. The default value SHALL be `false` when the field is absent from `conduit.toml`.

#### Scenario: Config with orchestration disabled by default
- **WHEN** `conduit.toml` does not contain an `[orchestration]` section
- **THEN** new Claude sessions start with orchestration disabled

#### Scenario: Config with orchestration enabled by default
- **WHEN** `conduit.toml` contains `[orchestration]` with `enabled_by_default = true`
- **THEN** new Claude sessions start with orchestration enabled

### Requirement: conduit-explore sub-agent definition
The `conduit-explore` agent definition SHALL specify `claude-haiku-4-5` as the model and include a system prompt that constrains the agent to: complete tasks in 3–8 tool calls, return concise summaries (not raw file dumps), parallelize independent searches, and not edit files.

#### Scenario: conduit-explore system prompt enforces read-only operation
- **WHEN** the `conduit-explore` agent definition is written to disk
- **THEN** its system prompt contains an explicit instruction not to edit files

#### Scenario: conduit-explore uses Haiku model
- **WHEN** the `conduit-explore` agent definition is written to disk
- **THEN** its frontmatter specifies `model: claude-haiku-4-5`

### Requirement: conduit-review sub-agent definition
The `conduit-review` agent definition SHALL specify `claude-haiku-4-5` as the model and include a system prompt that constrains the agent to: analyze diffs and code changes, return a brief structured report covering correctness, bugs/regressions, security, and performance concerns, and not repeat large code blocks.

#### Scenario: conduit-review system prompt focuses on analysis
- **WHEN** the `conduit-review` agent definition is written to disk
- **THEN** its system prompt explicitly covers correctness, bugs/regressions, security, and performance as review areas

#### Scenario: conduit-review uses Haiku model
- **WHEN** the `conduit-review` agent definition is written to disk
- **THEN** its frontmatter specifies `model: claude-haiku-4-5`

### Requirement: Orchestration default surfaced in workspace-ready config panel
When a new workspace is created successfully, the orchestration state for the first session SHALL be configurable via the workspace-ready config panel's Orchestration row, before the workspace tab is opened. The initial value SHALL be resolved from the defaults chain (workspace override → project override → global config). The Orchestration row SHALL be non-interactive when the selected provider is not Claude.

#### Scenario: Orchestration row reflects workspace override
- **WHEN** the workspace record has `orchestration_enabled = true`
- **THEN** the Orchestration row shows On when the config panel opens

#### Scenario: Orchestration row reflects project default
- **WHEN** the workspace has no `orchestration_enabled` override and the repository has `orchestration_enabled = false`
- **THEN** the Orchestration row shows Off when the config panel opens

#### Scenario: Orchestration row reflects global config
- **WHEN** neither workspace nor repository has an `orchestration_enabled` override
- **AND** `conduit.toml` has `orchestration.enabled_by_default = true`
- **THEN** the Orchestration row shows On when the config panel opens

#### Scenario: Orchestration row non-interactive for non-Claude provider
- **WHEN** the selected provider in the config panel is not Claude
- **THEN** the Orchestration row is dimmed and Space/Left/Right have no effect on it

### Requirement: ensure_orchestration_agents accepts optional adversarial review config
The `ensure_orchestration_agents()` function SHALL accept an optional `AdversarialReviewConfig` parameter containing `enabled: bool` and `model: String`. When `enabled` is `true`, the function SHALL write `conduit-adversarial-review.md` to `~/.claude/agents/` using the specified model in the frontmatter. This write follows the same idempotency rule as the existing `conduit-explore.md` and `conduit-review.md` writes.

#### Scenario: Adversarial review agent written when enabled
- **WHEN** `ensure_orchestration_agents(Some(AdversarialReviewConfig { enabled: true, model: "claude-sonnet-4-6".into() }))` is called
- **AND** the file does not exist or has different content
- **THEN** `~/.claude/agents/conduit-adversarial-review.md` is written with `model: claude-sonnet-4-6` in its frontmatter

#### Scenario: Adversarial review agent not written when disabled
- **WHEN** `ensure_orchestration_agents(Some(AdversarialReviewConfig { enabled: false, .. }))` is called
- **THEN** conduit does not write or modify `~/.claude/agents/conduit-adversarial-review.md`

#### Scenario: Adversarial review agent not written when config absent
- **WHEN** `ensure_orchestration_agents(None)` is called
- **THEN** conduit does not write or modify `~/.claude/agents/conduit-adversarial-review.md`

#### Scenario: Adversarial review agent updated when model changes
- **WHEN** `ensure_orchestration_agents` is called with `model: "claude-haiku-4-5"` and the on-disk file specifies `model: claude-sonnet-4-6`
- **THEN** conduit overwrites the file with `model: claude-haiku-4-5`

### Requirement: AgentStartConfig carries adversarial review configuration
The `AgentStartConfig` struct SHALL have an `adversarial_review: Option<AdversarialReviewConfig>` field. The `.with_adversarial_review(cfg)` builder method SHALL set this field. The Claude agent startup path SHALL pass this field to `ensure_orchestration_agents`.

#### Scenario: AgentStartConfig with adversarial review passes config to orchestration
- **WHEN** a Claude session is started with `AgentStartConfig::new().with_adversarial_review(cfg)` where `cfg.enabled = true`
- **THEN** `ensure_orchestration_agents` is called with that config
- **AND** the `conduit-adversarial-review.md` agent file is written accordingly

#### Scenario: AgentStartConfig without adversarial review does not break orchestration
- **WHEN** a Claude session is started without calling `.with_adversarial_review()`
- **THEN** `ensure_orchestration_agents` is called with `None` for the adversarial review config
- **AND** the existing `conduit-explore.md` and `conduit-review.md` files are still written normally (orchestration behaviour unchanged)

### Requirement: Project-level provider and model defaults stored in repository record
The `repositories` table SHALL store `default_provider` (TEXT, nullable) and `default_model` (TEXT, nullable) columns. These represent per-project overrides for the provider and model used when opening new sessions in that project's workspaces. NULL means "inherit global config".

#### Scenario: Repository with no provider default uses global config
- **WHEN** a repository has `default_provider = NULL`
- **THEN** the workspace-ready config panel resolves provider from global config

#### Scenario: Repository with provider default uses it
- **WHEN** a repository has `default_provider = "Gemini"`
- **THEN** the workspace-ready config panel shows Gemini as the initial provider selection

#### Scenario: Saving project defaults writes provider and model columns
- **WHEN** the user checks "Set as project default" and confirms the config panel with provider=Gemini, model=gemini-2.5-pro
- **THEN** the repository record has `default_provider = "Gemini"` and `default_model = "gemini-2.5-pro"` after the update
