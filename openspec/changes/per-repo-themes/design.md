## Context

The theme system is centralized: a global `OnceLock<RwLock<Theme>>` stores the active theme, and all rendering code reads from it via accessor functions (`text_primary()`, `bg_base()`, etc.). Themes are switched live via `set_theme()`. The global theme is persisted in `~/.conduit/config.toml` under `[theme]`.

Repositories are stored in SQLite (`~/.conduit/conduit.db`). Each workspace tab is backed by an `AgentSession` which knows its `workspace_id`. The tab switch path in `app_actions_tabs.rs` already calls several `sync_*` helpers after every switch.

## Goals / Non-Goals

**Goals:**
- Allow a unique theme to be assigned to each repository (project).
- Automatically apply a repository's theme when switching to any of its workspace tabs.
- Restore the global theme when switching to a tab with no repository theme.
- Provide a low-friction UI to assign/remove a project theme from within the existing theme picker.

**Non-Goals:**
- Per-workspace (branch-level) theming.
- Theme inheritance or fallback chains beyond: project → global.
- Changing the global theme loading or startup behavior for users who don't use project themes.

## Decisions

### 1. Store theme in `repositories` table, not `config.toml`

Alternatives: a `[project_themes]` table in config.toml keyed by repo name/path; a separate `project_themes` DB table.

**Decision**: Add `theme_name TEXT` directly to `repositories`. Repository metadata already lives there, it's the natural home, and it avoids a join or a second lookup.

### 2. Cache `project_theme` in `AgentSession`

Alternatives: look up the repository on every tab switch; store in `AppState`.

**Decision**: Cache `project_theme: Option<String>` on `AgentSession` when the session is created/workspace is loaded. Tab switches are frequent and synchronous; a DB round-trip per switch would add latency. The cache is invalidated whenever the user saves a new project theme.

### 3. Scope toggle inside the existing theme picker

Alternatives: separate "project settings" screen; context menu on the workspace in the sidebar.

**Decision**: Add a toggle (`Global` / `This project`) in the theme picker header. This keeps the workflow identical to setting a global theme — users are already in the picker, one keypress changes scope. Only enabled when the active session has a repository context.

### 4. Fall back to global config theme when leaving a project-themed tab

Alternatives: remember "last global state before override"; track whether user manually changed global theme during session.

**Decision**: On tab switch, always resolve: project theme if available, else `config.theme_name`, else built-in default. Simple and predictable — no hidden state stack.

## Risks / Trade-offs

- **DB migration on existing installs** → `ALTER TABLE repositories ADD COLUMN theme_name TEXT` is a safe additive migration; existing rows get `NULL` (= no project theme).
- **Scope toggle discoverability** → Users may not notice the toggle. Mitigation: show a `(project)` badge in the footer or theme picker title when a project theme is active, prompting curiosity.
- **Theme flicker on rapid tab switching** → The `set_theme()` path uses `RwLock` and is already designed for live switching; no special mitigation needed.
- **Session cache staleness** → If a project theme is changed from a different tab (future multi-window scenario), the cached value would be stale. Acceptable for a single-window TUI; document if multi-window support is ever added.

## Migration Plan

1. DB migration runs automatically on first launch after update (existing migration framework).
2. No config migration needed — `config.toml` schema unchanged.
3. Existing installs: all repositories have `theme_name = NULL`, behavior unchanged.
4. Rollback: remove migration step and revert code changes; old binary ignores unknown columns.

## Open Questions

- Should clearing a project theme (setting it back to "none") be done via the theme picker (e.g., a "Clear project theme" action), or implicitly by switching scope back to Global and confirming the current theme? Recommend an explicit "Clear" action for clarity.
