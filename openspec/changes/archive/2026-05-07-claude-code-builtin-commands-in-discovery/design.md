## Context

`DiscoveryRegistry` discovers slash commands by scanning the filesystem for `.claude/commands/`, `.claude/skills/`, etc. Claude Code's own built-in commands (e.g., `/compact`, `/context`, `/cost`) are not files — they are hardcoded inside the Claude Code CLI — so they are never found by the scanner and never appear in the slash menu autocomplete.

When a user types `/compact` in full and submits, it works: the text falls through as a `Passthrough` and Claude Code handles it. The gap is purely discoverability — the slash menu is silent about these commands.

Current menu pipeline:
1. `DiscoveryRegistry::discover()` scans filesystem
2. `CommandResolver::menu_entries()` merges Conduit commands + discovered entries
3. Slash menu renders entries, filtered by active provider via `provider_matches_source()`

## Goals / Non-Goals

**Goals:**
- Claude Code built-in commands appear in the Conduit slash menu when Claude is the active agent
- Selecting one inserts `/command` text into the input (same passthrough behaviour as today)
- No filesystem scanning required — the list is static

**Non-Goals:**
- Implementing any new behaviour for these commands (they remain passthroughs)
- Syncing the list dynamically from Claude Code at runtime
- Surfacing these commands for non-Claude providers

## Decisions

### Add a static injection method to `DiscoveryRegistry`

Add `inject_claude_builtins()` called at the end of `DiscoveryRegistry::discover()`. It inserts `ProviderInvocation::PromptCommand` entries with `source: Claude` and a sentinel path (`PathBuf::from("<builtin>/<name>")`).

**Why not a new enum variant?**  
`PromptCommand` already carries source, name, description, and content. The existing `render_invocation` logic already returns `original_text` when `provider_matches_source(Claude, Claude)` — i.e., the passthrough behaviour we want. Adding a new variant would require updating every `match` on `ProviderInvocation` across the codebase for no behavioural gain.

**Why a sentinel path?**  
The deduplication set in `discover()` keys on `(ProviderArtifactSource, PathBuf)`. Builtins need a stable, unique path that will never collide with real files. `"<builtin>/<name>"` is unambiguous and survives across multiple `discover()` calls.

**Content field:**  
Set `content` to the command token (e.g., `"/compact"`). When Claude is active, `render_invocation` ignores content and returns `original_text` directly. If somehow a non-Claude provider resolves a Claude builtin, `render_prompt_command("/compact", args)` renders to `/compact` — harmless.

**Commands to include** (excludes ones Conduit already intercepts: `/model`, `/status`):

| Command | Description |
|---|---|
| `/compact` | Compact conversation context |
| `/context` | Show context window usage |
| `/cost` | Show session cost and token usage |
| `/clear` | Clear conversation history |
| `/doctor` | Check Claude Code installation health |
| `/help` | Show Claude Code help |
| `/init` | Initialize CLAUDE.md for this project |
| `/memory` | Edit CLAUDE.md memory files |
| `/review` | Review pending code changes |

### Menu filtering is already correct

`DiscoveryRegistry::menu_entries()` already calls `provider_matches_source(active_provider, entry.source())`, which returns `true` only when active is Claude and source is Claude. No changes needed there.

### `resolve()` path

When `/compact` is typed in full, `CommandResolver::resolve()` already hits the `DiscoveryRegistry` lookup. With builtins injected, it'll now return `ProviderPrompt` (with `agent_text = "/compact"`) instead of `Passthrough`. The UI behaviour is identical — the text sent to Claude Code is the same.

## Risks / Trade-offs

- **Static list drift** → If Anthropic adds or removes Claude Code built-in commands, the list needs manual updating. Mitigation: document the list in a comment pointing to Claude Code's changelog; the cost of staleness is low (at worst, a non-functional menu entry).
- **Command conflicts** → A user could create a `.claude/commands/compact.md` file, which would also appear as a Claude command. The `seen` deduplication in `discover()` uses paths, so the real file and the builtin would both appear (different paths). Mitigation: inject builtins *before* scanning, and skip builtins whose name is already in the registry.
- **Sentinel path** → `<builtin>/compact` is not a valid filesystem path but is used only as a deduplication key, never read or displayed. Low risk.
