## ADDED Requirements

### Requirement: Adversarial review settings stored per workspace and per repository
The `workspaces` table SHALL have two new nullable columns: `adversarial_review_enabled INTEGER` and `adversarial_review_model TEXT`. The `repositories` table SHALL have the same two columns. NULL in either column means "no override at this level". Both columns SHALL be added via a new database migration that applies automatically at startup.

#### Scenario: Existing workspace after migration
- **WHEN** conduit starts and applies Migration 23 against an existing database
- **THEN** existing workspace rows have `adversarial_review_enabled = NULL` and `adversarial_review_model = NULL`
- **AND** the feature is effectively off for those workspaces (no behaviour change)

#### Scenario: New workspace created with feature off
- **WHEN** a workspace is created and the user does not enable adversarial review in the config panel
- **THEN** `workspaces.adversarial_review_enabled` is stored as `NULL` or `0`

#### Scenario: New workspace created with feature on
- **WHEN** a workspace is created and the user enables adversarial review in the config panel
- **THEN** `workspaces.adversarial_review_enabled` is stored as `1`
- **AND** `workspaces.adversarial_review_model` is stored as the selected model string

### Requirement: Adversarial review config resolved from a three-level hierarchy
When a workspace session starts, conduit SHALL resolve the adversarial review enabled flag and model from (1) the workspace record, (2) the repository record, then (3) the hard defaults (`false` / `"claude-sonnet-4-6"`). The first non-NULL value at each level wins.

#### Scenario: Workspace-level override takes precedence
- **WHEN** the workspace record has `adversarial_review_enabled = 1` and the repository record has `adversarial_review_enabled = 0`
- **THEN** the session starts with adversarial review enabled

#### Scenario: Repository-level default used when workspace has no override
- **WHEN** the workspace record has `adversarial_review_enabled = NULL` and the repository record has `adversarial_review_enabled = 1`
- **THEN** the session starts with adversarial review enabled

#### Scenario: Hard default used when neither level is set
- **WHEN** both workspace and repository have `adversarial_review_enabled = NULL`
- **THEN** the session starts with adversarial review disabled

#### Scenario: Model resolution follows same hierarchy
- **WHEN** the workspace record has `adversarial_review_model = NULL` and the repository record has `adversarial_review_model = "claude-haiku-4-5"`
- **THEN** the session uses `"claude-haiku-4-5"` as the adversarial review model

#### Scenario: Hard default model used when no override
- **WHEN** both workspace and repository have `adversarial_review_model = NULL`
- **THEN** the session uses `"claude-sonnet-4-6"` as the adversarial review model

### Requirement: conduit-adversarial-review sub-agent definition written at session start
When a Claude session starts with adversarial review enabled, conduit SHALL write `conduit-adversarial-review.md` to `~/.claude/agents/` using the configured model in the frontmatter. The write SHALL be idempotent — only performed when the file content differs from what is on disk.

#### Scenario: Agent file written on first session with feature enabled
- **WHEN** a Claude session starts with adversarial review enabled and `~/.claude/agents/conduit-adversarial-review.md` does not exist
- **THEN** conduit writes the file with the configured model in the frontmatter

#### Scenario: Agent file updated when model changes
- **WHEN** a Claude session starts with adversarial review enabled and the on-disk file specifies a different model
- **THEN** conduit overwrites the file with the new model

#### Scenario: Agent file not rewritten when content matches
- **WHEN** a Claude session starts with adversarial review enabled and the on-disk file already matches the expected content
- **THEN** conduit does not write to disk

#### Scenario: Agent file not written when feature disabled
- **WHEN** a Claude session starts with adversarial review disabled
- **THEN** conduit does not write or modify `~/.claude/agents/conduit-adversarial-review.md`

### Requirement: conduit-adversarial-review sub-agent system prompt covers adversarial review areas
The `conduit-adversarial-review` agent definition SHALL instruct the agent to critique diffs for: correctness (logic errors, wrong assumptions, unhandled edge cases), security (injection, auth bypass, data exposure, insecure defaults), concurrency (race conditions, deadlocks, unsafe shared state), error handling (unchecked errors, incorrect fallbacks, panic paths), performance (unnecessary allocations, blocking in async context), API design (breaking changes, missing validation), and test coverage. The report SHALL use severity ratings: CRITICAL / HIGH / MEDIUM / LOW.

#### Scenario: Agent definition covers all review areas
- **WHEN** the `conduit-adversarial-review.md` file is written to disk
- **THEN** its system prompt explicitly lists correctness, security, concurrency, error handling, performance, API design, and test coverage as review areas

#### Scenario: Agent definition requires severity ratings
- **WHEN** the `conduit-adversarial-review.md` file is written to disk
- **THEN** its system prompt instructs the agent to use CRITICAL / HIGH / MEDIUM / LOW severity ratings
