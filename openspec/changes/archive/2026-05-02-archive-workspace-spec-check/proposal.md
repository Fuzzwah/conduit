## Why

When a user archives a workspace that was created for an OpenSpec or Specify spec, the archive preflight only checks git status — it has no awareness of whether spec-driven work is still in progress. A workspace with incomplete tasks in `tasks.md` could be silently archived, leaving work orphaned with no warning.

## What Changes

- The archive preflight closure in `initiate_archive_workspace()` gains two additional checks:
  - **OpenSpec check**: if `openspec/changes/{workspace.name}/tasks.md` exists, count incomplete items (lines matching `- [ ]`); surface as a warning if any remain.
  - **Specify check**: if `.specify/specs/{workspace.name}/tasks.md` exists, do the same.
- Incomplete-tasks warnings appear alongside the existing git-status warnings in the archive confirmation dialog.
- No new UI, no new settings, no schema changes.

## Capabilities

### New Capabilities

- `archive-workspace-spec-check`: Detect linked OpenSpec/Specify spec files during archive preflight and warn when `tasks.md` contains incomplete items.

### Modified Capabilities

*(none — existing archive preflight behavior is extended, not changed at the requirement level)*

## Impact

- `crates/conduit-ui/src/app.rs` — `initiate_archive_workspace()` preflight closure
- `crates/conduit-ui/src/events.rs` — `ArchiveWorkspaceDialogPreflightResult` (may add an `info_items` entry or `warnings` entry)
- No API, database, or frontend changes required
