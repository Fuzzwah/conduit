## ADDED Requirements

### Requirement: /adversarial-review slash command available for Claude sessions
The `/adversarial-review` command SHALL appear in the slash menu for Claude sessions. It SHALL be registered as a Claude builtin command (alongside `/review`, `/compact`, etc.) and SHALL be available regardless of whether adversarial review is enabled in the workspace config.

#### Scenario: Command appears in slash menu for Claude sessions
- **WHEN** the active session uses Claude and the user types `/adv`
- **THEN** the slash menu shows an `/adversarial-review` entry with a description of its purpose

#### Scenario: Command not available for non-Claude sessions
- **WHEN** the active session uses a non-Claude agent (Codex, Gemini, etc.)
- **THEN** `/adversarial-review` does not appear in the slash menu

### Requirement: /adversarial-review injects the review prompt into the active session
Selecting `/adversarial-review` from the slash menu or submitting it as a command SHALL inject a structured adversarial review prompt into the active session via `submit_prompt`. The prompt SHALL instruct the primary agent to: (1) check for an open PR via `gh pr view`, (2) retrieve the diff from `gh pr diff` if a PR exists or from `git diff` against the branch base if not, (3) invoke the `conduit-adversarial-review` sub-agent with the diff, and (4) report findings by severity and offer to fix CRITICAL and HIGH issues immediately.

#### Scenario: Command with open PR uses PR diff
- **WHEN** the user invokes `/adversarial-review` and there is an open PR for the current branch
- **THEN** the injected prompt instructs the agent to use `gh pr diff` to obtain the diff

#### Scenario: Command with no PR uses local diff
- **WHEN** the user invokes `/adversarial-review` and there is no open PR
- **THEN** the injected prompt instructs the agent to use `git diff` against the branch base

#### Scenario: Command is a new ConduitCommand variant
- **WHEN** the user selects `/adversarial-review` from the slash menu
- **THEN** it is dispatched as `ConduitCommand::AdversarialReview` (not as a passthrough prompt)
- **AND** app.rs builds and submits the structured review prompt

### Requirement: Work Complete dialog offers Adversarial Review when enabled and changes exist
When the Work Complete preflight runs for a workspace that has adversarial review enabled AND there are changes to review (`is_dirty || commits_ahead > 0`), the dialog SHALL include `SuggestedAction::AdversarialReview` in the suggested actions list. The action SHALL NOT appear when adversarial review is disabled for the workspace or when there are no changes.

#### Scenario: Action shown when enabled and changes exist
- **WHEN** the workspace has `adversarial_review_enabled = true`
- **AND** the workspace has uncommitted changes or commits ahead of the base branch
- **THEN** "Adversarial Review" appears in the Work Complete suggested actions list

#### Scenario: Action not shown when feature disabled
- **WHEN** the workspace has `adversarial_review_enabled = false` (or NULL)
- **THEN** "Adversarial Review" does not appear in the Work Complete suggested actions list

#### Scenario: Action not shown when no changes
- **WHEN** the workspace has `adversarial_review_enabled = true`
- **AND** the working tree is clean and there are no commits ahead of the base branch
- **THEN** "Adversarial Review" does not appear in the Work Complete suggested actions list

### Requirement: Selecting Adversarial Review in Work Complete injects the review prompt and closes the dialog
When the user selects the Adversarial Review action in the Work Complete dialog, the state machine SHALL emit `SendAgentPrompt` and `Close`. App.rs SHALL build the same structured review prompt used by the slash command, including the PR-or-local-diff logic, and submit it to the active session.

#### Scenario: Selecting Adversarial Review closes dialog and submits prompt
- **WHEN** the user selects "Adversarial Review" in the Work Complete dialog
- **THEN** the dialog closes
- **AND** the primary agent receives the structured adversarial review prompt

#### Scenario: Prompt includes PR diff instruction when PR is open
- **WHEN** the Work Complete data includes an open PR
- **THEN** the injected prompt instructs the agent to use `gh pr diff` for the diff

#### Scenario: Prompt includes local diff instruction when no PR
- **WHEN** the Work Complete data has no open PR
- **THEN** the injected prompt instructs the agent to use `git diff` for the diff
