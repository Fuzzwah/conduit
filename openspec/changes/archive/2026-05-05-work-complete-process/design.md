## Context

Conduit's archive flow today is a single-purpose preflight that returns a small struct (`is_dirty`, `commits_ahead`, `commits_behind`, `is_merged`, `remote_branch_exists`, plus a name-based heuristic that warns about incomplete OpenSpec/Specify tasks). The TUI binds `M-S-x → Action::ArchiveCurrentWorkspace`, the web exposes `GET /workspaces/{id}/archive/preflight` and a `POST /workspaces/{id}` archive variant. Both UIs use the existing `ConfirmationDialog` (TUI) / `ConfirmDialog.tsx` (web) primitives.

The infrastructure for richer introspection already exists, just not composed:
- `crates/conduit-git/src/status.rs` produces `GitDiffStats` and `get_ahead_behind`.
- `crates/conduit-git/src/worktree.rs` exposes `BranchStatus { is_dirty, is_merged, commits_ahead, commits_behind, has_upstream, ... }`.
- `crates/conduit-git/src/pr.rs` `PrManager::preflight_check` already returns full PR status (number, state, checks, mergeable, review decision, `MergeReadiness`).
- `crates/conduit-git/src/specs.rs` `fetch_open_specs` walks `openspec/changes/*/tasks.md` (counts only — no remaining-line text).
- `crates/conduit-git/src/issues/github.rs` shells `gh issue list/view`.
- `crates/conduit-web/src/status_manager.rs` already aggregates per-workspace status for the web UI.

What is NOT in place:
- Workspaces don't persist which OpenSpec change or GitHub issue they were created for. The pickers in `workspace_creation.rs` collect `picked_issue` and `picked_spec` but throw them away.
- No Rust-side wrappers for `git commit`, `git push`, `gh pr create` (today's `create_workspace_pr` returns a *prompt* for the agent), `gh pr merge`, `gh issue close`, or the OpenSpec archive directory move.
- The TUI confirmation dialog is single-shot; multi-step branching flows use a state-machine pattern (see `workspace_creation.rs`).

`app.rs` contains 144 references to the legacy `Archive*` types — the rename has a real blast radius.

## Goals / Non-Goals

**Goals:**
- One context-aware "Work Complete" flow that replaces archive end-to-end.
- A single backend introspection endpoint that returns the full state, the classified scenario, and the suggested actions, so TUI and web stay in lockstep.
- Inline execution of every follow-up action (commit, push, PR create, PR merge, issue close, spec archive, workspace archive) without leaving the dialog.
- Persisted spec/issue linkage at workspace creation, with a worktree-scan fallback for legacy workspaces.
- Backward-compatible keybinding alias so user keybinding configs naming `ArchiveCurrentWorkspace` keep working.

**Non-Goals:**
- OpenSpec spec-sync (applying delta specs to the canonical specs directory). Out of scope for v1; the dialog warns and points users at `/opsx:archive` in the agent.
- Replacing the agent-driven PR creation flow. The existing `create_workspace_pr` prompt path stays available; we add a *real* `gh pr create` invocation as a parallel path the dialog uses.
- Multi-repo / cross-workspace bulk completion.
- Backporting the `active_change_id` / `active_issue_number` fields onto archived rows (legacy archived workspaces stay NULL forever).
- Octocrab migration (continues using `gh` CLI).

## Decisions

### D1. Replace archive entirely (vs. ship alongside)

`Action::ArchiveCurrentWorkspace` becomes `Action::CompleteWorkspaceWork`. The keybinding `M-S-x` and the sidebar `x` button are reused. The "clean workspace, no edits" path inside the new flow IS the quick-archive experience.

**Why:** Two parallel flows fragment muscle memory and create maintenance overhead. The new flow's `CleanReady` scenario is functionally equivalent to today's quick archive.

**Alternative considered:** ship a new `M-S-c` binding alongside archive. Rejected — "what's the difference?" friction outweighs the migration risk, and a parser alias for the legacy action name covers customised configs.

### D2. Hybrid spec/issue linkage (persisted + inferred)

Add `active_change_id: Option<String>` and `active_issue_number: Option<i32>` columns to `workspaces`. Populate at creation from the picker selections. For legacy workspaces with NULL columns, infer at preflight time:
- Spec: `git log --diff-filter=A --name-only origin/<base>..HEAD -- openspec/changes/` returns the directory created by this branch's commits.
- Issue: regex `#(\d+)` against branch name (and optionally first-commit message).

On successful inference, write the values back via `WorkspaceStore::update_active_links` so subsequent loads are O(1) and the dialog shows consistent state.

**Why:** Pure persistence loses data for every existing workspace. Pure inference is heuristic and brittle (branch renames, multi-spec branches, no-issue branches). The hybrid keeps new workspaces authoritative while degrading gracefully for old ones.

**Alternative considered:** persist-only with a one-shot inference migration at startup. Rejected — startup-time `git` shell-outs across N workspaces is slow and noisy; lazy inference at preflight time is bounded to one workspace at a time.

### D3. One introspection endpoint, granular action endpoints

`GET /workspaces/{id}/work-complete/preflight` returns the *whole* state (git, pr, spec, issue, classified scenario, suggested actions). Every action gets its own endpoint:
`POST .../commit`, `.../push`, `.../pr`, `.../pr/merge`, `.../issue/close`, `.../spec/archive`, `.../archive`.

The dialog drives sequencing — the backend has no notion of "which step is next."

**Why:** Composable, idempotent, easy to test in isolation. A monolithic "do all these things" endpoint conflates UX (sequencing, retries on partial failure) with state changes; splitting them lets the TUI and web share endpoint contracts but choose their own UX.

**Alternative considered:** a single `POST .../execute` taking a list of actions. Rejected — partial failures become hard to surface and per-action retries would need duplicated logic.

### D4. Pure scenario classifier

`crates/conduit-git/src/work_complete.rs::classify(git, pr, spec, issue) -> Scenario` is a pure function over already-fetched inputs. Endpoint handler is the only IO boundary; classifier is unit-tested with table-driven cases per scenario.

**Why:** Six scenarios with overlapping inputs is exactly where bugs hide; making the rules pure and tested in isolation eliminates whole categories of "does this scenario classify right?" regressions.

### D5. TUI state machine (not extended `ConfirmationContext`)

New top-level overlay session `WorkCompleteSession` modelled on `WorkspaceCreationSession` (transition function pattern in `workspace_creation.rs`). Phases:
`Idle → LoadingPreflight → ReviewingState{scenario} → AwaitingCommitMessage → Executing{action} → ConfirmingForceComplete → Done`.

**Why:** `ConfirmationContext` already has 4 archive-specific variants and is showing strain. The state machine handles branching scenarios (`SpecIncomplete` and `IssueOpen` need a force-confirm sub-phase; `Executing` loops back to `ReviewingState` after each action) cleanly with a pure transition function we can test.

**Alternative considered:** add 6+ new `ConfirmationContext` variants. Rejected — the existing context handler in `app_actions_confirm.rs` is already 100+ lines of branching, and the new flow has loops the single-shot dialog can't express.

### D6. Pre-filled inline commit message (vs. delegate to agent)

When the user picks "commit" from `EditsNoLink`, the TUI shows a single-line message input pre-filled from: branch name, first 1–2 dirty file paths, and (if linked) `Implement <change_id>` / `Fix #<N>`. Web shows the same in a text field. Agent escape hatch: cancel the dialog and use the agent session for richer messages.

**Why:** "Inline execution" was a locked decision. Forcing a session round-trip every commit defeats the flow. Pre-filled suggestions cover the common case; the cancel path covers the uncommon one.

**Risk acknowledged in Risks section.**

### D7. OpenSpec archive: rename only, no spec-sync (v1)

`crates/conduit-git/src/openspec_archive.rs::archive_change` does:
1. Verify `openspec/changes/<id>` exists, target `openspec/changes/archive/YYYY-MM-DD-<id>` does not.
2. `std::fs::rename`.
3. Return `{ new_path, warnings }`.

**Skips:** task-completeness gate (the dialog's `SpecComplete` scenario already enforced that), spec-sync (applying ADD/MODIFY/REMOVE/RENAME deltas to canonical specs).

The dialog surfaces a warning when delta spec files exist in the change directory: "Spec deltas not auto-synced — run /opsx:archive in agent if you have spec changes." After archive, the rename surfaces as a regular dirty change, so the dialog cycles back to `EditsNoLink` and the user commits the archive move.

**Why:** spec-sync correctness requires a real delta-spec parser (ADD/MODIFY/REMOVE/RENAME directives, capability spec format, conflict detection). Building that to v1 spec-sync quality is a multi-week project of its own. Splitting it lets us ship the 95% case and tackle sync as a follow-up.

**Alternative considered:** invoke `openspec` CLI subprocess. Rejected because the `openspec-archive-change` skill is *agent*-driven (it calls AskUserQuestion, prompts for confirmation, conditionally spawns sub-skills). Reusing only the non-interactive `openspec` CLI commands would still require us to implement the sync logic above the CLI.

### D8. Real `gh pr create` for the dialog path

`PrManager::create(path, opts) -> PrInfo` shells `gh pr create --base <base> --title <t> --body <b> --fill`. Optional `{ title, body }` overrides; `--fill` default. Existing `create_workspace_pr` handler that returns a prompt for the agent stays as-is for the agent flow; the new endpoint uses `PrManager::create` directly.

**Why:** Inline execution requires a real call. Keeping the prompt path means agents that prefer to write the body keep working.

### D9. Granular merge readiness gating

`MergeReadiness` enum has `Ready / Blocked / HasConflicts / Unknown`. The dialog enables the merge action only on `Ready`. `Blocked / HasConflicts / Unknown` show as disabled with the reason. An "merge with --admin" override exists but lives behind a secondary `ConfirmingForceComplete`-style sub-confirm so it's never one-click.

**Why:** Merging a PR with failing checks or unknown mergeability has consequences external to Conduit. The sub-confirm matches the gravity.

## Risks / Trade-offs

- **OpenSpec archive without spec-sync** → Dialog warns clearly when delta spec files exist; users with deltas run `/opsx:archive` in the agent. Spec-sync is a tracked v2 follow-up.
- **Inline commit-message UX is a behaviour change** → Pre-filled suggestions cover the common case. Cancel-and-use-agent is the escape hatch. Watch for user feedback; revisit if the agent path is preferred more than expected.
- **Merge with checks pending / unknown mergeability** → `MergeReadiness` gating + secondary sub-confirm for `--admin`. Never one-click.
- **`app.rs` has 144 archive references — large diff** → Migration concentrates the churn into PR 4 alongside the new state machine; reviewer sees one cohesive change rather than a half-migrated state.
- **Worktree-scan fallback inference is heuristic** → Spec inference uses `git log` on the actual base branch, which is precise for branches that touched `openspec/changes/`. Issue inference is a regex on the branch name only; if no `#N` appears it gracefully returns `None`. Inference is opt-in (only fires when the persisted column is NULL); legitimately link-less workspaces just stay link-less.
- **Backward-compatible keybinding alias** → A small parser-level alias maps `"ArchiveCurrentWorkspace"` strings in user configs to `Action::CompleteWorkspaceWork`. Dropped only after a future major version with a deprecation warning.
- **Endpoint deprecation** → `/archive/preflight` and the archive POST stay alive for one PR cycle (PRs 3–5) while the UIs switch over; PR 6 deletes them. The intermediate state is consistent.

## Migration Plan

The change ships in six PRs to keep each diff reviewable and CI-green:

1. **PR 1 — schema + persistence**: migration adds `active_change_id` / `active_issue_number`; `WorkspaceStore` INSERT/SELECT extended; `workspace_creation.rs` populates the columns. No UI behaviour change.
2. **PR 2 — pure libraries**: `crates/conduit-git/src/{actions.rs, openspec_archive.rs, work_complete.rs}` plus extensions to `specs.rs`, `issues/github.rs`, `pr.rs`. All unit-tested. No endpoint or UI changes yet.
3. **PR 3 — backend endpoints**: `/work-complete/preflight` and per-action endpoints, with integration tests. Old `/archive/*` endpoints stay alive but deprecated.
4. **PR 4 — TUI**: `WorkCompleteSession` state machine, dialog component, effects/events, `Action::CompleteWorkspaceWork` rename, keybinding alias. The 144 `app.rs` archive references migrate here. Removes legacy `ConfirmationContext::ArchiveWorkspace*`.
5. **PR 5 — Web**: `WorkCompleteDialog.tsx`, `Sidebar.tsx` rewires the `x` button, `useWorkspaceActions.tsx` updated.
6. **PR 6 — cleanup**: delete `/archive/preflight`, the archive `POST /workspaces/{id}` variant, `ArchivePreflightResponse` types, and any vestigial `ArchiveWorkspace*` symbols.

**Rollback:** PRs 1–3 are additive and can be reverted independently. PR 4 is the inflection point — once the TUI ships, rolling back means reverting PRs 4–5 (web depends on backend endpoints from PR 3, which can stay). PR 1's schema migration is forward-compatible (NULL columns); rolling back the migration is not required for downgrade.

## Open Questions

- **Spec-archive sync follow-up scope**: do we build a Rust delta-spec parser, or shell out to a non-interactive `openspec` CLI command if one exists? Decision deferred to the v2 spec-sync change.
- **Commit-message template hooks**: should we let users override the pre-filled suggestion via a config setting (e.g., a Tera template)? Out of scope for v1; revisit if multiple users ask.
- **Multi-spec branches**: a branch that touched two `openspec/changes/<id>` directories has ambiguous inference. v1 picks the most-recently-modified; if users hit this, switch to a picker phase. Tracked as a known edge case.
- **Issue close + PR merge ordering**: when both are needed (`IssueOpen` + open PR + green checks), should the dialog suggest closing the issue first or merging the PR first (which often auto-closes the issue via `Fixes #N`)? v1 suggests merge first, lets the issue auto-close, only offers explicit close if the issue is still open afterward. May revisit based on feel.
