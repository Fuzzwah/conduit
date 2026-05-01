## Why

When working across multiple projects in conduit, all workspace tabs look identical — there's no visual cue to distinguish which project you're in. Assigning a unique theme per repository lets users instantly recognise project context at a glance as they switch tabs.

## What Changes

- Repositories gain an optional `theme_name` field stored in the SQLite database.
- The TUI applies a repository's theme when switching to any of its workspace tabs, and restores the global theme when switching to a tab with no project theme.
- The theme picker gains a scope toggle so users can save a theme to "This project" instead of globally.
- A "(project)" indicator appears in the theme picker when a project-level override is active.

## Capabilities

### New Capabilities

- `per-repo-theme`: Store an optional theme name per repository and automatically apply it when the active workspace tab changes.

### Modified Capabilities

<!-- No existing spec-level requirements are changing -->

## Impact

- `src/data/models.rs` — `Repository` struct gains `theme_name: Option<String>`
- `src/data/database.rs` — DB migration adds `theme_name` column to `repositories`; new `update_repository_theme()` helper
- `src/ui/session.rs` — `AgentSession` caches `project_theme: Option<String>` to avoid per-switch DB lookups
- `src/ui/app.rs` — new `sync_theme_to_active_tab()` called on every tab switch and on startup
- `src/ui/app/app_actions_tabs.rs` — hooks `sync_theme_to_active_tab()` into tab switch flow
- `src/ui/components/theme_picker.rs` — scope toggle (Global / This project) and confirmation path for project-scoped saves
