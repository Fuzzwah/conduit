### Requirement: Keybindings editor accessible from Settings menu
The system SHALL provide a "Keybindings" entry in the Settings menu that opens the keybindings editor dialog.

#### Scenario: Open keybindings editor from settings
- **WHEN** the user opens Settings (Alt+,) and selects "Keybindings"
- **THEN** the keybindings editor dialog opens showing all current bindings

#### Scenario: Close keybindings editor returns to settings
- **WHEN** the user presses Esc in the keybindings editor
- **THEN** the editor closes and focus returns to the Settings menu

### Requirement: Keybindings displayed grouped by context
The system SHALL display all keybindings grouped by context (Global first, then Chat, Sidebar, etc.) with a filter input at the top. The Global group SHALL include a "Switch to tab (1–9)" row representing the tab-switch modifier prefix.

#### Scenario: Default display groups
- **WHEN** the keybindings editor opens
- **THEN** bindings are shown with Global bindings first, followed by other contexts in alphabetical order, with a filter input visible at the top

#### Scenario: Filter narrows list across all groups
- **WHEN** the user types text in the filter input
- **THEN** only bindings whose action name or current key contains the filter text (case-insensitive) are shown, across all context groups

#### Scenario: Filter cleared with Backspace
- **WHEN** the user presses Backspace in the editor
- **THEN** the last character is removed from the filter input and the list updates accordingly

#### Scenario: Switch-to-tab prefix row shown in Global group
- **WHEN** the keybindings editor opens
- **THEN** a row labelled "Switch to tab (1–9)" appears in the Global group showing the current modifier (e.g. `Alt+N`)

### Requirement: User-overridden bindings visually distinguished
The system SHALL visually mark bindings that have been overridden by the user (differ from the default).

#### Scenario: Override indicator shown
- **WHEN** a binding in the list has been changed from its default value
- **THEN** the binding row is visually distinguished (e.g., with an asterisk or accent colour)

#### Scenario: Default bindings have no indicator
- **WHEN** a binding in the list matches its default value
- **THEN** no override indicator is shown for that row

### Requirement: User can remap a keybinding via capture mode
The system SHALL allow the user to remap a keybinding by selecting it and pressing Enter, then pressing the desired new key combo.

#### Scenario: Enter capture mode
- **WHEN** the user selects a binding row and presses Enter
- **THEN** the UI enters capture mode, showing a prompt indicating it is waiting for a keypress

#### Scenario: New key saved and applied
- **WHEN** the user presses a key combo in capture mode
- **THEN** the binding is updated in `~/.conduit/config.toml`, applied in-memory immediately, and the list reflects the new key

#### Scenario: Capture mode cancelled with Esc
- **WHEN** the user presses Esc in capture mode
- **THEN** capture mode is exited without any changes, returning to the list

#### Scenario: Conflicting key shows error
- **WHEN** the user presses a key in capture mode that is already bound to a different action in the same context
- **THEN** a conflict message is shown and no change is saved

#### Scenario: Bare modifier keys ignored in capture mode
- **WHEN** the user presses a standalone modifier key (Shift, Ctrl, Alt) in capture mode
- **THEN** the keypress is ignored and capture mode remains active

### Requirement: User can reset an overridden binding to its default
The system SHALL allow the user to reset an overridden binding back to its default value by pressing Del or R.

#### Scenario: Reset overridden binding
- **WHEN** the user selects an overridden binding and presses Del or R
- **THEN** the user override is removed from `~/.conduit/config.toml`, the default binding is restored in-memory, and the override indicator disappears from the row

#### Scenario: Reset on default binding is a no-op
- **WHEN** the user presses Del or R on a binding that is already at its default
- **THEN** a status message indicates it is already the default and no changes are made

### Requirement: Changes applied without restart
The system SHALL apply saved keybinding changes immediately in the running session without requiring a restart.

#### Scenario: Immediate in-memory application
- **WHEN** a binding is saved or reset
- **THEN** the new binding takes effect immediately in the running conduit session

### Requirement: User can remap the tab-switch modifier prefix
The system SHALL allow the user to remap the modifier used for tab-switching by selecting the "Switch to tab (1–9)" row and pressing Enter to enter capture mode, then pressing any key combo ending in a digit 1–9. The digit is stripped and the remaining modifier prefix is stored.

#### Scenario: Enter capture mode for prefix row
- **WHEN** the user selects the "Switch to tab (1–9)" row and presses Enter
- **THEN** the UI enters capture mode showing a prompt "Press modifier + a digit (e.g. Alt+1)"

#### Scenario: New prefix saved and applied
- **WHEN** the user presses a key combo ending in a digit while in capture mode for the prefix row (e.g. Ctrl+3)
- **THEN** the modifier prefix (e.g. `C-`) is written to `switch_to_tab_prefix` in `~/.conduit/config.toml`, all nine tab-switch bindings are updated in memory, and the row reflects the new modifier

#### Scenario: Non-digit combo rejected in capture mode
- **WHEN** the user presses a key combo that does not end in a digit 1–9 while in capture mode for the prefix row
- **THEN** an error message is shown ("Must press modifier + digit 1–9") and capture mode remains active

#### Scenario: Capture mode cancelled with Esc
- **WHEN** the user presses Esc in capture mode for the prefix row
- **THEN** capture mode is exited without any changes

### Requirement: Tab-switch prefix row shows override indicator when changed
The system SHALL visually distinguish the "Switch to tab (1–9)" row when the user has set a non-default prefix, using the same override indicator as other overridden bindings.

#### Scenario: Override indicator shown for non-default prefix
- **WHEN** `switch_to_tab_prefix` in config differs from the default `"M-"`
- **THEN** the "Switch to tab (1–9)" row shows the override indicator

#### Scenario: No override indicator for default prefix
- **WHEN** `switch_to_tab_prefix` is absent from config or equals `"M-"`
- **THEN** the "Switch to tab (1–9)" row shows no override indicator
