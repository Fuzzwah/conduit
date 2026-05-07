## Context

The inline prompt (rendered for `ExitPlanMode` and `AskUserQuestion` tool calls) is handled at the top of `handle_key_event` in `app/app_input.rs` (around line 187). The Work Complete dialog interceptor sits lower in the same function (around line 601). Because Rust executes sequentially, keys reach the inline prompt block first.

The existing guard is:
```rust
let sidebar_has_focus = self.state.input_mode == InputMode::SidebarNavigation;
if !sidebar_has_focus {
    // inline prompt consumes the key
}
```

This only carves out the sidebar navigation case. All other overlays — Work Complete, confirmation dialogs, model selector, settings menu, slash menu, etc. — are invisible to the guard, so the inline prompt still swallows their Enter keys.

## Goals / Non-Goals

**Goals:**
- Inline prompt releases key focus whenever any dialog or overlay is active
- Work Complete (and every other overlay) can receive Enter without the inline prompt intercepting it
- Existing sidebar-focus bypass is preserved unchanged

**Non-Goals:**
- Restructuring the key dispatch ordering (the fix is a condition change, not an architectural refactor)
- Changing how the inline prompt renders or what it displays

## Decisions

### Extend the guard with `has_blocking_dialog`

```rust
let sidebar_has_focus = self.state.input_mode == InputMode::SidebarNavigation;
let has_blocking_dialog =
    self.has_active_dialog() || self.state.work_complete_session.is_some();
if !sidebar_has_focus && !has_blocking_dialog {
    // inline prompt consumes the key
}
```

`has_active_dialog()` delegates to `state.has_active_overlay()`, which already covers ~20 overlay types. `work_complete_session.is_some()` is required separately because the Work Complete session is not tracked in `has_active_overlay()`.

**Alternatives considered:**
- *Move the inline prompt block below the Work Complete block*: Would fix the immediate bug but not the general case (other overlays would still steal Enter from inline prompt). The guard approach fixes all overlays at once.
- *Add `work_complete_session` to `has_active_overlay()`*: Would work, but `has_active_overlay()` lives in `app_state.rs` and depends only on state fields — pulling in `work_complete_session` is consistent. However, the current call site already reads the field directly, so an inline `||` is simpler and less coupled.

## Risks / Trade-offs

- **Inline prompt becomes temporarily unresponsive when any overlay is open** → This is the desired behaviour; the user interacts with the overlay first, then returns to the prompt. The prompt remains rendered and will respond once the overlay is dismissed.
- **New overlay types added in future may also block the inline prompt** → Correct by default: any overlay registered through `has_active_overlay()` will automatically suppress inline prompt input.
