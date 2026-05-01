## Requirements

### Requirement: Move active tab left
The TUI SHALL move the active tab one position left (toward index 0) when the user presses Alt+Shift+Left. If the active tab is already the first tab, the keypress SHALL be a no-op.

#### Scenario: Move tab left from middle position
- **WHEN** the user has 3 or more tabs open and the active tab is not the first tab
- **WHEN** the user presses Alt+Shift+Left
- **THEN** the active tab moves one position left in the tab bar
- **THEN** the tab numbers update to reflect the new order
- **THEN** the active tab remains focused

#### Scenario: Move tab left when already first
- **WHEN** the active tab is the first tab (position 1)
- **WHEN** the user presses Alt+Shift+Left
- **THEN** the tab order does not change

### Requirement: Move active tab right
The TUI SHALL move the active tab one position right (toward the last position) when the user presses Alt+Shift+Right. If the active tab is already the last tab, the keypress SHALL be a no-op.

#### Scenario: Move tab right from middle position
- **WHEN** the user has 2 or more tabs open and the active tab is not the last tab
- **WHEN** the user presses Alt+Shift+Right
- **THEN** the active tab moves one position right in the tab bar
- **THEN** the tab numbers update to reflect the new order
- **THEN** the active tab remains focused

#### Scenario: Move tab right when already last
- **WHEN** the active tab is the last tab
- **WHEN** the user presses Alt+Shift+Right
- **THEN** the tab order does not change

### Requirement: Reordered tab order persists across sessions
The TUI SHALL persist the reordered tab sequence to the session database so that the order is restored on the next launch.

#### Scenario: Order restored after restart
- **WHEN** the user reorders tabs and closes the application
- **THEN** on the next launch, the tabs appear in the reordered sequence

### Requirement: Tab numbers reflect current order
The tab bar SHALL display 1-indexed numbers that always reflect the current position of each tab in the ordered sequence.

#### Scenario: Numbers update after reorder
- **WHEN** a tab is moved left or right
- **THEN** the number label on each tab immediately reflects its new position
