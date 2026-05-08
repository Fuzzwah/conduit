## 1. Database Migration

- [x] 1.1 Add Migration 23 to `crates/conduit-data/src/database.rs` adding `adversarial_review_enabled INTEGER` and `adversarial_review_model TEXT` columns to both the `workspaces` and `repositories` tables, following the pattern of Migration 21

## 2. Data Models and Repositories

- [x] 2.1 Add `adversarial_review_enabled: Option<bool>` and `adversarial_review_model: Option<String>` fields to the `Workspace` struct in `crates/conduit-data/src/models.rs`
- [x] 2.2 Add the same two fields to the `Repository` struct in `crates/conduit-data/src/models.rs`
- [x] 2.3 Update `WorkspaceStore` in `crates/conduit-data/src/workspace.rs` — extend INSERT, UPDATE, and SELECT queries to include the two new columns, using the same `Option<bool>` → `Option<i64>` serialisation pattern as `orchestration_enabled`
- [x] 2.4 Update `RepositoryStore` in `crates/conduit-data/src/repository.rs` — same INSERT/UPDATE/SELECT changes

## 3. Orchestration Agent Infrastructure

- [x] 3.1 Add `AdversarialReviewConfig { enabled: bool, model: String }` struct to `crates/conduit-agent/src/orchestration.rs`
- [x] 3.2 Add `ADVERSARIAL_REVIEW_AGENT_DEF_TEMPLATE` constant containing the adversarial reviewer system prompt (covering correctness, security, concurrency, error handling, performance, API design, test coverage; CRITICAL/HIGH/MEDIUM/LOW severity ratings; configurable `{MODEL}` placeholder in frontmatter)
- [x] 3.3 Update `ensure_orchestration_agents()` signature to accept `adversarial_review: Option<AdversarialReviewConfig>` and, when enabled, write `conduit-adversarial-review.md` with the model substituted, using the same idempotent write pattern as the existing agent files
- [x] 3.4 Add `adversarial_review: Option<AdversarialReviewConfig>` field and `.with_adversarial_review(cfg)` builder method to `AgentStartConfig` in `crates/conduit-agent/src/runner.rs`
- [x] 3.5 Update the `ensure_orchestration_agents()` call site in `crates/conduit-agent/src/claude.rs` to pass `config.adversarial_review.clone()`

## 4. SuggestedAction and Work Complete Git Logic

- [x] 4.1 Add `AdversarialReview` variant to the `SuggestedAction` enum in `crates/conduit-git/src/work_complete.rs`, with a doc comment noting it is TUI/web only and never executed server-side
- [x] 4.2 Update the `suggested_actions_for()` (or equivalent builder) function in `crates/conduit-git/src/work_complete.rs` to accept an `adversarial_review_enabled: bool` parameter and push `SuggestedAction::AdversarialReview` when enabled and `(git_state.is_dirty || git_state.commits_ahead > 0)`

## 5. Session State

- [x] 5.1 Add `adversarial_review_enabled: bool` and `adversarial_review_model: Option<String>` to the session state struct in `crates/conduit-ui/src/app.rs` (or the dedicated session state file)
- [x] 5.2 Resolve these fields when opening a workspace using the three-level hierarchy (workspace override → repository default → hard defaults: `false` / `"claude-sonnet-4-6"`), following the same pattern as `orchestration_enabled`
- [x] 5.3 Pass `adversarial_review_enabled` and `adversarial_review_model` to `AgentStartConfig` via `.with_adversarial_review()` when building the config for a Claude session start

## 6. Workspace Ready Config Dialog

- [x] 6.1 Add `adversarial_review_enabled: bool` and `adversarial_review_model: String` to `WorkspaceReadyConfigState` in `crates/conduit-ui/src/components/workspace_progress_dialog.rs`
- [x] 6.2 Add `toggle_adversarial_review()` method (mirrors `toggle_orchestration()`) and `set_adversarial_review_model(model: String)` method
- [x] 6.3 Initialise `WorkspaceReadyConfigState` from the resolved workspace/repository values (following the same defaults-chain logic added in task 5.2)
- [x] 6.4 Add an "Adversarial Review" toggle row to the config panel render function (Off/On; dimmed when provider is not Claude), following the Orchestration row pattern
- [x] 6.5 Add a "Review Model" text-input row below the Adversarial Review row, visible only when adversarial review is On
- [x] 6.6 Wire keyboard handling for the two new rows (Space/Left/Right to toggle, character input for the model row)
- [x] 6.7 When the user confirms ("Continue"), save `adversarial_review_enabled` and `adversarial_review_model` to the workspace record via `workspace_dao.update()`
- [x] 6.8 When "Set as project default" is checked and the user confirms, also write `adversarial_review_enabled` and `adversarial_review_model` to the repository record via `repository_dao.update()`

## 7. Work Complete State Machine and Preflight

- [x] 7.1 Add `adversarial_review_model: Option<String>` to `WorkCompleteData` in `crates/conduit-ui/src/work_complete.rs`
- [x] 7.2 In `select_action()` in `crates/conduit-ui/src/work_complete.rs`, add handling for `SuggestedAction::AdversarialReview` that emits `SendAgentPrompt(String::new())` and `Close` (matching the `ShowRemainingTasks` pattern)
- [x] 7.3 Update the Work Complete preflight endpoint (in `crates/conduit-web/` or `crates/conduit-core/`) to fetch `workspace.adversarial_review_enabled` and `workspace.adversarial_review_model`, pass the enabled flag to `suggested_actions_for()`, and include the model in the preflight response
- [x] 7.4 Add a display label for `SuggestedAction::AdversarialReview` in `crates/conduit-ui/src/components/work_complete_dialog.rs` (e.g. "Adversarial Review")

## 8. Prompt Building and Command Dispatch (app.rs)

- [x] 8.1 In the `WorkCompleteCommand::SendAgentPrompt` handler in `crates/conduit-ui/src/app.rs`, detect when the pending Work Complete action is `AdversarialReview` and build the structured review prompt (PR diff path if `session.data.pr` is `Some` and open, otherwise local diff path) and submit it via `submit_prompt`
- [x] 8.2 Add `ConduitCommand::AdversarialReview` variant to the `ConduitCommand` enum in `crates/conduit-resolver/src/lib.rs`
- [x] 8.3 Register `/adversarial-review` in `inject_claude_builtins()` in `crates/conduit-resolver/src/lib.rs` with description "Run adversarial code review on workspace changes"
- [x] 8.4 Handle `ConduitCommand::AdversarialReview` in `execute_resolved_conduit_command()` in `crates/conduit-ui/src/app.rs` by building and submitting the same structured review prompt used by the Work Complete path

## 9. Verification

- [x] 9.1 Run `cargo fmt --check` and fix any formatting issues
- [x] 9.2 Run `cargo clippy --workspace --all-targets -- -D warnings` and fix all warnings
- [x] 9.3 Run `cargo test --workspace` and confirm all tests pass
- [x] 9.4 Manual test: create a workspace, confirm "Adversarial Review" toggle appears in the config panel; enable it with model `claude-haiku-4-5`; verify `~/.claude/agents/conduit-adversarial-review.md` is written with the correct model on session start
- [x] 9.5 Manual test: make a code change in a workspace with adversarial review enabled; open Work Complete; confirm "Adversarial Review" appears in the action list; select it and confirm the primary agent receives and acts on the review prompt
- [x] 9.6 Manual test: type `/adversarial-review` in a Claude session; confirm the review prompt is injected into the session
- [x] 9.7 Manual test: open Work Complete in a workspace with adversarial review disabled; confirm "Adversarial Review" does not appear
