## Why

Conduit users running Ghostty (or other terminals) with system hotkeys disabled want to use those freed key combos (cmd+number, cmd+backtick, etc.) in conduit, but remapping currently requires manually editing `~/.conduit/config.toml` — there's no in-app UI for it. Adding a TUI keybindings editor makes customization accessible without leaving conduit.

## What Changes

- New "Keybindings" entry in the Settings menu (Alt+,)
- New dialog component showing all current keybindings grouped by context (Global, Chat, Sidebar, etc.) with a live filter input
- Selecting a binding and pressing Enter enters capture mode: the next keypress becomes the new binding
- Overridden bindings are visually marked; Del or R resets them to their default
- Changes are saved to `~/.conduit/config.toml` and applied in-memory immediately (no restart required)

## Capabilities

### New Capabilities

- `keybindings-editor`: In-TUI dialog for browsing, remapping, and resetting keyboard shortcuts. Accessible from the Settings menu. Supports filter-as-you-type, capture mode for new key input, and per-binding reset to default.

### Modified Capabilities

<!-- No existing spec-level behavior changes -->

## Impact

- **New file**: `src/ui/components/keybindings_editor.rs`
- **Modified**: `src/config/settings.rs` — add `action_to_name()`, `save_keybinding()`, `remove_keybinding()`
- **Modified**: `src/config/keys.rs` — add `KeyContext::toml_section_name()`, new `from_input_mode` arms
- **Modified**: `src/ui/events.rs` — two new `InputMode` variants (`KeybindingsEditor`, `KeybindingsEditorCapture`)
- **Modified**: `src/ui/components/settings_menu.rs` — add `Keybindings` entry
- **Modified**: `src/ui/app_state.rs` — add `keybindings_editor_state` field
- **Modified**: `src/ui/app.rs` — wire settings entry, render call, action dispatch
- **Modified**: `src/ui/app/app_input.rs` — capture mode bypass, defensive normalization
- **Dependencies**: No new crate dependencies; `toml_edit` (already used) handles config writes
