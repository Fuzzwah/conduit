## 1. Action Definitions

- [x] 1.1 Add `MoveTabLeft` and `MoveTabRight` variants to the `Action` enum in `src/ui/action.rs` (after `SwitchToTab`)
- [x] 1.2 Add description strings for both actions in the `Action` label/describe match (~line 285 of `src/ui/action.rs`)
- [x] 1.3 Add both actions to the `show_in_palette()` match in `src/ui/action.rs` alongside `CloseTab`/`NextTab`/`PrevTab`

## 2. TabManager Methods

- [x] 2.1 Add `move_tab_left()` to `TabManager` in `src/ui/tab_manager.rs`: swap `tabs[active_tab - 1]` and `tabs[active_tab]`, decrement `active_tab`; no-op if already at index 0
- [x] 2.2 Add `move_tab_right()` to `TabManager` in `src/ui/tab_manager.rs`: swap `tabs[active_tab]` and `tabs[active_tab + 1]`, increment `active_tab`; no-op if already at last index

## 3. Keybindings

- [x] 3.1 Add `Alt+Shift+Left` → `Action::MoveTabLeft` in `src/config/default_keys.rs` using `config.global.insert(KeyCombo::new(KeyCode::Left, KeyModifiers::ALT | KeyModifiers::SHIFT), ...)`
- [x] 3.2 Add `Alt+Shift+Right` → `Action::MoveTabRight` in `src/config/default_keys.rs` using `config.global.insert(KeyCombo::new(KeyCode::Right, KeyModifiers::ALT | KeyModifiers::SHIFT), ...)`

## 4. Action Handlers

- [x] 4.1 Add handler for `Action::MoveTabLeft` in `src/ui/app/app_actions_tabs.rs`: call `self.state.tab_manager.move_tab_left()` and push `Effect::SaveSessionState`
- [x] 4.2 Add handler for `Action::MoveTabRight` in `src/ui/app/app_actions_tabs.rs`: call `self.state.tab_manager.move_tab_right()` and push `Effect::SaveSessionState`

## 5. Verification

- [x] 5.1 Run `cargo fmt --check && cargo clippy -- -D warnings && cargo test` and confirm all pass
