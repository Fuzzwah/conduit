## 1. Database Migration

- [x] 1.1 Add migration (next after #17): `ALTER TABLE repositories DROP COLUMN mcp_enabled`, `ALTER TABLE repositories ADD COLUMN mcp_disabled_servers TEXT NOT NULL DEFAULT '[]'`
- [x] 1.2 Add to same migration: `ALTER TABLE workspaces ADD COLUMN mcp_disabled_servers TEXT` (nullable — NULL means inherit project)
- [x] 1.3 Verify migration runs cleanly against a fresh DB and an existing DB

## 2. Data Models and DAOs

- [x] 2.1 Update `Repository` struct in `conduit-data/src/models.rs`: remove `mcp_enabled: bool`, add `mcp_disabled_servers: Vec<String>`
- [x] 2.2 Update `Workspace` struct: add `mcp_disabled_servers: Option<Vec<String>>`
- [x] 2.3 Update `repository` DAO INSERT, UPDATE, SELECT to serialize/deserialize `mcp_disabled_servers` as JSON
- [x] 2.4 Update `workspace` DAO INSERT, UPDATE, SELECT to handle nullable `mcp_disabled_servers` JSON column
- [x] 2.5 Fix all compile errors from removed `mcp_enabled` field (search across all crates)

## 3. Types and Keybindings

- [x] 3.1 Rename `Action::ManageProjectMcp` → `Action::ManageMcp` in `conduit-types/src/action.rs`; update description string and `opens_dialog()`/palette entry
- [x] 3.2 Rename `InputMode::ProjectMcp` → `InputMode::ManageMcp` in `conduit-types/src/input_mode.rs`; update `KeyContext::from_input_mode()` mapping
- [x] 3.3 In `conduit-config/src/default_keys.rs`: rename sidebar binding to use `Action::ManageMcp`; add `bind(chat, "M-S-m", Action::ManageMcp)` and `bind(scrolling, "M-S-m", Action::ManageMcp)`
- [x] 3.4 Fix all compile errors from renamed Action and InputMode variants

## 4. MCP Server Detection Refactor

- [x] 4.1 Refactor `detect_codex_project_mcp_servers()` and `detect_generic_project_mcp_servers()` in `app.rs` to return `Vec<(String, McpSource)>` where `McpSource` is an enum `Codex | McpJson`
- [x] 4.2 Create a `detect_all_mcp_servers(base_path: &Path) -> Vec<(String, McpSource)>` helper combining both sources
- [x] 4.3 Update `project_mcp_summary()` to use the new helper (keep summary string generation for any remaining display use)

## 5. Dialog Component Replacement

- [x] 5.1 Replace `ProjectMcpDialogState` with `McpDialogState` in `project_mcp_dialog.rs` (or rename the file to `mcp_dialog.rs`); add fields: `scope: McpScope`, `workspace_id`, `workspace_name`, `project_servers: Vec<McpServer>`, `workspace_servers: Vec<McpServer>`
- [x] 5.2 Add `McpScope` enum (`Project`, `Workspace`) and `McpServer` struct (`name`, `source`, `enabled`) to the dialog module
- [x] 5.3 Implement tab rendering: Project/Workspace tabs at top of dialog, highlighted based on `scope`
- [x] 5.4 Implement server list rendering: scrollable list with `[✓]`/`[✗]` prefix, server name, and source column
- [x] 5.5 Implement keyboard navigation: Left/Right (or Tab) to switch scope tabs; Up/Down to move cursor; Space/Enter to toggle selected server's enabled state; `s` or navigating to Save to confirm; Esc to cancel
- [x] 5.6 When switching to Workspace tab and `workspace_servers` was pre-populated from project (first open), ensure visual indicator shows "using project defaults" or similar

## 6. Dialog Open Logic

- [x] 6.1 Update `app_actions_dialog.rs` handler for `Action::ManageMcp`: detect selected sidebar node type (`NodeType::Repository` → scope=Project, `NodeType::Workspace` → scope=Workspace)
- [x] 6.2 For Chat/Scrolling context trigger: derive `workspace_id` from the active tab's workspace, set scope=Workspace
- [x] 6.3 Load both server lists on dialog open: call `detect_all_mcp_servers()` for the project path, then load `repo.mcp_disabled_servers` and `workspace.mcp_disabled_servers` from DB to set initial `enabled` states
- [x] 6.4 For Workspace tab when workspace has no saved config (`None`): pre-populate `workspace_servers` from the project list as a visual starting point

## 7. Dialog Save Logic

- [x] 7.1 Update `app_actions_confirm.rs` save handler for `InputMode::ManageMcp`: branch on `scope`
- [x] 7.2 Project scope save: collect disabled server names from `project_servers`, serialize to JSON, update `repo.mcp_disabled_servers`, call `dao.update(&repo)`
- [x] 7.3 Workspace scope save: collect disabled server names from `workspace_servers`, serialize to JSON, update `workspace.mcp_disabled_servers`, call workspace DAO update
- [x] 7.4 Show appropriate success message: "Project MCP updated" or "Workspace MCP updated"

## 8. Enforcement Updates

- [x] 8.1 Add `resolve_disabled_servers(repo: &Repository, workspace: Option<&Workspace>) -> Vec<String>` helper in `app.rs`
- [x] 8.2 Update Codex enforcement path (~line 9621): replace `if !project_mcp_enabled` block with `resolve_disabled_servers()` call and per-server override loop
- [x] 8.3 Add `extract_mcp_server_name(tool: &str) -> Option<&str>` helper: parses `mcp__<server>__<tool>` → `<server>` (handle `mcp:` and `mcp/` prefixes too)
- [x] 8.4 Update Claude enforcement path (~line 8449): replace global `mcp_enabled` check with per-server check using `extract_mcp_server_name()` and `resolve_disabled_servers()`
- [x] 8.5 Update denial message to name the specific server: `"MCP server '{server}' is disabled for this workspace."`

## 9. CI Verification

- [x] 9.1 Run `cargo fmt --check`
- [x] 9.2 Run `cargo clippy --workspace --all-targets -- -D warnings`
- [x] 9.3 Run `cargo test --workspace`
- [x] 9.4 Manual smoke test: open dialog from sidebar (project node), sidebar (workspace node), and active session tab; toggle a server; verify enforcement blocks/allows the server in a Claude session
