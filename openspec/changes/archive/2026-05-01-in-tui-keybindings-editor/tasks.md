## 1. Config Layer — Action Names and TOML Persistence

- [x] 1.1 Add `action_to_name(&Action) -> Option<&'static str>` to `src/config/settings.rs` as inverse of `parse_action()` (line 425); return `None` for parametric variants (`ScrollUp`, `ScrollDown`, `SwitchToTab`, `OpenFile`)
- [x] 1.2 Add `KeyContext::toml_section_name() -> &'static str` method to `src/config/keys.rs` returning snake_case TOML table name (e.g. `"chat"`, `"scrolling"`, `"sidebar"`) matching `TomlKeybindings` field names at line 268 of `settings.rs`
- [x] 1.3 Add `save_keybinding(context: Option<KeyContext>, action_name: &str, key_str: &str) -> io::Result<()>` to `src/config/settings.rs` using `toml_edit` pattern from `save_theme_config()` (line 968): read config → parse `DocumentMut` → set `doc["keys"][action_name]` (global) or `doc["keys"][section][action_name]` (context) → write back
- [x] 1.4 Add `remove_keybinding(context: Option<KeyContext>, action_name: &str) -> io::Result<()>` to `src/config/settings.rs` using same pattern; removes the key from the TOML doc so the default takes effect on next load

## 2. Input Mode and Key Context Wiring

- [x] 2.1 Add `KeybindingsEditor` and `KeybindingsEditorCapture` variants to `InputMode` enum in `src/ui/events.rs` (before closing `}` at line ~358)
- [x] 2.2 Add arms for both new `InputMode` variants in `KeyContext::from_input_mode()` in `src/config/keys.rs` (line ~183), mapping both to `KeyContext::CommandPalette`

## 3. Keybindings Editor Component

- [x] 3.1 Create `src/ui/components/keybindings_editor.rs` with `KeybindingItem` struct (context_label, context: Option<KeyContext>, action: Action, action_name: &'static str, action_label: String, current_key: String, is_user_override: bool)
- [x] 3.2 Add `KeybindingsEditorState` struct to the new file with fields: visible, items, filtered indices, selected, scroll_offset, filter String, capture_mode bool, capture_item_idx, status_message
- [x] 3.3 Implement `build_keybinding_items(config: &KeybindingConfig) -> Vec<KeybindingItem>`: iterate `default_keybindings()` to get canonical action set, cross-reference live config for current effective key, set `is_user_override` where they differ; sort Global first then contexts alphabetically, skip parametric actions with no `action_to_name()`
- [x] 3.4 Implement filtering logic on `KeybindingsEditorState`: recompute `filtered` indices when `filter` changes (case-insensitive match on action_label and current_key)
- [x] 3.5 Implement Ratatui renderer for the dialog (~70×24 area): filter input at top, scrollable list grouped by context with section headers, overridden bindings visually marked (`*` or accent colour), capture mode prompt overlay, status message at bottom
- [x] 3.6 Register module in `src/ui/components/mod.rs` and re-export `KeybindingsEditorState` and `build_keybinding_items`

## 4. App State Integration

- [x] 4.1 Add `pub keybindings_editor_state: KeybindingsEditorState` field to `AppState` in `src/ui/app_state.rs` alongside `settings_menu_state`; initialize with `KeybindingsEditorState::default()`

## 5. Settings Menu Integration

- [x] 5.1 Add `Keybindings` variant to `SettingsMenuEntryId` enum in `src/ui/components/settings_menu.rs`
- [x] 5.2 Add `SettingsMenuEntry { id: SettingsMenuEntryId::Keybindings, title: "Keybindings", description: "Customize keyboard shortcuts", ... }` to `settings_menu_entries()` in `src/ui/app.rs` (line 4647)
- [x] 5.3 Add `SettingsMenuEntryId::Keybindings` arm to `open_selected_setting()` in `src/ui/app.rs` (line 4757): call `open_settings_child()`, build items via `build_keybinding_items`, call `keybindings_editor_state.show(items)`, set `input_mode = InputMode::KeybindingsEditor`

## 6. Input Handling

- [x] 6.1 Add early bypass in `handle_key_event()` in `src/ui/app/app_input.rs` before normal keybinding dispatch: if `input_mode == InputMode::KeybindingsEditorCapture` return `self.handle_keybinding_capture(key).await`
- [x] 6.2 Add defensive normalization in `app_input.rs` (line ~85 block): if `keybindings_editor_state.is_visible()` set `input_mode = InputMode::KeybindingsEditor`
- [x] 6.3 Implement `handle_keybinding_capture(&mut self, key: KeyEvent) -> anyhow::Result<Vec<Effect>>`: ignore modifier-only keys; Esc cancels capture; otherwise normalize to `KeyCombo`, check for conflicts, call `save_keybinding()`, update in-memory config, rebuild item list, revert to `InputMode::KeybindingsEditor`
- [x] 6.4 Add reset handler: when `Action::Delete` dispatched in `InputMode::KeybindingsEditor` and selected item `is_user_override`, call `remove_keybinding()`, restore default in-memory, rebuild item list
- [x] 6.5 Wire `Action::Cancel` for `InputMode::KeybindingsEditor` and `InputMode::KeybindingsEditorCapture` in cancel handler (`src/ui/app/app_actions_dialog.rs`): hide editor, call `return_to_settings_menu_or_normal()`

## 7. Rendering

- [x] 7.1 Add `KeybindingsEditor::new().render(...)` call in `draw()` loop in `src/ui/app.rs` (near line 11701, alongside `SettingsMenu` render call), guarded by `keybindings_editor_state.visible`

## 8. Verification

- [x] 8.1 Run `cargo fmt --check && cargo clippy -- -D warnings && cargo test` and confirm all pass
- [x] 8.2 Manual test: open Settings → Keybindings, navigate list, remap a binding, verify `~/.conduit/config.toml` updated and new binding works in-session
- [x] 8.3 Manual test: reset an overridden binding (Del/R), verify override indicator removed and default key works
- [x] 8.4 Manual test: attempt to bind a key already in use → confirm conflict message shown, no change saved
- [x] 8.5 Manual test: type filter text → list narrows; Backspace removes filter chars; Esc closes editor back to Settings
