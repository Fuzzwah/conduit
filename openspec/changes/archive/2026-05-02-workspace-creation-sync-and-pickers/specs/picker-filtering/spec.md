## ADDED Requirements

### Requirement: Issue picker has an always-visible text search
The issue picker dialog SHALL include a text-input field at the top of the dialog that is focused by default whenever the picker is interactive (i.e. not in the syncing or fetching phases). Printable character keys typed while the picker is interactive SHALL append to this input. The input SHALL filter the visible issues to those whose `#<number>` or `<title>` (case-insensitive substring) contains the input. The filter SHALL update on each keystroke.

#### Scenario: Typing filters the list live
- **GIVEN** an issue picker showing 50 issues
- **WHEN** the user types "auth"
- **THEN** only issues whose number or title contains "auth" (case-insensitive) remain visible
- **AND** the count footer updates to e.g. "3/50 issues"

#### Scenario: Backspace restores items
- **WHEN** the user backspaces a character from the search input
- **THEN** the filter is recomputed and previously hidden items reappear if they now match

### Requirement: Issue picker supports label filtering
The issue picker SHALL allow filtering by zero or more labels. Pressing `Tab` SHALL open a label-multiselect popover sourced from the union of labels present in the loaded issues (no extra network request). When one or more labels are selected, the visible issues SHALL be those whose `labels` field contains EVERY selected label (AND-composition).

#### Scenario: Selecting labels narrows the list
- **GIVEN** issues with labels `[bug,docs]`, `[bug]`, and `[feature]`
- **WHEN** the user selects label `bug`
- **THEN** the first two issues are visible and the third is not

#### Scenario: Multiple selected labels AND-compose
- **GIVEN** the same issues
- **WHEN** the user selects labels `bug` AND `docs`
- **THEN** only the first issue is visible

#### Scenario: Selected labels render as chips
- **WHEN** one or more labels are selected
- **THEN** the chips are rendered above the list, e.g. `[bug] [docs]`

### Requirement: Issue picker supports "mine only" assignee filter
The issue picker SHALL provide a toggle (bound to `m`) that filters the visible issues to those whose `assignee_logins` contains the current user. The current user SHALL be resolved via the active `IssueProvider::current_user()`. If `current_user()` returns `None`, toggling SHALL have no effect on the list and the footer SHALL display `mine: unavailable`.

#### Scenario: Mine-only filter applies
- **GIVEN** the current user is `octocat` and three issues are loaded with assignees `[octocat]`, `[]`, `[other]`
- **WHEN** the user presses `m`
- **THEN** only the first issue is visible
- **AND** the footer shows `mine: on`

#### Scenario: Mine-only without resolvable user is a no-op
- **GIVEN** `current_user()` returns `None`
- **WHEN** the user presses `m`
- **THEN** the visible list is unchanged
- **AND** the footer shows `mine: unavailable`

### Requirement: Issue picker filters compose with AND
When more than one filter is active (text, labels, mine-only), the visible issues SHALL be those that satisfy ALL active filters.

#### Scenario: Text + label compose
- **GIVEN** the user has typed "auth" and selected label `bug`
- **THEN** only issues whose title/number contains "auth" AND whose labels contain `bug` are visible

### Requirement: Issue picker has progressive Esc semantics
Pressing Esc SHALL clear active filters in the order: text input first (if non-empty), then labels (if any selected), then mine-only (if on). Pressing Esc when no filters are active SHALL dismiss the picker (skipping the issue phase).

#### Scenario: Esc clears search before dismissing
- **GIVEN** the search input contains "auth" and labels are selected
- **WHEN** the user presses Esc once
- **THEN** the search input is cleared but the labels remain selected
- **AND** the picker stays visible

#### Scenario: Esc with no filters dismisses
- **GIVEN** no filters are active
- **WHEN** the user presses Esc
- **THEN** the picker is dismissed and the flow advances to the spec phase

### Requirement: Spec picker has a text search
The OpenSpec and spec-kit pickers SHALL each include a text-input field at the top of the dialog that filters the visible specs by case-insensitive substring of the change identifier.

#### Scenario: Spec search filters live
- **GIVEN** a spec picker with 23 changes
- **WHEN** the user types "auth"
- **THEN** only changes whose `change_id` contains "auth" (case-insensitive) are visible
- **AND** the footer count updates to e.g. "4/23 specs"

### Requirement: Spec picker sort applies to filtered subset
The spec picker's sort cycler (bound to `s`) SHALL operate on the currently filtered subset of specs. After a sort cycle, the selected index SHALL reset to 0 of the filtered subset.

#### Scenario: Sort respects active filter
- **GIVEN** the user has typed "auth" leaving 4 visible specs
- **WHEN** the user presses `s`
- **THEN** only those 4 specs are reordered; specs hidden by the filter are not promoted into view

### Requirement: Picker filtering reuses SearchableListState
The implementation SHALL use the existing `SearchableListState` (`src/ui/components/searchable_list.rs`) to hold the search input, filtered indices, selection, and scroll for both pickers, rather than introducing parallel state types.

#### Scenario: No new selection/scroll types
- **WHEN** the picker source files are reviewed
- **THEN** they hold their list cursor state via `SearchableListState`, not via independent `selected: usize` + `scroll_offset: usize` fields
