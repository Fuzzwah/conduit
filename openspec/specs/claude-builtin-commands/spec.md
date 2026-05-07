# claude-builtin-commands Specification

## Purpose
TBD - created by archiving change claude-code-builtin-commands-in-discovery. Update Purpose after archive.
## Requirements
### Requirement: Claude built-in commands appear in slash menu
When Claude is the active agent, the slash menu SHALL include a static list of Claude Code's built-in slash commands alongside Conduit commands and discovered skills/commands.

#### Scenario: Built-in commands visible in menu when Claude is active
- **WHEN** the active provider is Claude AND the user opens the slash menu (types `/`)
- **THEN** the menu SHALL display entries for Claude built-in commands including `/compact`, `/context`, `/cost`, `/clear`, `/doctor`, `/help`, `/init`, `/memory`, and `/review`

#### Scenario: Built-in commands hidden when Claude is not active
- **WHEN** the active provider is not Claude AND the user opens the slash menu
- **THEN** Claude built-in command entries SHALL NOT appear in the menu

#### Scenario: Built-in commands filterable by prefix
- **WHEN** the user types `/comp` in the input
- **THEN** the slash menu SHALL show `/compact` as a matching entry (and any other commands matching the prefix)

### Requirement: Selecting a Claude built-in command passes it through unchanged
When a user selects a Claude built-in command from the slash menu, the command SHALL be submitted to Claude Code exactly as typed (passthrough behaviour).

#### Scenario: Selecting /compact inserts command text
- **WHEN** the user selects `/compact` from the slash menu
- **THEN** the text `/compact` SHALL be inserted into the input box

#### Scenario: Submitting a Claude built-in sends it to Claude Code
- **WHEN** Claude is active AND the user submits `/compact`
- **THEN** the text `/compact` SHALL be sent to the Claude Code process as the prompt for a new turn

### Requirement: Built-in commands do not conflict with user-defined commands
If a user has a `.claude/commands/<name>.md` file whose name matches a Claude built-in command, both entries MAY appear in the menu, or the user-defined command SHALL take precedence.

#### Scenario: User-defined command shadows same-named builtin
- **WHEN** a `.claude/commands/compact.md` file exists AND the user types `/compact`
- **THEN** the resolved invocation SHALL prefer the user-defined command file over the static builtin

### Requirement: Built-in commands display descriptive metadata in the menu
Each built-in command entry in the slash menu SHALL display a name, a short description, and a source badge identifying it as a Claude built-in.

#### Scenario: Menu entry shows name and description
- **WHEN** the slash menu is open and Claude built-in commands are listed
- **THEN** each entry SHALL show the command label (e.g., `/compact`) and a human-readable description (e.g., "Compact conversation context")

#### Scenario: Menu entry shows Claude source badge
- **WHEN** a Claude built-in command entry is visible in the slash menu
- **THEN** the source badge SHALL read "Claude command" (consistent with other Claude-sourced entries)

