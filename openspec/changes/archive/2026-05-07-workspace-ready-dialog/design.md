## Context

After a workspace is created the user lands immediately in a session with whatever provider/model/orchestration settings are currently active globally. PR #231 added per-workspace and per-project `orchestration_enabled` overrides (stored in SQLite) and a sidebar `M-o` toggle, but there is still no natural moment to configure a new workspace before the agent starts.

The workspace progress dialog already has a "complete" state (used by the error path and previously by the "Continue" button). This design extends that state into a configuration panel rather than replacing the dialog.

The existing selector modals — `ProviderSelectorState`, `ModelSelectorState`, `OrchestrationSelectorState` — are full-featured search/toggle dialogs. The config panel is intentionally lightweight: it displays current selections inline, and activating a Provider or Model row opens the relevant existing modal. Mode and Orchestration are binary toggles that work entirely inline with arrow keys or Space.

## Goals / Non-Goals

**Goals:**
- Always show the config panel after successful workspace creation (no auto-dismiss).
- Four configurable rows: Provider, Model, Mode, Orchestration.
- Orchestration row greyed/non-interactive when the selected provider is not Claude.
- Mode row greyed/non-interactive when the selected provider does not support plan mode (via `AgentCapabilities`).
- "Set as project default" checkbox that persists provider, model, and orchestration to the repository record.
- Resolution chain for initial selections: workspace defaults → project defaults → global config.
- Error path unchanged — red border, dismiss-only, no config rows.

**Non-Goals:**
- Per-workspace (as opposed to per-project) persistence of provider/model defaults — saving as "project default" is sufficient.
- Changing workspace defaults mid-session from within the dialog.
- Storing any config in the workspace row itself (only the repository row gains new columns).

## Decisions

### D1 — Embed in `WorkspaceProgressDialogState`, not a new dialog

The config panel reuses the existing dialog widget and state machine. `WorkspaceProgressDialogState` gains a nested `WorkspaceReadyConfigState` (populated on `finish()` success) that holds focused row index, pending selections, and the save-as-default flag. This keeps `InputMode::CreatingWorkspace` as the sole mode gate and avoids a second dialog transition.

*Alternative considered:* A separate `WorkspaceReadyDialogState` and `InputMode::WorkspaceReady`. Rejected because it duplicates the dialog frame and transition logic for no user-visible benefit.

### D2 — Provider and Model rows open existing modals; Mode and Orchestration toggle inline

Provider has 7 options requiring availability checks; Model has 10+ options and a search field. Embedding these inline as text fields would duplicate the existing selector logic. Instead, pressing Enter on those rows transitions to `InputMode::SelectingProviders` / `InputMode::SelectingModel` with a `ModelPickerContext::WorkspaceReadyConfig` context to return focus to the config panel on close.

Mode (Build/Plan) and Orchestration (On/Off) are binary — Left/Right or Space toggles them in place. No modal needed.

*Alternative considered:* Inline dropdown/cycle for Provider and Model. Rejected — model list is too long and already has a fuzzy search; duplicating that here would be significant work with no new value.

### D3 — Pending session config stored in `AppState`, applied on dialog close

`close_workspace_progress_dialog()` already reads `pending_created_workspace_id` from `AppState`. We add a parallel `pending_workspace_session_config: Option<PendingSessionConfig>` (`{ provider, model_id, mode, orchestration_enabled }`) to `AppState`. On dialog dismiss, `close_workspace_progress_dialog()` applies those settings to the newly opened tab's session before returning. This avoids changing `open_workspace_with_options()`'s signature.

### D4 — Repository gains `default_provider` and `default_model` columns (migration 22)

`orchestration_enabled` already exists on `Repository` (PR #231). "Set as project default" saves all three fields. New columns: `default_provider TEXT` (nullable) and `default_model TEXT` (nullable), read as `Option<String>` on the `Repository` struct.

Resolution at dialog init:
1. Load workspace record → check `orchestration_enabled` (no provider/model on workspace).
2. Load repository record → check `default_provider`, `default_model`, `orchestration_enabled`.
3. Fall back to `config().preferred_provider_for_new_sessions()` / `config().default_model_for(provider)` / `config().orchestration.enabled_by_default`.

### D5 — Dialog height expands to accommodate config rows

Current heights: 15 lines (running), 17 lines (complete with button). The config panel adds approximately 10 lines (1 separator + 4 rows + 1 save-default row + 1 gap + 1 continue button row + gaps). Target complete-with-config height: ~27 lines. The `dialog_height()` method is updated accordingly; `DIALOG_WIDTH` (68) is unchanged.

## Risks / Trade-offs

[Dialog height may clip on very small terminals] → Mitigation: cap config panel rendering at available height, skip rows that don't fit rather than crashing.

[Provider modal returning to config panel requires new `ModelPickerContext` variant] → Mitigation: add `WorkspaceReadyConfig` variant alongside the existing `SessionSelection` variant; the close path checks context and returns focus to `InputMode::CreatingWorkspace` instead of `InputMode::Normal`.

[`default_provider` stored as a string may diverge from `AgentType` enum values] → Mitigation: use `AgentType`'s existing `to_string()` / `from_str()` as the canonical serialisation (same pattern used elsewhere in the config layer).

[Adding provider/model to repository is a new concept — global config already handles defaults] → Accepted trade-off. Per-project provider/model overrides are the natural complement to the per-project orchestration override from PR #231 and follow the same `Option<T>` / nullable column pattern already established.

## Migration Plan

Migration 22 (idempotent `ALTER TABLE` guards, same pattern as migration 21 in PR #231):
```sql
ALTER TABLE repositories ADD COLUMN default_provider TEXT;
ALTER TABLE repositories ADD COLUMN default_model TEXT;
```

No data backfill needed — `NULL` means "inherit global config" for both columns.

Rollback: columns are nullable and additive; older binary versions simply ignore the new columns.

## Open Questions

- Should the "Set as project default" checkbox default to **checked** or **unchecked**? Leaning toward unchecked (opt-in) to avoid silently overwriting existing project defaults when a user just wants a one-off config change.
- Should the dialog show the workspace name in the title (`Creating Workspace` → `free-bay ready`) on success? Nice-to-have, not blocking.
