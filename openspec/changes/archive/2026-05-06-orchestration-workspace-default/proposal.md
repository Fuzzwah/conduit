## Why

The global `orchestration.enabled_by_default` config applies to every session, but users may want orchestration on for some projects (large codebases where context savings matter) and off for others (quick scripts where sub-agent overhead isn't worth it). There is also no keyboard shortcut to toggle orchestration — users must navigate the command palette every time.

## What Changes

- Add `orchestration_enabled: bool` column to both the `workspaces` and `repositories` (projects) SQLite tables via a new migration, defaulting to `NULL` (meaning "inherit from global config").
- Add `orchestration_enabled: Option<bool>` field to the `Workspace` and `Repository` Rust structs and their DAO read/write paths.
- When a new Claude session is created for a workspace, resolve orchestration default: workspace override → project (repository) override → global config default.
- Add `M-S-o` keybinding in the session context bound to `Action::ShowOrchestrationSelector` (toggles the current session).
- Add `M-S-o` keybinding in the sidebar context; when a workspace or project node is focused, pressing it cycles the node's `orchestration_enabled` override (None → true → false → None) with a brief status message.
- New sidebar action `Action::ToggleOrchestrationDefault` handles the sidebar-level toggle.

## Capabilities

### New Capabilities

- `orchestration-workspace-default`: Per-workspace and per-project orchestration default that new sessions inherit, with a sidebar hotkey to toggle it.

### Modified Capabilities

<!-- none -->

## Impact

- `crates/conduit-data/src/database.rs` — new migration adding `orchestration_enabled` column to `workspaces` and `repositories` tables.
- `crates/conduit-data/src/models.rs` — `orchestration_enabled: Option<bool>` field on `Workspace` and `Repository`.
- `crates/conduit-data/src/workspace_dao.rs` / `repository_dao.rs` — read/write the new field; add `set_orchestration_enabled()` update method.
- `crates/conduit-ui/src/app.rs` — resolve orchestration default from workspace/project during session creation; handle new sidebar action.
- `crates/conduit-types/src/action.rs` — add `Action::ToggleOrchestrationDefault` variant.
- `crates/conduit-config/src/default_keys.rs` — bind `M-S-o` in session and sidebar contexts.
- `crates/conduit-ui/src/app/app_actions_sidebar.rs` — handle `ToggleOrchestrationDefault` for workspace and project nodes.
