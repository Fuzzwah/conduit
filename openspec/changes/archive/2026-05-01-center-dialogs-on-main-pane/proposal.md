## Why

When the sidebar is visible, dialog windows (help, model selector, confirmation, error, etc.) center themselves relative to the full terminal width, causing them to appear partially or fully over the sidebar. Dialogs should center within the main pane — the content area to the right of the sidebar.

## What Changes

- All dialog render calls in `draw()` will receive `right_area` (the main pane rect) instead of `size` (the full terminal rect) as their area argument.
- When no sidebar is visible, `right_area == size`, so behavior is unchanged.

## Capabilities

### New Capabilities

- `sidebar-aware-dialog-centering`: Dialogs center within the main pane (excluding sidebar) rather than the full terminal area.

### Modified Capabilities

<!-- No existing spec-level requirements are changing — this is a pure UX fix with no API or behavior contract changes. -->

## Impact

- `src/ui/app.rs` — all dialog render call sites in `draw()` (both the empty-state branch and the main render block at the bottom)
- No new types, traits, or public APIs introduced
- No breaking changes
