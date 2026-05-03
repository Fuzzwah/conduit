## 1. Schema migration and Workspace persistence (PR 1)

- [ ] 1.1 Add `active_change_id: Option<String>` and `active_issue_number: Option<i32>` fields to the `Workspace` struct in `crates/conduit-data/src/models.rs`, defaulting to `None` in `Workspace::new`
- [ ] 1.2 Add builder helpers `Workspace::with_active_change(self, id)` and `Workspace::with_active_issue(self, n)` to keep call-sites tidy
- [ ] 1.3 Update `CREATE TABLE workspaces` in `crates/conduit-data/src/database.rs` to include the new columns
- [ ] 1.4 Append a guarded migration block in `database.rs::apply_migrations` that adds `active_change_id TEXT` and `active_issue_number INTEGER` to existing databases via `ALTER TABLE`, gated by `pragma_table_info` checks (mirror existing patterns in lines 154-165)
- [ ] 1.5 Extend the INSERT in `WorkspaceStore::create` (currently only 8 columns) to write the two new columns
- [ ] 1.6 Extend SELECT lists in `get_by_id`, `get_by_repository`, `get_all`, `get_default_for_repository`, and `get_by_path` to include the new columns
- [ ] 1.7 Update `row_to_workspace` to read the two new columns
- [ ] 1.8 Add `WorkspaceStore::update_active_links(id, active_change_id, active_issue_number)` that rewrites both columns atomically and is safe to call repeatedly
- [ ] 1.9 In `crates/conduit-ui/src/workspace_creation.rs`, propagate `picked_issue.as_ref().map(|i| i.number)` and `picked_spec.as_ref().map(|s| s.change_id.clone())` (and the equivalent for `picked_specify_spec`) into the workspace ctor at the `StartNaming → CreateWorkspace` transition
- [ ] 1.10 In `crates/conduit-ui/src/app.rs`, ensure the `Effect::CreateWorkspace` execution path passes the picked values through to `Workspace::new` + `with_active_change` / `with_active_issue`
- [ ] 1.11 Add unit tests for the migration: fresh DB has the columns; existing-row migration adds them as NULL; double migration is a no-op
- [ ] 1.12 Add unit tests for `WorkspaceStore::create` and `update_active_links` covering NULL and populated cases
- [ ] 1.13 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and confirm all pass

## 2. Pure work-complete primitives (PR 2)

- [ ] 2.1 Extend `crates/conduit-git/src/specs.rs` with `fetch_change_detail(repo_path, change_id) -> Option<SpecDetail>` returning `{ change_id, total, completed }` (counts only — full task line text is not needed since the dialog summarises rather than rendering each line)
- [ ] 2.2 Add `infer_active_change(repo_path, base_branch) -> Option<String>` to `specs.rs` running `git log --diff-filter=A --name-only origin/<base>..HEAD -- openspec/changes/`, returning the most-recently-modified change directory's basename if any
- [ ] 2.3 Add `infer_active_issue(branch_name) -> Option<i32>` to `crates/conduit-git/src/issues/github.rs` matching `#(\d+)` against the branch name
- [ ] 2.4 Add `close_issue(repo_path, number) -> Result<()>` to `crates/conduit-git/src/issues/github.rs` shelling `gh issue close <n>`
- [ ] 2.5 Create `crates/conduit-git/src/actions.rs` exporting `commit_all(path, message) -> Result<String>` (returns new SHA via `git add -A && git commit -m`) and `push_branch(path, branch, set_upstream) -> Result<()>` (runs `git push [-u] origin <branch>`)
- [ ] 2.6 Wire `actions.rs` into `crates/conduit-git/src/lib.rs` exports
- [ ] 2.7 Add `PrManager::create(path, opts: PrCreateOpts) -> Result<PrInfo>` to `crates/conduit-git/src/pr.rs` shelling `gh pr create --base <base>` with `--fill` when title/body absent, otherwise `--title <t> --body <b>`; return the PR URL and number from `gh` JSON output
- [ ] 2.8 Add `PrManager::merge(path, method: MergeMethod, admin: bool) -> Result<()>` to `pr.rs` shelling `gh pr merge --<method>` with optional `--admin`
- [ ] 2.9 Create `crates/conduit-git/src/openspec_archive.rs` exporting `archive_change(repo_path, change_id, today: NaiveDate) -> Result<ArchiveResult>` that verifies source exists, computes `openspec/changes/archive/YYYY-MM-DD-<id>`, refuses dup target, runs `std::fs::rename`, and returns `{ new_path, warnings: Vec<String> }`; warnings include a "delta specs not auto-synced" message when the change directory contains delta `.md` files under `specs/`
- [ ] 2.10 Wire `openspec_archive.rs` into `crates/conduit-git/src/lib.rs`
- [ ] 2.11 Create `crates/conduit-git/src/work_complete.rs` defining the `Scenario` enum (`CleanReady`, `EditsNoLink`, `SpecComplete`, `SpecIncomplete`, `IssueOpen`, `IssueClosed`), the `SuggestedAction` enum, and a pure `classify(git: &GitState, pr: Option<&PrSnapshot>, spec: Option<&SpecSnapshot>, issue: Option<&IssueSnapshot>) -> (Scenario, Vec<SuggestedAction>)` function
- [ ] 2.12 Add table-driven unit tests to `work_complete.rs`, one per scenario plus edge cases: PR exists but closed unmerged; commits ahead but no upstream; spec linked but `tasks.md` missing on disk; multi-spec branch; branch with no `#N` token; both spec and issue linked simultaneously
- [ ] 2.13 Add unit tests in `openspec_archive.rs` using `tempdir` (mirror the pattern in `specs.rs`): happy-path rename; refuses duplicate target with 409-style error; refuses missing source; preserves nested files; surfaces delta-spec warning
- [ ] 2.14 Add unit tests for `infer_active_change` (single dir, no dirs, multiple dirs picks most recent) and `infer_active_issue` (matches `#123`, returns None for none)
- [ ] 2.15 Add unit tests for `commit_all`, `push_branch`, `PrManager::create`, `PrManager::merge`, and `close_issue` using a tempdir-backed git repo with `gh` stubs (check existing test infra in `crates/conduit/tests/` for the stubbing pattern)
- [ ] 2.16 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and confirm all pass

## 3. Backend HTTP endpoints (PR 3)

- [ ] 3.1 Add `WorkCompletePreflight`, `GitState`, `DirtyFile`, `PrSnapshot`, `SpecSnapshot`, `IssueSnapshot`, `ContextSource` (`Linked` | `Detected`), `Scenario`, and `SuggestedAction` to `crates/conduit-web/src/handlers/models.rs` (or a new `work_complete_models.rs`) with serde derives
- [ ] 3.2 Add a `git_diff_files(path) -> Vec<DirtyFile>` helper to `crates/conduit-git/src/status.rs` enumerating dirty files (status code + path) since `GitDiffStats` only counts
- [ ] 3.3 Implement `GET /workspaces/{id}/work-complete/preflight` in `crates/conduit-web/src/handlers/workspaces.rs` that composes the response from existing services: `worktree_manager.get_branch_status_with_gh_option`, `git_diff_files`, `PrManager::preflight_check`, `fetch_change_detail`, `gh issue view`, then calls `work_complete::classify` for the scenario
- [ ] 3.4 Implement spec resolution: prefer `workspace.active_change_id`; if NULL, call `infer_active_change` and write the result back via `update_active_links`
- [ ] 3.5 Implement issue resolution: prefer `workspace.active_issue_number`; if NULL, call `infer_active_issue` and write the result back via `update_active_links`
- [ ] 3.6 Implement `POST /workspaces/{id}/work-complete/commit` body `{ message: String }` returning `{ status, log_lines, sha }`; reject empty messages with 400
- [ ] 3.7 Implement `POST /workspaces/{id}/work-complete/push` returning `{ status, log_lines }`; handle "already up to date" as success
- [ ] 3.8 Implement `POST /workspaces/{id}/work-complete/pr` body `{ title?, body? }` returning `{ status, log_lines, url, number }`; this is a *new* handler distinct from the existing prompt-returning `create_workspace_pr`
- [ ] 3.9 Implement `POST /workspaces/{id}/work-complete/pr/merge` body `{ method, admin }` returning `{ status, log_lines }`; refuse with 409 when `MergeReadiness != Ready` and `admin: false`
- [ ] 3.10 Implement `POST /workspaces/{id}/work-complete/issue/close` returning `{ status, log_lines }`; reject with 400 when no resolved issue link
- [ ] 3.11 Implement `POST /workspaces/{id}/work-complete/spec/archive` body `{ change_id }` returning `{ status, log_lines, new_path, warnings }`; surface 404 (missing source), 409 (existing target), and the delta-spec warning
- [ ] 3.12 Implement `POST /workspaces/{id}/work-complete/archive` reusing the existing `archive_workspace` body verbatim (worktree removal, branch deletion, DB update, session close, status manager cleanup)
- [ ] 3.13 Register all new routes in `crates/conduit-web/src/routes/api.rs` near the existing archive routes
- [ ] 3.14 Mark the legacy `/workspaces/{id}/archive/preflight` and the legacy archive `POST /workspaces/{id}` route as deprecated in code comments (deletion happens in PR 6); leave them functional
- [ ] 3.15 Add integration tests in `crates/conduit-web/tests/` (or `crates/conduit/tests/`) that spin up a temp repo per scenario and assert the preflight response shape, including: clean repo → `CleanReady`; dirty repo no-link → `EditsNoLink`; dirty repo with linked complete spec → `SpecComplete` with matching `total` and `completed` counts; dirty repo with linked incomplete spec → `SpecIncomplete` with matching counts; closed-issue link → `IssueClosed`; open-issue link → `IssueOpen`
- [ ] 3.16 Add integration tests for each action endpoint covering happy path, idempotency (push when up-to-date; archive-spec when already archived), and error paths (commit with empty message; merge with `Blocked` readiness)
- [ ] 3.17 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and confirm all pass

## 4. TUI dialog and action rename (PR 4)

- [ ] 4.1 Create `crates/conduit-ui/src/work_complete.rs` defining `WorkCompleteSession` (workspace_id, phase, preflight: Option, pending_action: Option, log: Vec<String>), `WorkCompletePhase` (`Idle`, `LoadingPreflight`, `ReviewingState{scenario}`, `AwaitingCommitMessage{suggestion}`, `Executing{action}`, `ConfirmingForceComplete{kind}`, `Done`), `WorkCompleteEvent`, and a pure `transition(phase, event) -> (next_phase, Vec<WorkCompleteCommand>)` mirroring `workspace_creation.rs`
- [ ] 4.2 Add unit tests for `transition`: happy-path scenario walks; force-confirm ordering; cancel from each phase; loop back from `Executing` to `ReviewingState` after each action
- [ ] 4.3 Create `crates/conduit-ui/src/components/work_complete_dialog.rs` rendering the dialog per phase: status sections (always-show git, conditionally-show PR / spec / issue), action list with merge gating, force-confirm sub-phase UI, commit-message input with pre-fill, scrolling log panel
- [ ] 4.4 Compute the pre-filled commit message from branch name + first 1-2 dirty file paths + `Implement <change_id>` (when linked) + `Fix #<n>` (when linked)
- [ ] 4.5 Render the spec section as a single-line summary (`<change_id> (<source>) — X of Y tasks complete`) for both `SpecComplete` and `SpecIncomplete`; do not render the full task list inline
- [ ] 4.5a Add a "Show remaining tasks" action to the dialog's action list that is enabled only when the resolved spec has at least one incomplete task; selecting it closes the Work Complete dialog and dispatches the prompt `show incomplete tasks in <change_id>` to the workspace's active agent session (reuse whichever existing pattern the codebase uses for sending prompts to the agent — check `app_prompt` in `crates/conduit-types/` and the agent-prompt dispatch path)
- [ ] 4.6 Render the issue section showing number, title, state, URL, source for both `IssueOpen` and `IssueClosed`
- [ ] 4.7 Add `Effect::WorkCompletePreflight { workspace_id }` and `Effect::WorkCompleteAction { workspace_id, action: SuggestedAction, payload: serde_json::Value }` to `crates/conduit-ui/src/effect.rs`
- [ ] 4.8 Add corresponding `AppEvent::WorkCompletePreflightLoaded`, `AppEvent::WorkCompleteActionFinished`, etc., and wire their handling in `crates/conduit-ui/src/app.rs`
- [ ] 4.9 Rename `Action::ArchiveCurrentWorkspace` → `Action::CompleteWorkspaceWork` in `crates/conduit-ui/src/action.rs`
- [ ] 4.10 Add a parser alias entry mapping the legacy string `"ArchiveCurrentWorkspace"` to the new variant in the keybinding parser
- [ ] 4.11 Update `crates/conduit-config/src/default_keys.rs:58` to bind `M-S-x` to `Action::CompleteWorkspaceWork`
- [ ] 4.12 Update sidebar key handling so the workspace-row `x` button dispatches `Action::CompleteWorkspaceWork`
- [ ] 4.13 Migrate the 144 `app.rs` references to legacy `Archive*` symbols to the new flow: replace `initiate_archive_workspace`, archive-related effects/events, and `ConfirmationContext::ArchiveWorkspace*` paths with the new state-machine session
- [ ] 4.14 Delete `ConfirmationContext::ArchiveWorkspace`, `ConfirmationContext::ArchiveWorkspaceRemoteDelete`, and any remaining archive-only confirmation variants once the new flow is in place
- [ ] 4.15 Update the help/keybindings dialog text so `M-S-x` is labelled "Complete workspace work" (or similar) rather than "Archive workspace"
- [ ] 4.16 Add `insta` snapshot tests for each `WorkCompletePhase` rendering: `LoadingPreflight`, `ReviewingState{CleanReady}`, `ReviewingState{EditsNoLink}`, `ReviewingState{SpecComplete}` (with summary line), `ReviewingState{SpecIncomplete}` (with summary line + "Show remaining tasks" action visible), `ReviewingState{IssueOpen}`, `ReviewingState{IssueClosed}`, `AwaitingCommitMessage`, `ConfirmingForceComplete{Spec}`, `ConfirmingForceComplete{Issue}`, `Executing`, `Done`
- [ ] 4.17 Add a `termwright` E2E test under the existing E2E test directory: create workspace linked to issue + spec, edit a file, press `M-S-x`, walk through commit (accept pre-filled message) → push → open PR → merge → archive-spec → archive workspace
- [ ] 4.18 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and confirm all pass

## 5. Web dialog (PR 5)

- [ ] 5.1 Create `crates/conduit-web/web/src/components/WorkCompleteDialog.tsx` with internal state mirroring `WorkCompletePhase` as a discriminated union (`'loading' | 'reviewing' | 'awaitingCommitMessage' | 'executing' | 'forceConfirm' | 'done'`)
- [ ] 5.2 Implement preflight fetch via TanStack Query (`useQuery(['work-complete', workspaceId])` or whichever pattern matches the codebase) hitting `GET /workspaces/{id}/work-complete/preflight`
- [ ] 5.3 Implement one `useMutation` per action endpoint (commit, push, pr, pr/merge, issue/close, spec/archive, archive); each appends `log_lines` to a scrolling log component
- [ ] 5.4 Render status sections: always-show git block; conditionally-show PR block when `pr` is non-null; conditionally-show spec block (single-line summary `<change_id> (<source>) — X of Y tasks complete`) when `spec` is non-null; conditionally-show issue block (with state, title, url) when `issue` is non-null
- [ ] 5.5 Render action buttons gated by `suggested_actions` and merge readiness; disable merge with the readiness reason as a tooltip when not `Ready`
- [ ] 5.5a Render a "Show remaining tasks" button when the resolved spec has at least one incomplete task; clicking it closes `WorkCompleteDialog` and dispatches the prompt `show incomplete tasks in <change_id>` to the workspace's active agent session via the existing prompt-dispatch hook
- [ ] 5.6 Implement the force-confirm sub-flow for `SpecIncomplete` and `IssueOpen` (require explicit "Complete anyway" click before action buttons appear)
- [ ] 5.7 Implement the commit-message input with the same pre-fill rules as the TUI (branch + dirty files + `Implement <change_id>` / `Fix #<n>`)
- [ ] 5.8 Implement the admin-merge secondary confirm dialog (separate from the main `WorkCompleteDialog`)
- [ ] 5.9 Update `crates/conduit-web/web/src/components/Sidebar.tsx` so the workspace-row `x` button opens `WorkCompleteDialog` instead of the legacy archive confirm path
- [ ] 5.10 Update `crates/conduit-web/web/src/hooks/useWorkspaceActions.tsx` to replace the archive method with work-complete equivalents (or delete the archive method and let `WorkCompleteDialog` own its own mutations)
- [ ] 5.11 Manually exercise all six scenarios via `cargo run -- serve` and a browser; verify the same status sections render in both UIs for the same backend state
- [ ] 5.12 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and confirm all pass

## 6. Cleanup (PR 6)

- [ ] 6.1 Delete the legacy `GET /workspaces/{id}/archive/preflight` handler and its route registration
- [ ] 6.2 Delete the legacy archive `POST /workspaces/{id}` variant (or the `archive_workspace` handler and its route) now that `/work-complete/archive` is the canonical path
- [ ] 6.3 Delete `ArchivePreflightResponse` and any related request/response types in `handlers/models.rs`
- [ ] 6.4 Remove any vestigial `ArchiveWorkspace*` symbols from the TUI codebase (final sweep)
- [ ] 6.5 Update `crates/conduit-web/web/src/` to remove archive-specific API client functions if any remain
- [ ] 6.6 Update `FORK_CHANGES.md` with the user-visible change description (replaced archive with Work Complete)
- [ ] 6.7 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and confirm all pass
