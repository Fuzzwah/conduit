## Why

When an agent triggers ExitPlanMode, a "build or give feedback" inline prompt appears in the active workspace. A previous fix stopped this prompt from consuming arrow keys while the sidebar has focus, but it still intercepts Enter (and all other keys) when any dialog overlay is displayed — most visibly the Work Complete dialog. Because the inline prompt key handler runs before the Work Complete interceptor in the event dispatch chain, pressing Enter while Work Complete is open drives the inline prompt instead of the dialog.

## What Changes

- The inline prompt key handler gains an additional guard: it is skipped whenever any blocking dialog or overlay is active (`has_active_dialog()` or `work_complete_session.is_some()`), not only when the sidebar has focus.
- The inline prompt will only consume keys when the active workspace is in focus AND no dialog is displayed — consistent with how every other passive UI element behaves.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `inline-prompt-key-routing`: Extend key-routing rules so that all inline prompt input is suppressed (keys fall through) whenever any dialog overlay or work-complete session is active, not just when the sidebar is focused.

## Impact

- `crates/conduit-ui/src/app/app_input.rs` — one-line condition change at the inline prompt guard (lines ~187–188)
- No API, schema, or dependency changes
- Existing sidebar-focus bypass behaviour is preserved; new dialog-bypass behaviour is additive
