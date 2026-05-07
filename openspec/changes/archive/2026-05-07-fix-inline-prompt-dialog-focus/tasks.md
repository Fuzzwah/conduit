## 1. Implementation

- [x] 1.1 In `crates/conduit-ui/src/app/app_input.rs` around line 187, extend the inline prompt guard: add `let has_blocking_dialog = self.has_active_dialog() || self.state.work_complete_session.is_some();` and change the condition to `if !sidebar_has_focus && !has_blocking_dialog {`

## 2. Verification

- [x] 2.1 Run `cargo fmt --check` and confirm clean
- [x] 2.2 Run `cargo clippy --workspace --all-targets -- -D warnings` and confirm clean
- [x] 2.3 Run `cargo test --workspace` and confirm all tests pass
