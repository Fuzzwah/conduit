## 1. Database Migration

- [x] 1.1 Add a new migration step in `src/data/database.rs` that runs `ALTER TABLE repositories ADD COLUMN theme_name TEXT`
- [x] 1.2 Increment the `USER_VERSION` pragma in the migration sequence to reflect the schema change

## 2. Repository Model

- [x] 2.1 Add `pub theme_name: Option<String>` to the `Repository` struct in `src/data/models.rs`
- [x] 2.2 Update `Repository::from_row()` to read the `theme_name` column
- [x] 2.3 Add `update_repository_theme(conn, repo_id: Uuid, theme_name: Option<&str>) -> Result<()>` in `src/data/database.rs`

## 3. Session Cache

- [x] 3.1 Add `pub project_theme: Option<String>` field to `AgentSession` in `src/ui/session.rs`
- [x] 3.2 Populate `project_theme` when a workspace is associated with the session (look up repository from DB and read `theme_name`)

## 4. Theme Sync on Tab Switch

- [x] 4.1 Add `sync_theme_to_active_tab(&mut self)` to `App` in `src/ui/app.rs`: resolve project theme from active session, fall back to `config.theme_name`, then apply via the existing theme loading utilities
- [x] 4.2 Call `sync_theme_to_active_tab()` in `handle_tab_action()` in `src/ui/app/app_actions_tabs.rs` after the existing `sync_*` calls for `NextTab`, `PrevTab`, and `SwitchToTab`
- [x] 4.3 Call `sync_theme_to_active_tab()` during app startup after `restore_session_state()` so the correct project theme is applied on launch

## 5. Theme Picker Scope Toggle

- [x] 5.1 Add `ThemeScope` enum (`Global`, `Project`) and a `scope` field to the `ThemePicker` state struct in `src/ui/components/theme_picker.rs`
- [x] 5.2 Disable / hide the `Project` scope option when the active session has no repository context
- [x] 5.3 Bind a key (e.g., `Tab`) inside the theme picker to toggle between `Global` and `Project` scope; display the current scope in the picker header
- [x] 5.4 On confirm with `ThemeScope::Project`: call `update_repository_theme()` to persist the chosen theme name to the repository, update the active session's `project_theme` cache, and apply the theme
- [x] 5.5 Add a "Clear project theme" action in the picker (available when scope is `Project` and a project theme is set): calls `update_repository_theme(repo_id, None)`, clears session cache, and applies global theme
- [x] 5.6 Display a `(project)` badge in the theme picker title/header when `ThemeScope::Project` is active or a project override is currently applied

## 6. Verification

- [x] 6.1 Run `cargo fmt --check` and resolve any formatting issues
- [x] 6.2 Run `cargo clippy -- -D warnings` and fix all warnings
- [x] 6.3 Run `cargo test` and confirm all tests pass
- [x] 6.4 Manual test: assign different themes to two repositories, switch tabs, confirm theme changes on each switch
- [x] 6.5 Manual test: open a new-workspace tab (no repository), confirm global theme is used and "This project" scope is disabled in the picker
- [x] 6.6 Manual test: restart conduit, confirm project themes are restored for the active tab on launch
