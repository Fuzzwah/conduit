## 1. Extend WorkspaceCreated struct

- [x] 1.1 Add `initial_message: Option<String>` field to `WorkspaceCreated` struct in `src/ui/events.rs`

## 2. Compose context-load message during workspace creation

- [x] 2.1 In the `Effect::CreateWorkspace` handler in `src/ui/app.rs`, derive `initial_message` from `spec` and `specify_spec` after the `(workspace_name, branch_name)` match block: OpenSpec → message referencing `openspec/changes/{change_id}/`; Specify → message referencing `.specify/specs/{spec_id}/tasks.md`; no spec → `None`
- [x] 2.2 Pass `initial_message` into `Ok(WorkspaceCreated { repo_id, workspace_id, initial_message })` at the end of the creation closure

## 3. Store pending message in AppState

- [x] 3.1 Add `pending_created_workspace_initial_message: Option<String>` field to `AppState` in `src/ui/app_state.rs`, next to `pending_created_workspace_id`, initialized to `None`
- [x] 3.2 In the `AppEvent::WorkspaceCreated` handler in `src/ui/app.rs`, set `self.state.pending_created_workspace_initial_message = created.initial_message.clone()` alongside the existing `pending_created_workspace_id` assignment

## 4. Auto-send message on workspace open

- [x] 4.1 In `close_workspace_progress_dialog` in `src/ui/app.rs`, after `open_workspace_with_options`, take `self.state.pending_created_workspace_initial_message`; if `Some(msg)`, call `self.submit_prompt(msg, vec![], vec![])` and append the returned effects to the function's return value

## 5. Verification

- [x] 5.1 Run `cargo fmt --check && cargo clippy -- -D warnings && cargo test` and confirm all pass
- [x] 5.2 Manual test: create a workspace from an OpenSpec change, confirm the agent receives and processes the context message on first open
- [x] 5.3 Manual test: create a workspace with no spec, confirm no message is auto-sent
- [x] 5.4 Manual test: close and reopen a spec-linked workspace, confirm the context message is not re-sent

