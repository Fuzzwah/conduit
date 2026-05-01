## 1. Update dialog render call sites in `draw()`

- [x] 1.1 In `src/ui/app.rs` `draw()`, replace all `size` arguments passed to dialog render calls in the **empty-state branch** (~lines 11384–11492) with `right_area` — covers `BaseDirDialog`, `ProviderSelector`, `ProjectPicker`, `AddRepoDialog`, `SessionImportPicker`, `ModelSelector` (including `update_viewport`), `ReasoningSelector`, `render_theme_picker`, `AgentSelector`, `ConfirmationDialog`, `ErrorDialog`, `MissingToolDialog`, `HelpDialog`, `SettingsMenu`, `CommandPalette`, `WorkspaceDefaultsDialog`, `RenameProjectDialog`
- [x] 1.2 In `src/ui/app.rs` `draw()`, replace all `size` arguments passed to dialog render calls in the **main render block** at the bottom of `draw()` (~lines 11852–12052) with `right_area` — same dialog set plus `KeybindingsEditor`, `FilePickerDialog`, `ScpCommandDialog`, `IssuePicker`, `SpecPicker`, `SpecifyPicker`, `WorkspaceProgressDialog`, and the two inline `DialogFrame::new(...).render(size, ...)` calls for CloningRepository and RemovingProject spinners

## 2. Verify and validate

- [x] 2.1 Run `cargo fmt --check && cargo clippy -- -D warnings && cargo test` and confirm all pass
- [x] 2.2 Manually test: open conduit with sidebar visible, open each dialog type and confirm centering is within the main pane
