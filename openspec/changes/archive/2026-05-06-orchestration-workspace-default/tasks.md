## 1. Data Model

- [x] 1.1 Add `orchestration_enabled: Option<bool>` field to `Workspace` struct in `crates/conduit-data/src/models.rs`
- [x] 1.2 Add `orchestration_enabled: Option<bool>` field to `Repository` struct in `crates/conduit-data/src/models.rs`

## 2. Database Migration

- [x] 2.1 Add migration 21 in `crates/conduit-data/src/database.rs`: idempotent `ALTER TABLE workspaces ADD COLUMN orchestration_enabled INTEGER` (nullable, no default)
- [x] 2.2 Add migration 21 (same block): idempotent `ALTER TABLE repositories ADD COLUMN orchestration_enabled INTEGER` (nullable, no default)

## 3. DAO Read / Write

- [x] 3.1 In `crates/conduit-data/src/workspace.rs`: include `orchestration_enabled` in INSERT and `update()` SQL; read it back from rows (store as `Option<bool>` via `Option<i64>` → map 0/1)
- [x] 3.2 In `crates/conduit-data/src/repository.rs`: same — include `orchestration_enabled` in INSERT and `update()` SQL; read it back from rows

## 4. Action Variant

- [x] 4.1 Add `Action::ToggleOrchestrationDefault` variant to `crates/conduit-types/src/action.rs`; add display name, `action_to_name` / `parse_action` / known-actions entries in `crates/conduit-config/src/settings.rs`

## 5. Keybindings

- [x] 5.1 In `crates/conduit-config/src/default_keys.rs`: bind `M-o` → `Action::ShowOrchestrationSelector` in the session/global context (same context as other session selectors)
- [x] 5.2 In `crates/conduit-config/src/default_keys.rs`: bind `M-o` → `Action::ToggleOrchestrationDefault` in the sidebar context (`KeyContext::Sidebar`)

## 6. Sidebar Action Handler

- [x] 6.1 In `crates/conduit-ui/src/app/app_actions_sidebar.rs`: handle `Action::ToggleOrchestrationDefault` — get the focused sidebar node, determine if it's a workspace or project node, load the current value from the DAO, cycle `None → true → false → None`, save via `update()`, show a chat/notification message describing the new state

## 7. Session Initialization

- [x] 7.1 In `crates/conduit-ui/src/app.rs`, in the session-creation path (around line 5666): after reading `orchestration_default` from global config, also load the workspace and project records from DAO and resolve the override chain (workspace → project → global); set `session.orchestration_enabled` to the resolved value

## 8. Verification

- [x] 8.1 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and confirm all pass
- [x] 8.2 Manual test: set workspace orchestration default to `true` via `M-o` in sidebar; create a new Claude session for that workspace; confirm status bar shows correct orchestration state
- [x] 8.3 Manual test: set project default to `false`, workspace to `None`; confirm new session inherits `false`
- [x] 8.4 Manual test: press `M-o` in a Claude session tab; confirm orchestration selector modal opens and `orch:on/off` updates
