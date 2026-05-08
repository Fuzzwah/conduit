## Why

The Work Complete dialog already chains commit → push → PR creation, but stops there — the user must manually monitor CI and decide when to merge. This gap means finishing a task still requires leaving conduit and watching a browser tab, defeating the purpose of having an integrated flow.

## What Changes

- After `OpenPr` succeeds, the Work Complete dialog automatically enters a new **CI Monitoring** phase that runs `gh pr checks --watch` and displays live progress
- After `Push` succeeds when an open PR already exists, the same CI Monitoring phase is entered automatically
- Once all checks reach a terminal state, the dialog refreshes preflight and returns to the action list with `MergePr` surfaced as the top action
- The dialog blocks user interaction (swallows all keys) during CI monitoring, matching existing behaviour during action execution

## Capabilities

### New Capabilities
- `ci-monitoring-phase`: A new Work Complete phase that monitors PR CI checks after PR creation or push, streaming results into the dialog log and transitioning back to the action list when checks are terminal

### Modified Capabilities

## Impact

- `crates/conduit-ui/src/work_complete.rs` — new phase, event, and command variants; new transition arms
- `crates/conduit-ui/src/components/work_complete_dialog.rs` — render the new phase (spinner + label + log)
- `crates/conduit-ui/src/app.rs` — effect handler for CI monitoring, updated `WorkCompleteActionFinished` handler for `OpenPr` and `Push`
- `crates/conduit-ui/src/app_input.rs` — swallow keys during new phase
- `crates/conduit-ui/src/events.rs` — new `AppEvent` variant
- `crates/conduit-git/` — new `wait_for_ci_checks` function wrapping `gh pr checks --watch`
- No public API or database changes; no breaking changes
