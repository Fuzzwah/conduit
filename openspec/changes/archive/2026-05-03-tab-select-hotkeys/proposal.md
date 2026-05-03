## Why

The keybindings editor has no entry for the Alt+1–9 tab-switching shortcuts, so users cannot rebind them. The underlying `SwitchToTab(u8)` action is parametric, which causes it to be silently excluded from the editor's enumeration.

## What Changes

- Add a `switch_to_tab_prefix` config key (TOML) that stores the modifier prefix used for tab-number switching (default: `"M-"` for Alt).
- On config load, regenerate the nine `SwitchToTab(1)`–`SwitchToTab(9)` bindings from whatever prefix is stored.
- Expose a single row in the keybindings editor labelled "Switch to tab (1–9)" that shows the current modifier (e.g. `Alt+N`) and allows capture-mode rebinding of the modifier portion only.
- Teach capture mode to strip the digit suffix from pressed combos so the user presses e.g. `Alt+3` and conduit stores `M-` as the prefix.

## Capabilities

### New Capabilities

- `tab-select-hotkeys`: A single configurable modifier prefix for the Alt+1–9 tab-switching shortcuts, exposed as one row in the keybindings editor.

### Modified Capabilities

- `keybindings-editor`: The editor must display and support capture-mode rebinding for parametric prefix entries, not just static one-to-one action bindings.

## Impact

- `crates/conduit-config/src/settings.rs` — add `switch_to_tab_prefix` field; update `action_to_name` / name-to-action roundtrip for the prefix entry.
- `crates/conduit-config/src/default_keys.rs` — loop now driven by the loaded prefix instead of a hardcoded `"M-"`.
- `crates/conduit-types/src/action.rs` — no change to the `SwitchToTab(u8)` variant itself; a new companion concept (the prefix binding) lives in config.
- `crates/conduit-ui/src/components/keybindings_editor.rs` — `build_keybinding_items` and capture-mode logic extended to handle the prefix entry type.
