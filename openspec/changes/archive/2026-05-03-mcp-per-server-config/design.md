## Context

MCP (Model Context Protocol) servers are configured in project directories via `.mcp.json` or `.codex/config.toml`. The existing system stores a single `mcp_enabled: bool` on `Repository`, toggled via a project-level dialog (`M-S-m` in sidebar). Enforcement disables all servers when the flag is false. No workspace-level overrides exist. The feature is new enough that no real data exists in the field — clean migration is safe.

Key paths:
- Detection: `app.rs` `detect_codex_project_mcp_servers()` / `detect_generic_project_mcp_servers()` → `Vec<String>` of server names
- Codex enforcement: `app.rs` ~line 9621 — builds `session_config_overrides` disabling all servers
- Claude enforcement: `app.rs` ~line 8449 — denies `mcp__*` tool calls when `mcp_enabled=false`
- Dialog: `components/project_mcp_dialog.rs`, opened from `app_actions_dialog.rs:255`, saved in `app_actions_confirm.rs:349`

## Goals / Non-Goals

**Goals:**
- Replace global on/off with per-server enable/disable
- Add workspace scope alongside project scope (workspace fully overrides project when set)
- Single unified dialog for both scopes with a Project/Workspace tab
- Hotkey available from sidebar (both node types) and from active session tab (Chat/Scrolling contexts)
- Clean DB migration — no backward compat needed

**Non-Goals:**
- UI for creating or editing MCP server definitions (just toggling existing ones)
- Sync/merge logic between project and workspace (workspace overrides entirely)
- Per-agent-type MCP control (same disabled list applies to all agents)

## Decisions

### D1: Storage format — JSON column vs join table

**Decision**: JSON columns (`TEXT DEFAULT '[]'`) on both `repositories` and `workspaces`.

**Rationale**: The disabled server list is a small, unordered set of strings with no attributes beyond membership. A join table adds schema complexity and query joins for no analytical benefit. JSON is simpler, survives schema evolution (add attributes later if needed), and matches how similar config blobs are stored elsewhere in conduit.

**Alternative considered**: `mcp_server_overrides(repo_id/workspace_id, server_name, enabled BOOL)` join table — rejected as over-engineered.

### D2: Workspace override semantics — full override vs additive

**Decision**: Workspace list fully replaces project list when present (`NULL` = inherit project).

**Rationale**: Allows a workspace to re-enable servers the project disables (e.g. enable `context7` in a docs workspace even if the project disables it). Additive-only would prevent this. `NULL` vs `[]` distinguishes "never configured" (inherit) from "explicitly set to none disabled".

**Alternative considered**: Additive (union of disabled) — rejected because it prevents workspace-level re-enabling.

### D3: Scope selection UX — separate actions vs single dialog with tabs

**Decision**: Single `Action::ManageMcp` with a Project/Workspace tab in the dialog. Default tab determined by trigger context.

**Rationale**: One action is simpler to bind and document. The user can switch tabs within the dialog, which is more discoverable than two separate hotkeys. Default tab based on context (sidebar node type or active tab) handles the common cases automatically.

### D4: Server list source for project scope

**Decision**: Scan the default/main worktree path for `.mcp.json` and `.codex/config.toml`.

**Rationale**: The project-level config lives in the repo root. The main worktree is the canonical location. All worktrees share the same git history so the server list is consistent regardless.

### D5: Rename Action and InputMode

**Decision**: Rename `ManageProjectMcp` → `ManageMcp`, `InputMode::ProjectMcp` → `InputMode::ManageMcp`.

**Rationale**: The scope is now broader than "project". Clean rename is safe since no external API depends on these variants.

## Data Model

```
repositories
  - REMOVE: mcp_enabled INTEGER NOT NULL DEFAULT 1
  - ADD:    mcp_disabled_servers TEXT NOT NULL DEFAULT '[]'

workspaces
  - ADD:    mcp_disabled_servers TEXT NOT NULL DEFAULT '[]'
            (NULL = inherit project; '[]' = all enabled; '["a","b"]' = a+b disabled)
```

## Dialog State

```rust
pub enum McpScope { Project, Workspace }

pub struct McpServer {
    pub name: String,
    pub source: McpSource,   // Codex | McpJson
    pub enabled: bool,
}

pub struct McpDialogState {
    pub visible: bool,
    pub scope: McpScope,
    pub repo_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub project_name: String,
    pub workspace_name: Option<String>,
    pub project_servers: Vec<McpServer>,    // loaded from project scope
    pub workspace_servers: Vec<McpServer>,  // loaded from workspace scope (or cloned from project if None)
    pub selected: usize,                    // cursor in server list
}
```

When switching tabs, the dialog re-renders from the appropriate server list without re-querying the DB — both lists are loaded on open.

## Keybinding Changes

```
// default_keys.rs additions:
bind(chat,      "M-S-m", Action::ManageMcp)
bind(scrolling, "M-S-m", Action::ManageMcp)
// existing (rename only):
bind(sidebar,   "M-S-m", Action::ManageMcp)
```

## Enforcement Changes

```rust
// Resolve disabled list for the current workspace
fn resolve_disabled_servers(repo: &Repository, workspace: Option<&Workspace>) -> Vec<String> {
    match workspace.and_then(|w| w.mcp_disabled_servers.as_ref()) {
        Some(ws_list) => ws_list.clone(),   // workspace overrides
        None => repo.mcp_disabled_servers.clone(),  // inherit project
    }
}

// Codex path: per-server disable via session_config_overrides
for server in disabled_servers {
    overrides.insert(format!("mcp_servers.{}.enabled", server), "false".to_string());
}

// Claude path: check specific server name extracted from tool name
// mcp__<server>__<tool> → server = second segment
fn extract_mcp_server_name(tool: &str) -> Option<&str> { ... }
if disabled_servers.contains(server_name) { /* deny */ }
```

## Risks / Trade-offs

- **Empty server list**: If `.mcp.json` and `.codex/config.toml` both absent, dialog shows "No MCP servers detected" — user cannot configure what doesn't exist. No mitigation needed; this is correct behavior.
- **Server list drift**: If a server is removed from config after being disabled in the DB, the stale entry in `mcp_disabled_servers` is harmless (it refers to a server that no longer runs). No cleanup needed.
- **`NULL` vs `'[]'` on workspaces**: Must ensure DAO reads `NULL` correctly as `None` in Rust and serializes new saves as `'[]'` not `NULL`. SQL `DEFAULT '[]'` only applies to new rows; existing workspace rows get `NULL` via ALTER TABLE — this is intentional (inherit project).
- **Rename breakage**: `ManageProjectMcp` / `InputMode::ProjectMcp` used in keybindings editor display and potentially user `keybindings.json` files. Users with custom bindings to the old action name will silently lose them. Risk is low (new feature, few users).

## Migration Plan

Single migration (next migration number after 17):
1. `ALTER TABLE repositories DROP COLUMN mcp_enabled` — SQLite 3.35+ supports this
2. `ALTER TABLE repositories ADD COLUMN mcp_disabled_servers TEXT NOT NULL DEFAULT '[]'`
3. `ALTER TABLE workspaces ADD COLUMN mcp_disabled_servers TEXT`  ← nullable, NULL = inherit

No rollback strategy needed (no prod data at risk).

## Open Questions

- None — all design decisions resolved during exploration.
