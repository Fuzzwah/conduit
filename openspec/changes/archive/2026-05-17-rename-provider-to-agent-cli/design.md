## Context

Currently, Conduit's UI uses "Provider" to label the set of available coding agents (Claude Code, Codex CLI, Gemini CLI, OpenCode, Pi, Dirac, GitHub Copilot). The internal enum is `AgentType`, creating an inconsistency between the code model and the user-facing terminology. This is a pure UI label change — no config keys, database fields, or internal identifiers change.

## Goals / Non-Goals

**Goals:**
- Rename all user-facing "Provider" terminology to "Agent CLI" across the TUI
- Update associated Rust identifiers (enums, variants, doc comments) to match
- Keep all config-file and internal identifiers unchanged (no breaking changes)

**Non-Goals:**
- No config key changes (`providers` in TOML stays as-is)
- No database schema changes
- No internal API renaming beyond what is visible to users
- No behavioral changes

## Decisions

**Decision 1: "Agent CLI" over alternatives**
- "Agent" alone was considered but is ambiguous without context
- "Agent CLI" precisely communicates what these items are — CLI tools that function as coding agents
- This matches how several of them already name themselves (e.g., "Codex CLI", "Gemini CLI", "Dirac CLI")
- No internal identifiers are renamed; this is solely the user-facing label

**Decision 2: Scope of rename**
- Only user-visible strings and their immediate supporting identifiers (enum variants, doc comments, variable names referencing the label) are changed
- TOML config key `providers` in `[settings]` and its `enabled` sub-key remain unchanged — this would be a breaking change
- The `Action::ShowProvidersSelector` variant name stays — it's internal plumbing
- The `InputMode::SelectingProviders` variant stays — internal plumbing
- Web UI API types (`ProviderInfo`, `ProvidersResponse`, `SetProvidersRequest`, `get_providers`, `set_providers`, `useProviders`, `useSetProviders`, `ProvidersSubEditor`) stay as-is — they are internal code identifiers, not user-facing strings
- Web UI React heading string `"Enabled Providers"` and Rust backend setting item title/description ARE renamed — they are user-facing

**Decision 3: All affected files changed in a single commit**
- The change is mechanical and localized to a handful of strings across 6-8 files
- A single commit keeps blame clean and the change atomic

## Risks / Trade-offs

- **Risk: Stale references in documentation or user guides** → Mitigation: update user-facing docs in `docs/` alongside code
- **Risk: Users searching settings for "providers" won't find it** → Mitigation: acceptable; the rename is more accurate and users adapt quickly; filter/search handles partial matches on descriptions and values
- **Trade-off: "Agent CLI" is slightly longer than "Provider"** → Acceptable; readability and accuracy outweigh the minor width increase
