## Why

The term "Provider" in Conduit's UI is misleading. The items listed — Claude Code, Codex CLI, Gemini CLI, OpenCode, Pi, Dirac, GitHub Copilot — are not API providers (e.g., Anthropic, OpenAI, Google). Conduit invokes each one as a subprocess CLI. What unifies them is that they are all **CLI tools that act as coding agents**. The internal enum is already called `AgentType`, but the UI uses the inconsistent term "Provider".

## What Changes

Rename all user-facing occurrences of "Provider" to "Agent CLI" in:
- Workspace configuration dialog (post-creation config panel): label and picker title
- Settings menu (TUI): entry title and description
- Settings dialog (Web UI): sub-editor heading and setting item title/description
- Provider selector multi-select dialog: title, description, per-item description, validation error
- Confirmation toast message
- Slash command description
- Action description for `ShowProvidersSelector` in command palette/keybindings
- Reasoning selector auto-option description

No functional or behavioral changes — pure terminology rename.

## Capabilities

### New Capabilities
*(none — pure rename, no new functionality)*

### Modified Capabilities
*(none — no spec-level behavior changes)*

## Impact

- **Files touched**: ~10-12 files across `crates/conduit-ui/`, `crates/conduit-web/`, `crates/conduit-resolver/`, `crates/conduit-types/`, and `crates/conduit-web/web/src/` covering UI strings, doc comments, the `InputMode` variant doc, and the Web UI React settings dialog
- **No API changes**, no database schema changes, no config format changes
- No breaking changes — config keys (e.g., `providers` in TOML) remain unchanged
