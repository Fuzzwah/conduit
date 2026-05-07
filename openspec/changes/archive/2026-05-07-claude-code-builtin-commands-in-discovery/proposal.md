## Why

When using the Claude agent in Conduit, Claude Code's built-in slash commands (e.g., `/compact`, `/context`) don't appear in the autocomplete slash menu, making them effectively invisible. Users must know these commands exist and type them in full — they work when submitted (passed through to Claude Code as a prompt turn) but are undiscoverable via the UI.

## What Changes

- Add a static list of Claude Code built-in commands to the `DiscoveryRegistry` (or a new provider-specific extension), surfaced only when the active agent is Claude
- These entries appear in the slash menu autocomplete alongside Conduit and skill commands
- Selecting a command from the menu inserts its text into the input box (same passthrough behaviour as today)

## Capabilities

### New Capabilities

- `claude-builtin-commands`: A static registry of Claude Code's built-in slash commands, surfaced in the slash menu when the active provider is Claude

### Modified Capabilities

<!-- none — existing passthrough behaviour is unchanged; only discoverability changes -->

## Impact

- `crates/conduit-resolver/` — `DiscoveryRegistry` gains a new source of static Claude-specific commands
- `crates/conduit-ui/` — slash menu already handles provider-filtered entries; no structural changes expected
- No breaking changes; passthrough routing is unchanged
