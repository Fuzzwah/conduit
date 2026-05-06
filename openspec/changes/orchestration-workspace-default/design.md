## Context

Conduit already has a global `orchestration.enabled_by_default` in `conduit.toml` (added in the `model-orchestration` change). Sessions read this flag at creation time (`app.rs:5666-5672`). The `Workspace` and `Repository` (project) structs already carry per-level overrides for MCP disabled servers and theme using the same `Option<T>` / "inherit from parent" pattern that this feature needs. SQLite migrations follow a well-established idempotent ALTER TABLE pattern (migrations 18-20 in `database.rs`).

The sidebar sidebar key handling runs through `InputMode::SidebarNavigation` → `KeyContext::Sidebar`, and sidebar actions are dispatched in `app_actions_sidebar.rs`.

## Goals / Non-Goals

**Goals:**
- `Workspace.orchestration_enabled: Option<bool>` — per-workspace override (`None` = inherit)
- `Repository.orchestration_enabled: Option<bool>` — per-project override (`None` = inherit)
- Resolution order at session creation: workspace override → project override → global config default
- `M-S-o` in session context → existing `ShowOrchestrationSelector` (toggles the running session)
- `M-S-o` in sidebar context → `ToggleOrchestrationDefault` (cycles workspace/project override: `None` → `true` → `false` → `None`)

**Non-Goals:**
- Visual indicator in the sidebar for the orchestration state (keep sidebar lean; the cycle action gives immediate feedback via a chat/status message)
- Per-workspace-session (a session that already exists doesn't retroactively change when the workspace default changes)
- Web UI support

## Decisions

### 1. `Option<bool>` stored as nullable INTEGER in SQLite

SQLite stores booleans as 0/1. `NULL` means "inherit from parent level". This is consistent with the existing `archived_at` nullable pattern and avoids the JSON overhead of the `mcp_disabled_servers` approach.

**Alternative considered:** Store as JSON text like `mcp_disabled_servers`. Rejected — a single boolean doesn't benefit from JSON serialization, and nullable INTEGER is simpler to query and migrate.

### 2. Three-state cycle for sidebar toggle: None → true → false → None

`None` (inherit) is the default state. Cycling through all three states allows users to explicitly force-on, force-off, or revert to inherit — all without opening a modal.

**Alternative:** Binary toggle (None/true, where None means off). Rejected — users lose the ability to explicitly force-off orchestration for a project that inherits a global `enabled_by_default = true`.

### 3. New `Action::ToggleOrchestrationDefault` rather than reusing `ShowOrchestrationSelector`

The sidebar action persists to the database (workspace/project), while the session action only affects the running session. Different semantics warrant a different action variant.

### 4. Full-row update via existing `WorkspaceStore::update()` and `RepositoryStore::update()`

Both DAOs already have a full-row `update()` method. Adding `orchestration_enabled` to the struct and those calls is lower-friction than adding a dedicated single-field updater. Consistent with how `theme_name` and `mcp_disabled_servers` are handled.

## Risks / Trade-offs

- **Migration on old databases** → Mitigation: idempotent `pragma_table_info` check before `ALTER TABLE`, same as migrations 18-20.
- **Sidebar action on NewWorkspace node** → Mitigation: `ToggleOrchestrationDefault` handler checks node type; silently no-ops on non-workspace/non-project nodes.
- **Stale session state** — toggling the project/workspace default after a session is already running has no effect on that session → Acceptable and intentional; documented in the chat message.

## Migration Plan

Migration 21: add `orchestration_enabled INTEGER` (nullable, no default) to both `workspaces` and `repositories` tables. Idempotent check. No data backfill needed — `NULL` correctly means "inherit from global config".
