## 1. Git Layer — CI Check Helper

- [x] 1.1 Add `wait_for_ci_checks(pr_url: &str) -> Result<(bool, Vec<String>), String>` to `crates/conduit-git/src/` (new file `ci_monitor.rs` or append to `actions.rs`) — runs `gh pr checks --watch <pr_url>`, captures stdout+stderr, returns `(passed, lines)` based on exit code
- [x] 1.2 Export the new function from `crates/conduit-git/src/lib.rs`

## 2. State Machine — New Phase, Event, Command

- [x] 2.1 Add `MonitoringCi { pr_url: String }` variant to `WorkCompletePhase` in `crates/conduit-ui/src/work_complete.rs`
- [x] 2.2 Add `CiStarted { pr_url: String }` and `CiCompleted { passed: bool, log: Vec<String> }` variants to `WorkCompleteEvent`
- [x] 2.3 Add `MonitorCi { pr_url: String }` variant to `WorkCompleteCommand`
- [x] 2.4 Add transition arms in `transition()`:
  - `(_, CiStarted { pr_url })` → `MonitoringCi { pr_url }` + `[MonitorCi { pr_url }]`
  - `(MonitoringCi, CiCompleted { .. })` → `LoadingPreflight` + `[RefreshPreflight]`
  - `(MonitoringCi, Close)` → no-op (stay in phase, no commands)

## 3. App Event and Effect

- [x] 3.1 Add `WorkCompleteCiFinished { workspace_id: Uuid, result: Result<(bool, Vec<String>), String> }` to `AppEvent` in `crates/conduit-ui/src/events.rs`
- [x] 3.2 Add `WorkCompleteCiMonitor { workspace_id: Uuid, pr_url: String }` to the `Effect` enum (or wherever effects are defined)
- [x] 3.3 Add effect handler: spawn `tokio::task::spawn_blocking` calling `wait_for_ci_checks`, on completion send `AppEvent::WorkCompleteCiFinished`
- [x] 3.4 Add `AppEvent::WorkCompleteCiFinished` handler: extend `session.log` with result lines, dispatch `WorkCompleteEvent::CiCompleted`

## 4. Action Finished Handler Updates

- [x] 4.1 In the `WorkCompleteActionFinished` handler in `crates/conduit-ui/src/app.rs`, for `OpenPr` success: parse the PR URL from the log line (format `"Created PR #N: <url>"`), extend `session.log`, dispatch `CiStarted { pr_url }` instead of `ActionCompleted`
- [x] 4.2 In the same handler, for `Push` success: check `session.data.as_ref()?.pr` for an open PR URL; if found, dispatch `CiStarted { pr_url }` instead of `ActionCompleted`; if not found, fall through to existing `ActionCompleted` path
- [x] 4.3 Add `WorkCompleteCommand::MonitorCi { pr_url }` mapping in `dispatch_work_complete_event()` → push `Effect::WorkCompleteCiMonitor { workspace_id, pr_url }`

## 5. Input Handling

- [x] 5.1 In `handle_work_complete_key()` in `crates/conduit-ui/src/app_input.rs`, add `MonitoringCi` to the guard that swallows all keypresses (same pattern as `Executing`)

## 6. Dialog Rendering

- [x] 6.1 In `crates/conduit-ui/src/components/work_complete_dialog.rs`, add a render branch for `WorkCompletePhase::MonitoringCi { pr_url }`: show spinner + `"Monitoring CI checks…"` label + PR URL subtitle + accumulated log lines (reuse the `Executing` phase render as a model)

## 7. Verification

- [x] 7.1 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and confirm all pass
- [x] 7.2 Manual test: commit → push → PR created → dialog enters CI monitoring spinner → checks complete → dialog returns to action list with MergePr visible
- [x] 7.3 Manual test: with existing open PR, push → dialog enters CI monitoring automatically
- [x] 7.4 Manual test: push with no open PR → no CI monitoring phase, normal preflight refresh
