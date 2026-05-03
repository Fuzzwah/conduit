## MODIFIED Requirements

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

## ADDED Requirements

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
