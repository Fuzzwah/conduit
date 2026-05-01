## 1. Extend AgentSession with branch_name field

- [x] 1.1 Add `pub branch_name: Option<String>` field to `AgentSession` in `src/ui/session.rs` (near the `workspace_name` field, ~line 48), initialized to `None` in `AgentSession::new()` and `AgentSession::with_working_dir()`

## 2. Populate branch_name at session creation points

- [x] 2.1 In `src/ui/app.rs` ~line 701 (workspace lookup during session restore): add `session.branch_name = Some(workspace.branch.clone())` alongside the existing `session.workspace_name` assignment
- [x] 2.2 In `src/ui/app.rs` ~line 5971 (`start_new_session_in_place`): add `branch_name` to the destructured tuple from the existing session and assign it to `new_session.branch_name`
- [x] 2.3 In `src/ui/app.rs` ~line 10527 (fork session creation): add `session.branch_name = Some(workspace.branch.clone())` alongside the existing `session.workspace_name` assignment

## 3. Carry branch_name through handoff flow

- [x] 3.1 Add `pub branch_name: Option<String>` field to `PendingHandoffRequest` in `src/ui/app_state.rs` (after the `workspace_name` field, ~line 305)
- [x] 3.2 In `src/ui/app.rs` ~line 10166 (building `PendingHandoffRequest` from session): extract `session.branch_name.clone()` and include it in the struct literal
- [x] 3.3 In `src/ui/app.rs` ~line 10256 (restoring session from `PendingHandoffRequest`): add `session.branch_name = pending.branch_name.clone()`
- [x] 3.4 In `src/ui/app.rs` test instances of `PendingHandoffRequest` (~lines 14470, 14586, 14654, 14881): add `branch_name: None` to each struct literal to satisfy the compiler

## 4. Update tab_name() rendering logic

- [x] 4.1 Replace `AgentSession::tab_name()` in `src/ui/session.rs` with the new format: truncate project name to 10 chars (append `…` if longer), append ` [<trailing-branch-segment>]` when `branch_name` is set; keep existing fallback to working_dir name or `"{AgentType} (new)"` when `project_name` is absent

## 5. Verify

- [x] 5.1 Run `cargo fmt --check && cargo clippy -- -D warnings && cargo test` and confirm all pass
- [x] 5.2 Run conduit manually and verify tab titles show the new format (e.g., `conduit [old-rose]`)
