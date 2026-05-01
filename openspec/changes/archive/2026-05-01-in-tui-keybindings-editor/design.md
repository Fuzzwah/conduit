## Context

Conduit already has a complete keybinding infrastructure: `KeybindingConfig` (runtime lookup), `KeyCombo` (key representation), `KeyContext` (19 contexts), `Action` (60+ actions), and `default_keybindings()`. Config is loaded from `~/.conduit/config.toml` at startup; user overrides are merged on top of defaults. Saves use `toml_edit` for structure-preserving writes (same pattern as `save_theme_config()`, `save_default_model()`, etc. in `settings.rs`).

The Settings menu already uses a child-dialog pattern: a parent `SettingsMenuState` opens child dialogs (model selector, theme picker, workspace defaults, etc.) in sequence. The keybindings editor follows this same pattern.

## Goals / Non-Goals

**Goals:**
- Browse all keybindings grouped by context with a live filter
- Remap any binding by pressing Enter then the new key combo
- Reset an overridden binding to its default (Del or R)
- Visually distinguish user-overridden bindings from defaults
- Save changes to `~/.conduit/config.toml` and apply in-memory immediately

**Non-Goals:**
- Rebinding parametric actions (`ScrollUp(u16)`, `SwitchToTab(u8)`) — no static TOML key exists
- Binding multiple keys to one action (TOML schema is 1:1)
- Global undo/redo across multiple edits in one session
- Exporting or importing keybinding profiles

## Decisions

### 1. Reuse `KeyContext::CommandPalette` instead of a new `KeyContext`

Two new `InputMode` variants are added (`KeybindingsEditor`, `KeybindingsEditorCapture`). Both map to `KeyContext::CommandPalette` in `from_input_mode()`. CommandPalette already binds Up/Down/Enter/Esc/Backspace — exactly what the editor needs for navigation and filter input. A dedicated `KeyContext::KeybindingsEditor` would require duplicating those bindings with no new behaviour.

*Alternative considered*: New `KeyContext::KeybindingsEditor`. Rejected — identical bindings, extra churn.

### 2. Capture mode as a separate `InputMode` with early bypass

When the user presses Enter to remap a binding, the app transitions to `InputMode::KeybindingsEditorCapture`. In `handle_key_event()` this mode is checked before the normal keybinding lookup, so the raw `KeyEvent` is handed directly to `handle_keybinding_capture()`. This ensures no action is dispatched for the captured key.

*Alternative considered*: A flag on `KeybindingsEditorState`. Rejected — the existing InputMode-based pattern is how all modal states work; a flag would bypass the architecture.

### 3. `action_to_name()` as inverse of `parse_action()`

`parse_action()` in `settings.rs` maps `&str → Action`. A companion `action_to_name(&Action) -> Option<&'static str>` is added alongside it. Returns `None` for parametric variants (filtering them from the editor list). This co-location ensures the exhaustiveness check catches new actions at compile time.

### 4. `save_keybinding()` and `remove_keybinding()` follow existing `toml_edit` pattern

Both functions: read config file → parse to `DocumentMut` → edit the `[keys]` or `[keys.<context>]` table → write back. Context subtable names come from a new `KeyContext::toml_section_name() -> &'static str` method returning snake_case strings matching `TomlKeybindings` field names.

After save, in-memory `config.keybindings` is updated directly (insert or remove + re-insert default). No full `Config::load()` reload needed.

### 5. Build item list from `default_keybindings()` cross-referenced with live config

`build_keybinding_items()` iterates `default_keybindings()` (the canonical action set), looks up the current effective key from the live `config.keybindings`, and sets `is_user_override = true` when they differ. This guarantees every action is listed exactly once and that "reset to default" always has a known target value.

## Risks / Trade-offs

- **Conflict detection complexity** → `KeybindingConfig::get_action()` already encodes global-takes-precedence logic. The conflict check scans both the global map and the context map; a key bound globally that would shadow a context binding shows a warning but is still allowed (consistent with how the runtime resolves them).
- **TOML structure for global vs. context bindings** → `TomlKeybindings` uses serde flatten for global fields, meaning global action names sit directly under `[keys]` while context bindings go under `[keys.chat]` etc. The `save_keybinding()` implementation must handle both cases correctly.
- **Terminal modifier-only events** → Some terminals send standalone modifier key events (Shift, Ctrl). The capture handler filters these with `matches!(key.code, KeyCode::Modifier(_))` to avoid capturing bare modifiers as bindings.
- **Long action list performance** → 100+ bindings, but all in-memory filtering with simple string contains. No performance concern.

## Migration Plan

No migration needed. Changes are additive:
- New `InputMode` variants are non-breaking (exhaustive matches in `from_input_mode` are updated).
- New `SettingsMenuEntryId::Keybindings` is non-breaking.
- `~/.conduit/config.toml` is only written when the user explicitly saves a binding.
- No schema changes to existing TOML format.

## Open Questions

None — design is fully resolved based on the existing codebase patterns.
