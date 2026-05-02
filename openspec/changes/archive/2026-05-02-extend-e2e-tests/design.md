## Context

The current E2E suite uses `termwright` as the TUI driver and `socat` for IPC, with a shared `lib.sh` providing helpers (`start_conduit`, `wait_idle`, `press`, `type_text`, `assert_contains`, `assert_not_contains`, `run_test`). Tests that need pre-seeded workspace state create a `DATA_DIR`, populate a SQLite database via `sqlite3` heredocs, initialise git repos with dummy branches, and pass `--data-dir` to conduit.

The six new test scripts follow the same pattern. No new Rust code is needed. All new tests must run without a live AI API — they test navigation, dialogs, and UI state, not agent responses.

## Goals / Non-Goals

**Goals:**
- Six new test scripts, one per capability, each independently runnable
- Use only existing `lib.sh` helpers where possible; add `assert_not_contains` usage (already exists), `hotkey` helper if not present, and a `wait_for_text` helper wrapping termwright's wait mechanism
- Tests pass on both debug and release binaries (controlled by `CONDUIT_BINARY` env var)
- All six registered in `run_all.sh`

**Non-Goals:**
- Testing actual agent responses (requires live API or complex mock agent)
- Testing PR creation (requires GitHub auth)
- Testing workspace creation from scratch via the full remote-sync flow (too many external deps)
- 100% feature coverage in a single PR — this targets the highest-value, most-stable-to-test interactions

## Decisions

### Scope the six scripts to fully local, no-network interactions
All six scripts exercise UI flows that complete without network calls: key navigation, dialog open/close, tab switching, file viewer, sidebar. Workspace-lifecycle tests pre-seed the DB (same pattern as `test_tab_switch_file_tab.sh`) rather than driving the full creation wizard.

### Use `termwright` step files for complex multi-step flows
`test_session_tabs_persistence.sh` embeds YAML step files for `termwright run`. New tests that need `waitForText` (async TUI rendering) use the same inline-YAML-heredoc pattern. Simpler synchronous tests use the `run_test` + `assert_contains` pattern.

### Add `hotkey` helper to lib.sh for modifier-key combos
`press` sends a single key. Ctrl+W, Alt+T, Ctrl+1, etc. require a modifier. Add a `hotkey` function:
```bash
hotkey() {
  local sock="$1" mod="$2" key="$3"
  tw_auto "$sock" "hotkey" "{\"mod\":\"$mod\",\"key\":\"$key\"}" > /dev/null
}
```
Check whether `termwright` supports a `hotkey` command or whether modifier keys are sent as escape sequences via `type_text`. Use whichever the existing test infrastructure already uses.

### Seed strategy for workspace-lifecycle test
`test_workspace_lifecycle.sh` seeds one project + two workspaces in the DB, then tests archiving one. It does NOT test workspace creation via the UI wizard (too fragile without real git remote). Archive is triggered via the conduit command palette or key binding, confirmed via dialog, then the test asserts the workspace tab disappears.

### File viewer test reuses kind-mist fixture
`test_file_operations.sh` is structurally identical to `test_tab_switch_file_tab.sh` setup — same git branch, same `README.md` content, same DB seed SQL. Extract the seed SQL into a shared function in `lib.sh` (or duplicate inline for now; refactor later).

## Risks / Trade-offs

- **Timing** → TUI renders asynchronously after key events. All new tests use `wait_idle` after actions that trigger state changes. If flaky on CI, increase `idleMs`/`timeoutMs` values.
- **termwright modifier key support** → If `hotkey` is not a supported termwright verb, modifier combos must be sent as ANSI escape sequences via `type_text`. Check `termwright --help` or existing test usage before implementing.
- **Archive confirmation dialog** → The archive flow shows a preflight check dialog; tests must navigate through it (confirm/cancel) correctly. Read the archive dialog's expected text from `app.rs` before writing assertions.
- **Sidebar focus** → Sidebar-navigation tests require the sidebar to be visible; use `press Ctrl+T` to ensure it is open before navigating, then assert sidebar-specific text.
