## ADDED Requirements

### Requirement: Tab-switch modifier prefix is configurable via TOML
The system SHALL read a `switch_to_tab_prefix` key from the `[keybindings]` section of `~/.conduit/config.toml` and use it as the modifier prefix for the nine tab-switching bindings (tabs 1–9). When the key is absent the default prefix SHALL be `"M-"` (Alt).

#### Scenario: Default prefix used when key absent
- **WHEN** `~/.conduit/config.toml` contains no `switch_to_tab_prefix` key
- **THEN** pressing Alt+1 through Alt+9 switches to the corresponding tab

#### Scenario: Custom prefix applied on load
- **WHEN** `~/.conduit/config.toml` contains `switch_to_tab_prefix = "C-"` under `[keybindings]`
- **THEN** pressing Ctrl+1 through Ctrl+9 switches to the corresponding tab and Alt+1 through Alt+9 no longer trigger tab switching

### Requirement: Tab-switch prefix change takes effect without restart
The system SHALL apply a new `switch_to_tab_prefix` to the live session immediately when the user saves the change via the keybindings editor.

#### Scenario: New prefix active immediately
- **WHEN** the user changes the tab-switch prefix from `M-` to `C-` in the keybindings editor and saves
- **THEN** pressing Ctrl+2 immediately switches to tab 2 in the same session without restarting conduit

### Requirement: Tab-switch prefix reset restores Alt+1–9
The system SHALL restore the default `M-` prefix bindings and remove `switch_to_tab_prefix` from `~/.conduit/config.toml` when the user resets the tab-switch entry in the keybindings editor.

#### Scenario: Reset removes TOML key and restores default
- **WHEN** the user selects the "Switch to tab (1–9)" row in the keybindings editor and presses Del or R
- **THEN** `switch_to_tab_prefix` is removed from `~/.conduit/config.toml`, Alt+1–9 are restored, and the override indicator disappears from the row
