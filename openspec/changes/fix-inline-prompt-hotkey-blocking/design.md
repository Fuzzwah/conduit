## Context

`InlinePromptState::handle_key` in `src/ui/components/inline_prompt.rs` is called by `app_input.rs` whenever an inline prompt is active. It returns a `PromptAction` that determines whether the key was consumed or should fall through to global dispatch.

Two match arms — `KeyCode::Left | BackTab | Char('h')` and `KeyCode::Right | Tab | Char('l')` — are intended solely for navigating between question tabs in multi-question `AskUserQuestion` prompts. Their navigation logic is already correctly gated:

```rust
if let InlinePromptType::AskUserQuestion { questions } = &self.prompt_type {
    if questions.len() > 1 || self.has_submit_tab() { ... }
}
```

However, both arms unconditionally return `PromptAction::Consumed` after the guard block, so keys are swallowed even when no navigation occurred (e.g., `ExitPlanMode` prompts, or single-question prompts without a submit tab).

## Goals / Non-Goals

**Goals:**
- Tab/BackTab reach global action dispatch when the inline prompt has no use for them, restoring tab switching and sidebar focus.
- Arrow key question-tab navigation within multi-question prompts is unchanged.

**Non-Goals:**
- Changing how Up/Down navigate between options within a prompt (intentional capture).
- Any refactor of the broader key-routing or InputMode system.

## Decisions

**Minimal in-place fix over InputMode refactor**

The two faulty arms already have the correct condition — they just need to return `NotHandled` instead of `Consumed` when the condition is false. Adding an early `return PromptAction::Consumed` inside the navigation block and changing the trailing return to `PromptAction::NotHandled` is a two-line change per arm.

Alternative considered: adding an `InputMode::PresentingInlinePrompt` variant and blocking global dispatch at the routing level. Rejected — overkill for a one-file, two-arm fix, and would require threading InputMode changes through several files.

## Risks / Trade-offs

- `Char('h')` / `Char('l')` will also pass through as `NotHandled` when no question navigation applies. If those characters are bound to a global action while a prompt is showing, that action will now fire. Currently `h`/`l` have no global binding, so this is safe. [Risk: future bindings] → If `h`/`l` are ever bound globally, the inline prompt should explicitly consume them when active to avoid accidental actions.
