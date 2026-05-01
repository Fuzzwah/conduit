## Why

Users can switch between tabs but cannot change their order, so related tabs (e.g. a PR tab next to its file viewer) cannot be grouped together. This adds drag-free keyboard reordering to match the level of control users have in most tabbed environments.

## What Changes

- New keybinding `Alt+Shift+Left` moves the active tab one position left in the tab bar.
- New keybinding `Alt+Shift+Right` moves the active tab one position right in the tab bar.
- Tab numbers (`[1]`, `[2]`, …) update immediately to reflect the new order.
- The new order is persisted to the session database and restored on next launch.

## Capabilities

### New Capabilities

- `tab-reorder`: Move the active TUI tab left or right via keyboard, with order persisted across sessions.

### Modified Capabilities

<!-- None — no existing spec-level behavior changes. -->

## Impact

- `src/ui/action.rs` — two new `Action` variants
- `src/ui/tab_manager.rs` — two new methods on `TabManager`
- `src/config/default_keys.rs` — two new global keybindings
- `src/ui/app/app_actions_tabs.rs` — handlers for the new actions
- Session persistence is unchanged; the existing `snapshot_session_state` / `tab_index` mechanism already captures order correctly.
