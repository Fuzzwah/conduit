## Why

The E2E test suite currently covers ~15% of user-facing features (22 of ~130 interactions), leaving critical flows — workspace lifecycle, tab management, slash commands, settings dialogs, sidebar advanced navigation, and clipboard operations — completely untested. Expanding coverage now, while the codebase has just been restructured into a clean workspace layout, gives regressions a place to surface before they reach users.

## What Changes

- Add six new E2E test scripts under `crates/conduit/tests/e2e/`, each targeting a coherent feature group
- Register the new scripts in `run_all.sh`
- Expand existing `lib.sh` test helpers if new assertion patterns are needed (e.g. `assert_not_contains`, `wait_for_key_result`)
- No changes to Rust source code — this is purely a test-coverage addition

## Capabilities

### New Capabilities

- `e2e-workspace-lifecycle`: Tests covering workspace creation (naming, branch), archiving, and the confirmation dialog flow
- `e2e-tab-management`: Tests covering numbered tab navigation (Ctrl+1–9), tab close (Ctrl+W), and switching between multiple open tabs
- `e2e-slash-commands`: Tests covering the `/` slash-command menu, command palette (Ctrl+P), and basic command selection and cancellation
- `e2e-settings-dialogs`: Tests covering model selector (Ctrl+O), provider selector, theme picker (Alt+T), and session import picker (Alt+I) — open, navigate, dismiss
- `e2e-sidebar-navigation`: Tests covering sidebar focus (Ctrl+T), project tree expand/collapse, workspace selection from sidebar, and sidebar search/filter
- `e2e-file-operations`: Tests covering opening a file in the file viewer, scrolling in the file viewer, and closing a file tab independently of the workspace tab

### Modified Capabilities

<!-- None — no existing spec-level behavior changes -->

## Impact

- **New files**: Six shell test scripts, one spec file per capability above
- **Modified files**: `crates/conduit/tests/e2e/run_all.sh` (register new tests), possibly `crates/conduit/tests/e2e/lib.sh` (new helpers)
- **Dependencies**: Same as existing E2E tests — requires `termwright`, `socat`, `jq`, `sqlite3`, and a built `target/release/conduit` binary
- **CI**: New tests run in the same E2E job; no CI config changes beyond what already runs `run_all.sh`
- **Risks**: Timing-sensitive tests may need generous `Sleep`/timeout values; tests that exercise workspace creation require seeded DB state similar to `test_tab_switch_file_tab.sh`
