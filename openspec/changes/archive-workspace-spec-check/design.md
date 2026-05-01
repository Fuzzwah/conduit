## Context

The archive preflight runs in a blocking closure inside `initiate_archive_workspace()` in `crates/conduit-ui/src/app.rs`. It already reads the workspace's `path` and `branch`, calls `get_branch_status_with_gh_option()`, and builds `warnings` / `info_items` vectors that feed into `ArchiveWorkspaceDialogPreflightResult`.

When a workspace is created from an OpenSpec change, the workspace's `name` is set to the change's `change_id` (e.g. `aged-reed`). When created from a Specify spec, the `name` is set to `spec_id`. This name-as-identifier pattern makes it straightforward to infer the linked spec from the workspace name alone — no additional DB column or stored association is needed.

## Goals / Non-Goals

**Goals:**
- Warn the user during archive preflight if there are incomplete tasks in a linked OpenSpec `tasks.md`
- Warn the user during archive preflight if there are incomplete tasks in a linked Specify `tasks.md`
- Reuse the existing `warnings` vec so the new information appears in the same confirmation dialog UI with zero frontend changes

**Non-Goals:**
- Storing the spec association in the database
- Checking any file other than `tasks.md` (proposal.md / design.md completeness is out of scope)
- Blocking the archive — warnings are advisory, not blocking
- Handling workspaces whose names were manually changed after creation

## Decisions

### Decision 1: Infer spec from workspace name, not a stored field

**Chosen:** Probe the filesystem for `openspec/changes/{workspace.name}/tasks.md` and `.specify/specs/{workspace.name}/tasks.md` relative to `repo.base_path`.

**Alternative considered:** Store the spec/change ID in the `workspaces` DB table at creation time.

**Rationale:** Adding a DB column requires a migration and schema change for a read-only lookup. The name convention is already enforced at workspace creation — it's reliable for workspaces created via the spec pickers. A filesystem probe is a cheap, zero-migration approach that covers the cases that matter.

### Decision 2: Surface as a warning (not blocking, not info)

**Chosen:** Add to the `warnings` vec with a message like `"OpenSpec change has N incomplete tasks"`.

**Alternative considered:** Info item (green tick) or a new severity level.

**Rationale:** Incomplete tasks represent genuinely unfinished work — the same severity bucket as "branch not merged". The existing warning display is already prominent in the confirmation dialog. An info item would be too easy to ignore.

### Decision 3: Parse `tasks.md` in-process with a simple line scan

**Chosen:** Read the file and count lines matching `- [ ]`.

**Alternative considered:** Parse the full Markdown AST or call an external tool.

**Rationale:** The `- [ ]` GFM checkbox syntax is the only task format used by both OpenSpec and Specify. A line-prefix scan is O(n) and has no dependencies. False positives (e.g., `- [ ]` in a code block) are acceptable edge cases.

## Risks / Trade-offs

- **Name mismatch**: If a user renames a workspace after creation, the spec probe will silently find nothing. This is acceptable — the feature is best-effort.  
  → Mitigation: None required; the check degrades gracefully to "no spec found".

- **Large tasks.md**: Scanning a large file is still fast (pure I/O, no parsing overhead).  
  → No mitigation needed.

- **Both OpenSpec and Specify found**: Unlikely, but if both paths exist, both warnings are surfaced independently.  
  → This is the correct behavior.
