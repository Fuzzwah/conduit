## 1. Database Schema

- [x] 1.1 Add `agent_pid INTEGER` and `agent_pid_start_time INTEGER` column definitions to `SCHEMA` in `crates/conduit-data/src/database.rs`
- [x] 1.2 Add migration guards in `apply_migrations()` to `ALTER TABLE session_tabs ADD COLUMN agent_pid INTEGER` and `ALTER TABLE session_tabs ADD COLUMN agent_pid_start_time INTEGER` for existing databases

## 2. Data Layer

- [x] 2.1 Add `agent_pid: Option<u32>` and `agent_pid_start_time: Option<u64>` fields to the `SessionTab` struct in `crates/conduit-data/src/models.rs`
- [x] 2.2 Update `SessionTab::new` and any `Default` / constructor impls to default both fields to `None`
- [x] 2.3 Add `set_agent_pid(id: Uuid, pid: u32, start_time: Option<u64>)` method to `SessionTabStore` in `crates/conduit-data/src/session_tab.rs`
- [x] 2.4 Add `clear_agent_pid(id: Uuid)` method to `SessionTabStore`
- [x] 2.5 Update `get_by_id` and `get_all` query row-mapping in `SessionTabStore` to read `agent_pid` and `agent_pid_start_time` columns

## 3. TUI Integration

- [x] 3.1 In `crates/conduit-ui/src/app.rs`, after setting `session.agent_pid = Some(pid)` (~line 7778), call `session_tab_store.set_agent_pid(session.id, pid, pid_start_time)` — log and ignore errors
- [x] 3.2 In the agent exit event handler (~line 7855 where `session.agent_pid = None`), call `session_tab_store.clear_agent_pid(session.id)` — log and ignore errors
- [x] 3.3 In `interrupt_agent` (after `session.agent_pid.take()`), call `clear_agent_pid` for the session — log and ignore errors
- [x] 3.4 In `stop_agent_for_tab` (after `session.agent_pid.take()`), call `clear_agent_pid` for the session — log and ignore errors

## 4. Web Session Cleanup

- [x] 4.1 In `stop_session` in `crates/conduit-web/src/ws/handler.rs`, after removing from the in-memory map — if `pid` is `None` — read `agent_pid` and `agent_pid_start_time` from the DB via the session tab store and call `terminate_process_tree` with those values

## 5. Verification

- [x] 5.1 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and confirm all pass
