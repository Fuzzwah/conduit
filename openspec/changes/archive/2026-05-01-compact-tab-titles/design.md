## Context

`AgentSession::tab_name()` in `src/ui/session.rs` builds the tab label from two optional fields: `project_name` (e.g., `conduit`) and `workspace_name` (the randomized name, e.g., `old-rose`), producing `conduit (old-rose)`. The workspace's git branch (e.g., `fuz/old-rose`) is stored in the `Workspace` struct (`workspace.branch`) but is not currently surfaced in the session.

The branch name is more meaningful to users than the workspace name, and a shorter project label keeps the tab bar compact when many tabs are open.

## Goals / Non-Goals

**Goals:**
- Truncate project name to 10 chars (append `…` when truncated)
- Replace the workspace name with the trailing segment of the branch (after the last `/`)
- Carry the branch name through handoff flows so renamed sessions remain correct

**Non-Goals:**
- Live-updating the tab title as the user checks out a different branch (the status bar already does this)
- Changing tab title format for file-viewer tabs
- Altering any other UI element

## Decisions

**Add `branch_name: Option<String>` to `AgentSession`** rather than reading from the existing `status_bar.branch_name()`.

The status bar's branch is populated asynchronously by the git tracker after a polling delay. At session creation the field may be empty, leading to a flash of no-branch in the tab title. The workspace's `branch` column in the DB is the canonical, immediately-available value for what branch conduit created the workspace on.

**Add `branch_name: Option<String>` to `PendingHandoffRequest`** so the branch label is preserved when a session is handed off to a different agent type.

**Truncation via `chars().count()`** (not byte length) to handle non-ASCII project names correctly. The `…` ellipsis is a single Unicode character, keeping total display width predictable.

## Risks / Trade-offs

- [Branch name drift] If the user manually renames their branch outside conduit, the tab will show the original branch name → Acceptable; the status bar already reflects the live branch. Tab title is a workspace identifier, not a live git probe.
- [Longer branch suffixes] Very long branch names after the `/` (e.g., `fix-very-descriptive-long-name`) will still be shown in full in the brackets → Tab bar will expand, but this matches current workspace-name behaviour and is bounded by normal branch naming conventions.

## Migration Plan

No data migration required. The `branch` column already exists in the `workspaces` table. The change is purely in how the TUI reads and displays that data. No rollback complexity.

## Open Questions

None.
