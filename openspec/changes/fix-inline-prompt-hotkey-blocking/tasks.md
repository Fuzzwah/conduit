## 1. Fix key routing in inline_prompt.rs

- [x] 1.1 In the `KeyCode::Left | KeyCode::Char('h') | KeyCode::BackTab` arm of `InlinePromptState::handle_key` (src/ui/components/inline_prompt.rs ~line 283), add `return PromptAction::Consumed;` inside the navigation block and change the trailing `PromptAction::Consumed` to `PromptAction::NotHandled`
- [x] 1.2 In the `KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab` arm (~line 296), apply the same change: early `return PromptAction::Consumed` inside the navigation block, trailing `PromptAction::NotHandled` otherwise

## 2. Verify

- [x] 2.1 Run `cargo fmt --check && cargo clippy -- -D warnings && cargo test` and confirm all pass
