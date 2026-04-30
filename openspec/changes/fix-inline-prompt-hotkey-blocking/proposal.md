## Why

When Claude presents an inline prompt (ExitPlanMode or AskUserQuestion), pressing Tab, BackTab, Left, or Right silently swallows the keypress — the user cannot switch tabs or move focus to the sidebar while any inline prompt is visible. This is a regression in basic TUI navigation that surfaces every time Claude uses plan mode.

## What Changes

- The `Left | BackTab | Char('h')` and `Right | Tab | Char('l')` match arms in `InlinePromptState::handle_key` will return `PromptAction::NotHandled` when their question-navigation guard does not apply, instead of unconditionally returning `PromptAction::Consumed`.
- Global hotkeys (tab switching, sidebar toggle) will now work while an inline prompt is displayed.
- Navigation within multi-question AskUserQuestion prompts is unchanged.

## Capabilities

### New Capabilities
<!-- none — this is a bug fix with no new user-facing capability -->

### Modified Capabilities
<!-- None — no spec-level requirement change; this restores intended key-routing behaviour -->

## Impact

- `src/ui/components/inline_prompt.rs` — two match arms in `handle_key()`
- No API, database, or dependency changes
- No breaking changes
