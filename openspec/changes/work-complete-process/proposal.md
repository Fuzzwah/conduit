## Why

Today's "archive workspace" flow runs a thin git preflight (dirty / ahead / merged) and a single confirm dialog before soft-deleting the workspace. It has no idea whether the user actually finished the work it was meant for: there is no awareness of the linked OpenSpec change (only a name-based heuristic warning), no awareness of the linked GitHub issue, and no way to drive the real follow-up actions (commit, push, open PR, merge, close issue, archive the spec) without leaving Conduit and shelling out manually. Users routinely hit `Alt+Shift+X` and end up with stale branches, never-merged PRs, or specs that were never archived.

This change replaces archive with a context-aware "Work Complete" flow that detects the workspace's situation, classifies it into one of six scenarios, and offers (and executes inline) the right follow-up actions. The terminal step is still archiving the workspace — but only after the work is genuinely done.

## What Changes

- **BREAKING** Replace the archive workspace flow entirely. `Alt+Shift+X` and the sidebar `x` button now invoke "Work Complete" instead of "Archive". The old `Action::ArchiveCurrentWorkspace` is renamed to `Action::CompleteWorkspaceWork`; a parser alias keeps user-customised keybinding configs working.
- Add a context-aware preflight that introspects the workspace's git state, PR status, linked OpenSpec change progress, and linked GitHub issue status, then classifies the situation (`CleanReady` / `EditsNoLink` / `SpecComplete` / `SpecIncomplete` / `IssueOpen` / `IssueClosed`) and emits a list of suggested actions.
- Persist spec and issue linkage on the workspace at creation time (new `active_change_id` and `active_issue_number` columns), with a worktree-scan fallback for legacy workspaces that have NULL columns.
- Execute follow-up actions inline from the dialog: commit (with pre-filled message), push, `gh pr create`, `gh pr merge`, `gh issue close`, OpenSpec change archive (filesystem rename only, no spec-sync in v1), and finally workspace archive.
- Build a multi-step state machine in the TUI (mirroring `workspace_creation.rs`) and a matching `WorkCompleteDialog.tsx` in the web UI, both backed by one shared introspection endpoint.
- **BREAKING** The existing `/workspaces/{id}/archive/preflight` and archive endpoints are deprecated and removed in a follow-up cleanup PR; new endpoints live under `/workspaces/{id}/work-complete/...`.
- Replace the agent-prompt-only PR creation path with a real `gh pr create` invocation when the dialog drives it; the existing prompt path stays available for the agent flow.

## Capabilities

### New Capabilities

- `work-complete-process`: end-to-end "Work Complete" flow — introspection endpoint, scenario classifier, suggested-action sequencing, TUI state machine, web dialog, and the inline action endpoints (commit / push / pr create / pr merge / issue close / spec archive / workspace archive).
- `workspace-context-links`: persisted `active_change_id` and `active_issue_number` on the Workspace model; populated at creation from the picker selections; worktree-scan fallback inferring the values for legacy workspaces and writing them back.

### Modified Capabilities

- `archive-workspace-spec-check`: REMOVED. The two requirements (incomplete-OpenSpec warning, incomplete-Specify warning) are subsumed by the richer Work Complete flow, which does not just warn but classifies the scenario and lists the actual remaining task lines. The legacy archive preflight that hosted those requirements is being deleted.
- `workspace-creation-prelude`: MODIFIED — when the user picks an issue and/or a spec during creation, the workspace record is created with `active_issue_number` and/or `active_change_id` populated. Existing prelude phasing semantics are unchanged.

## Impact

- **Code**:
  - `crates/conduit-data/src/{models.rs,database.rs,workspace.rs}` — schema migration + new columns + INSERT/SELECT extensions + `update_active_links` setter.
  - `crates/conduit-git/src/{specs.rs,issues/github.rs,pr.rs,status.rs}` — extend `fetch_open_specs` to expose remaining task lines, add `infer_active_change` and `infer_active_issue`, add `close_issue`, add `PrManager::create` (real `gh pr create`) and `PrManager::merge`.
  - `crates/conduit-git/src/{actions.rs,openspec_archive.rs,work_complete.rs}` — new pure libraries: git action wrappers, OpenSpec archive (rename only, no sync), scenario classifier.
  - `crates/conduit-web/src/handlers/workspaces.rs` and `routes/api.rs` — new `/work-complete/preflight` and per-action endpoints; deprecate `/archive/preflight`.
  - `crates/conduit-ui/src/{work_complete.rs,components/work_complete_dialog.rs,effect.rs,action.rs,app.rs}` — new state machine + dialog component + effects + action rename. `app.rs` has 144 references to the legacy archive types; this is the largest single touch point.
  - `crates/conduit-config/src/default_keys.rs` — re-bind `M-S-x` to the new action.
  - `crates/conduit-web/web/src/components/WorkCompleteDialog.tsx`, `Sidebar.tsx`, `hooks/useWorkspaceActions.tsx` — new web dialog + replacement of archive button.
- **APIs**:
  - New: `GET /workspaces/{id}/work-complete/preflight`, `POST /workspaces/{id}/work-complete/{commit,push,pr,pr/merge,issue/close,spec/archive,archive}`.
  - Deprecated → removed in cleanup PR: `GET /workspaces/{id}/archive/preflight`, `POST /workspaces/{id}` archive variant.
- **Database**: schema migration adds two nullable columns (`active_change_id`, `active_issue_number`) to `workspaces`. Idempotent, guarded by `pragma_table_info` in the existing `apply_migrations` pattern.
- **Dependencies**: no new crates. Continues using `gh` CLI (no octocrab).
- **User-visible behaviour**: keybinding stays `M-S-x` but its name and dialog change. Users with custom keybinding configs referencing `ArchiveCurrentWorkspace` continue to work via parser alias. The "quick archive a clean workspace" experience is preserved as the `CleanReady` path inside the new flow.
- **OpenSpec workflow**: `opsx:archive`-equivalent move is performed, but spec-sync (delta application) is intentionally out of scope for v1; users with delta specs see a dialog warning recommending they run `/opsx:archive` in their agent session for sync.
