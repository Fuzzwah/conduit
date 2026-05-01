## Context

Tabs in the conduit TUI are stored as a `Vec<Tab>` in `TabManager`. The tab bar renders each tab with a 1-indexed number derived from its position in the Vec. Persistence is handled by `snapshot_session_state`, which writes each tab's Vec index as `tab_index` to the SQLite `session_tabs` table; loading restores them in that order.

The existing action pipeline (`Action` enum → `app_actions_tabs.rs` handler → `TabManager` method) is the established pattern for all tab operations.

## Goals / Non-Goals

**Goals:**
- Move the active tab left or right in the Vec with a single keypress.
- Numbers in the tab bar update immediately (automatic, since they're positional).
- Order persists across sessions via the existing snapshot mechanism.

**Non-Goals:**
- Mouse drag-to-reorder.
- Moving a tab to an arbitrary position (only adjacent swaps).
- Any change to how tabs are rendered or how persistence works.

## Decisions

**Use `Vec::swap` to reorder.**
Swapping adjacent elements is O(1) and keeps the `active_tab` index correct with a simple ±1 adjustment. Alternatives (remove + insert, rotate) are more complex and offer no benefit for adjacent moves.

**Bind to `Alt+Shift+Left` / `Alt+Shift+Right`.**
These are unassigned globally and consistent with the existing `Alt+Shift+Tab` / `Alt+Shift+W` tab-management cluster. Direct `KeyCombo::new(KeyCode::Left/Right, ALT | SHIFT)` inserts are used (same as the BackTab bindings) rather than the string-notation `bind()` helper, because arrow key + modifier combos are already handled this way in the codebase.

**Trigger `SaveSessionState` on move.**
The existing `Effect::SaveSessionState` path calls `snapshot_session_state`, which re-enumerates the Vec and writes updated `tab_index` values. No changes to persistence logic are needed.

## Risks / Trade-offs

- **Terminal sends wrong modifiers for Alt+Shift+Arrow**: Some terminals map this combo differently. → Low risk; users can remap via the keybinding config if needed. No mitigation needed at this stage.
- **Single-tab edge case**: `move_tab_left`/`move_tab_right` are no-ops when the tab is already at the boundary — correct behavior, no special handling required.
