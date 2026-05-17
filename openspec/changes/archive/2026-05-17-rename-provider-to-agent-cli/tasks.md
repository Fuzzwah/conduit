## 1. Workspace Config Panel (post-creation dialog)

- [x] 1.1 Rename `InlinePickerTarget::Provider` variant and `"Select Provider"` string in `workspace_progress_dialog.rs`
- [x] 1.2 Rename `"Provider"` row label to `"Agent CLI"` in `workspace_progress_dialog.rs`

## 2. Settings Menu

- [x] 2.1 Rename `"Enabled Providers"` title to `"Agent CLIs"` in `app.rs`
- [x] 2.2 Rename `"Providers shown in model selection"` description to `"Agent CLIs available for new sessions"` in `app.rs`

## 3. Provider Selector Dialog

- [x] 3.1 Rename dialog title `"Select Providers"` to `"Select Agent CLIs"` in `provider_selector.rs`
- [x] 3.2 Rename dialog description `"Choose which installed providers to include in model selection."` in `provider_selector.rs`
- [x] 3.3 Rename per-item description template `"{} provider"` to `"{}"` (remove suffix) in `provider_selector.rs`
- [x] 3.4 Rename validation error `"Select at least one provider."` in `provider_selector.rs`
- [x] 3.5 Update file-level doc comment in `provider_selector.rs`

## 4. Web UI — Backend Settings Handler

- [x] 4.1 Rename `"Enabled Providers"` title to `"Agent CLIs"` in `crates/conduit-web/src/handlers/settings.rs`
- [x] 4.2 Rename `"Providers shown in model selection"` description to `"Agent CLIs available for new sessions"` in `crates/conduit-web/src/handlers/settings.rs`

## 5. Web UI — React Settings Dialog

- [x] 5.1 Rename `'Enabled Providers'` sub-editor heading to `'Agent CLIs'` in `crates/conduit-web/web/src/components/SettingsDialog.tsx`

## 6. Confirmation Message & Action Description

- [x] 6.1 Rename `\"Providers updated\"` to `\"Agent CLIs updated\"` in `app_actions_confirm.rs`
- [x] 6.2 Rename `Action::ShowProvidersSelector` description `\"Select providers\"` to `\"Select agent CLIs\"` in `crates/conduit-types/src/action.rs`
- [x] 6.3 Rename `/providers` command description `\"Select enabled providers\"` to `\"Select enabled agent CLIs\"` in `crates/conduit-resolver/src/lib.rs`

## 7. Doc Comments & Type Annotations

- [x] 7.1 Update `InputMode::SelectingProviders` doc comment in `conduit-types/src/input_mode.rs`
- [x] 7.2 Update `reasoning_selector.rs` description string `\"Provider default reasoning behavior\"`

## 8. Final Verification

- [x] 8.1 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
