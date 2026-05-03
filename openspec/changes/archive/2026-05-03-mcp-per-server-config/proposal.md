## Why

The current MCP configuration is a single project-level on/off toggle, which is too coarse — users need to enable specific servers only in the workspaces where they're useful (e.g. enabling `context7` only in a workspace doing doc lookups) without affecting the whole project. The feature also needs to extend to workspace scope so per-worktree overrides are possible.

## What Changes

- **BREAKING**: Remove `mcp_enabled: bool` from `repositories` table; replace with `mcp_disabled_servers TEXT NOT NULL DEFAULT '[]'` (JSON array of server names)
- Add `mcp_disabled_servers TEXT NOT NULL DEFAULT '[]'` column to `workspaces` table (new capability)
- Replace the global MCP on/off dialog with a per-server list dialog that covers both Project and Workspace scope
- Extend the `M-S-m` keybinding from sidebar-only to also fire from `Chat` and `Scrolling` key contexts
- Update MCP enforcement (Codex and Claude paths) to use per-server disabled lists instead of a global bool
- Rename `Action::ManageProjectMcp` → `Action::ManageMcp` and `InputMode::ProjectMcp` → `InputMode::ManageMcp`

## Capabilities

### New Capabilities

- `mcp-per-server-config`: Per-server MCP enable/disable at both project and workspace scope, with workspace settings fully overriding project settings when present

### Modified Capabilities

<!-- none — no existing specs cover MCP configuration -->

## Impact

- **DB schema**: migration adds/removes columns on `repositories` and `workspaces` tables
- **`conduit-data`**: `Repository` and `Workspace` models; both DAOs (insert/update/select)
- **`conduit-types`**: `Action`, `InputMode` enum variants
- **`conduit-config`**: `default_keys.rs` keybindings (add Chat/Scrolling bindings)
- **`conduit-ui`**: `project_mcp_dialog` component (replaced); `app_actions_dialog.rs`; `app_actions_confirm.rs`; `app.rs` (MCP detection helpers + enforcement paths for Codex ~line 9621 and Claude ~line 8449)
