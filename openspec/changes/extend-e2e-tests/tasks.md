## 1. Prerequisite: review and extend lib.sh helpers

- [ ] 1.1 Read `lib.sh` fully and confirm `ctrl`, `alt`, `press`, `type_text`, `assert_contains`, `assert_not_contains`, and `run_test` all exist and work as expected — they are needed by every new test
- [ ] 1.2 Check whether a `ctrl_num` or similar helper exists for Ctrl+1…9; if not, confirm that `ctrl "$sock" "1"` works via the existing `ctrl` helper (it sends `{"ctrl":true,"ch":"1"}`)
- [ ] 1.3 Read the archive dialog text from `crates/conduit-ui/src/app.rs` (search for "Archive" near confirmation dialog rendering) so assertions match the exact strings shown on screen
- [ ] 1.4 Read the tab bar rendering in `crates/conduit-ui/src/components/tab_bar.rs` to confirm the active-tab indicator character (currently `▸`) used in assertions

## 2. Write `test_workspace_lifecycle.sh`

- [ ] 2.1 Scaffold the script: copy the DB-seed pattern from `test_tab_switch_file_tab.sh` — create `DATA_DIR`, init git repo with `test/kind-mist` branch, insert project + workspace rows into SQLite
- [ ] 2.2 Start conduit with `start_conduit "$DATA_DIR"`, `wait_idle`, then assert the workspace tab `[kind-mist]` is visible
- [ ] 2.3 Trigger the archive action (find the correct key binding from `app.rs` — likely via command palette or a direct keybinding); use `ctrl` or `alt` helper as appropriate
- [ ] 2.4 Assert the archive preflight dialog text appears on screen (e.g. "Archive" and the workspace name)
- [ ] 2.5 Send Escape and assert the workspace tab is still present (`assert_contains … "[kind-mist]"`)
- [ ] 2.6 Trigger archive again, this time confirm (press Enter or the confirm key shown in the dialog)
- [ ] 2.7 `wait_idle` then assert the workspace tab is gone (`assert_not_contains … "[kind-mist]"`)
- [ ] 2.8 Run the script manually against a release binary; fix any assertion text mismatches

## 3. Write `test_tab_management.sh`

- [ ] 3.1 Scaffold: seed two workspace entries (`kind-mist` + `live-jade`) using the same DB/git pattern as the existing file-tab test; verify both tabs show `[kind-mist]` and `[live-jade]`
- [ ] 3.2 Assert the first tab shows the active-tab indicator (`▸`) via `assert_contains`
- [ ] 3.3 Press `ctrl "$sock" "2"` (Ctrl+2) and `wait_idle`; assert `[live-jade]` now has the active indicator
- [ ] 3.4 Press `ctrl "$sock" "1"` (Ctrl+1) and `wait_idle`; assert `[kind-mist]` is active again
- [ ] 3.5 Press `ctrl "$sock" "w"` (Ctrl+W) to close the active tab; `wait_idle`; assert `[kind-mist]` tab is gone and `[live-jade]` is now the only/active tab
- [ ] 3.6 Run the script manually; fix any timing issues

## 4. Write `test_slash_commands.sh`

- [ ] 4.1 Scaffold: seed one workspace (`kind-mist`) and start conduit; the workspace session tab must be active for slash commands to be available
- [ ] 4.2 `type_text "$sock" "/"` then `wait_idle 100 2000`; assert the slash menu appears (look for a menu title or a known command like `open` or `help`)
- [ ] 4.3 Press Escape; `wait_idle`; assert the slash menu is gone (`assert_not_contains`)
- [ ] 4.4 Press `ctrl "$sock" "p"` (Ctrl+P); `wait_idle`; assert the command palette dialog appears (look for its title text)
- [ ] 4.5 Press Escape; `wait_idle`; assert the command palette is gone
- [ ] 4.6 Type `/op` and `wait_idle`; assert the menu filters to show only commands matching "op" (assert a matching command is visible and an unrelated command is absent)
- [ ] 4.7 Run manually; adjust timing and assertion strings as needed

## 5. Write `test_settings_dialogs.sh`

- [ ] 5.1 Scaffold: use a fresh data dir (no pre-seeded workspaces needed — model/theme pickers work from the main screen)
- [ ] 5.2 Start conduit; assert the main screen shows provider selector and default model selector text (reuse existing `test_project_add` assertions as a reference)
- [ ] 5.3 Press `ctrl "$sock" "o"` (Ctrl+O); `wait_idle`; assert model selector dialog appears (look for a model name string like "claude" or "sonnet")
- [ ] 5.4 Press Escape; `wait_idle`; assert model selector is gone
- [ ] 5.5 Press `alt "$sock" "t"` (Alt+T); `wait_idle`; assert theme picker appears (look for "Dracula" or "Default" or similar theme name)
- [ ] 5.6 Press Escape; `wait_idle`; assert theme picker is gone
- [ ] 5.7 Press `alt "$sock" "i"` (Alt+I); `wait_idle`; assert session import picker appears (look for its dialog title)
- [ ] 5.8 Press Escape; `wait_idle`; assert import picker is gone
- [ ] 5.9 Run manually; fix assertion strings

## 6. Write `test_sidebar_navigation.sh`

- [ ] 6.1 Scaffold: seed two workspaces under one project (`kind-mist` + `live-jade`) so the sidebar tree has meaningful content
- [ ] 6.2 Start conduit with sidebar visible (it is shown by default on startup with workspaces); assert project name and `[kind-mist]` appear in the sidebar area
- [ ] 6.3 Press `ctrl "$sock" "t"` to toggle sidebar off; `wait_idle`; assert project name is no longer visible
- [ ] 6.4 Press `ctrl "$sock" "t"` again to toggle sidebar back on; `wait_idle`; assert project name is visible again
- [ ] 6.5 Navigate sidebar with Down arrow: `press "$sock" "Down"`; `wait_idle`; assert a different item has the active indicator
- [ ] 6.6 Press Up arrow; `wait_idle`; assert the previous item is selected again
- [ ] 6.7 Navigate to `live-jade` in the sidebar and press Enter; `wait_idle`; assert `[live-jade]` is the active tab (active indicator visible on `[live-jade]`)
- [ ] 6.8 Run manually; identify correct sidebar indicator text for assertions

## 7. Write `test_file_operations.sh`

- [ ] 7.1 Scaffold: re-use the exact seed from `test_tab_switch_file_tab.sh` (one workspace, `kind-mist` branch, `README.md` with known content "FILE TAB MARKER")
- [ ] 7.2 Start conduit; assert workspace tab `[kind-mist]` is visible
- [ ] 7.3 Open file viewer: `press "$sock" ":"`, `type_text "$sock" "open README.md"`, `press "$sock" "Enter"`, `wait_idle 500 5000`; assert "FILE TAB MARKER" is visible on screen
- [ ] 7.4 Assert a second tab (file viewer) is now visible in the tab bar (look for "README.md" or the file-tab indicator)
- [ ] 7.5 Press Tab; `wait_idle`; assert the workspace tab is now active (active indicator on `[kind-mist]`)
- [ ] 7.6 Press Tab again; `wait_idle`; assert the file viewer tab is active
- [ ] 7.7 Press `ctrl "$sock" "w"`; `wait_idle`; assert the file viewer tab is gone and "FILE TAB MARKER" is no longer visible
- [ ] 7.8 Assert the workspace tab `[kind-mist]` is still present after closing the file tab
- [ ] 7.9 Run manually; fix timing and assertion strings

## 8. Register new tests and verify

- [ ] 8.1 Add all six new script filenames to `crates/conduit/tests/e2e/run_all.sh` (or confirm it auto-discovers `test_*.sh` files — check the discovery mechanism)
- [ ] 8.2 Run the full suite with `bash crates/conduit/tests/e2e/run_all.sh` and confirm all six new tests pass alongside the existing five
- [ ] 8.3 If any test is flaky on a second run, increase `idleMs`/`timeoutMs` in the relevant `wait_idle` call
