## 1. Config layer — prefix storage and loading

- [x] 1.1 Add `switch_to_tab_prefix: String` field (default `"M-"`) to the TOML config struct in `crates/conduit-config/src/settings.rs`
- [x] 1.2 Parse `switch_to_tab_prefix` from `[keybindings]` in `TomlKeybindings::to_keybinding_config()` and store it on `KeybindingConfig`
- [x] 1.3 Add `switch_to_tab_prefix: String` to `KeybindingConfig` in `crates/conduit-config/src/keys.rs` with default `"M-"`

## 2. Default keybindings — drive loop from prefix

- [x] 2.1 Add `pub fn default_keybindings_with_prefix(prefix: &str) -> KeybindingConfig` to `crates/conduit-config/src/default_keys.rs`; have `default_keybindings()` delegate to it with `"M-"`
- [x] 2.2 In `to_keybinding_config()`, call `default_keybindings_with_prefix(&prefix)` (using the loaded prefix) when building the user's live keybinding config so the nine `SwitchToTab` bindings use the stored prefix

## 3. Keybinding editor — display the prefix row

- [x] 3.1 In `build_keybinding_items()` (`crates/conduit-ui/src/components/keybindings_editor.rs`), after the global loop, append one `KeybindingItem` with `action_name = "switch_to_tab_prefix"`, `action_label = "Switch to tab (1–9)"`, `current_key` formatted as e.g. `Alt+N` from the live prefix, `default_key = "Alt+N"`, and `is_user_override` = true if prefix differs from `"M-"`
- [x] 3.2 Add a helper `format_prefix_for_display(prefix: &str) -> String` that converts `"M-"` → `"Alt+N"`, `"C-"` → `"Ctrl+N"`, `"C-M-"` → `"Ctrl+Alt+N"`, etc.

## 4. Keybinding editor — capture mode for prefix row

- [x] 4.1 In the capture-mode keypress handler, detect `action_name == "switch_to_tab_prefix"` and apply different logic: require the combo to end in digit 1–9, strip the digit, derive the prefix string
- [x] 4.2 Show a custom prompt in capture mode for the prefix row: "Press modifier + digit 1–9 (e.g. Alt+1)"
- [x] 4.3 If the captured combo does not end in a digit 1–9, display an error and remain in capture mode

## 5. Save and apply

- [x] 5.1 In the save path for the prefix row, write `switch_to_tab_prefix = "..."` into `[keybindings]` in `~/.conduit/config.toml` using `toml_edit`
- [x] 5.2 After saving, rebuild the nine `SwitchToTab` bindings in the live `KeybindingConfig`: clear old modifier+digit combos from `global` and re-run the loop with the new prefix
- [x] 5.3 In the reset path for the prefix row, remove `switch_to_tab_prefix` from `~/.conduit/config.toml` and restore `"M-"` bindings in memory

## 6. Verification

- [x] 6.1 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and confirm all pass
- [ ] 6.2 Manual smoke test: open keybindings editor, verify "Switch to tab (1–9)" row appears, remap to `Ctrl+N`, confirm Ctrl+1 switches tabs, reset, confirm Alt+1 works again
